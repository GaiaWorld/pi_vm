use std::sync::Arc;
use std::io::{Error, ErrorKind};
use std::sync::atomic::{AtomicU8, Ordering};

use parking_lot::RwLock;

use atom::Atom;
use worker::{impls::cast_js_task, task::TaskType};
use handler::{Args, GenType};
use hash::XHashMap;

use adapter::{pause, JS};
use pi_vm_impl::push_msg;
use bonmgr::NativeObjsAuth;
use proc::{ProcStatus, ProcInfo, Process, ProcessFactory};
use proc_pool::register_process;

/*
* 默认的异步虚拟机任务优先级
*/
const DEFAULT_ASYNC_VM_TASK_PRIORITY: usize = 1000;

/*
* 基于Duktape运行时的进程
*/
pub struct DukProcess {
    pid:        usize,          //进程唯一id
    name:       Option<Atom>,   //进程名称
    status:     Arc<AtomicU8>,  //进程运行状态
    init_call:  Option<Atom>,   //记录调用入口
    priority:   usize,          //异步虚拟机任务优先级
    vm:         Arc<JS>,        //虚拟机
    receiver:   u32,            //虚拟机异步接收消息的回调入口
}

impl Process<(Arc<NativeObjsAuth>, Arc<Vec<Vec<u8>>>), Box<FnOnce(Arc<JS>) -> usize>, (Arc<Vec<u8>>, Vec<usize>)> for DukProcess {
    type Process = Self;
    type Output = ();
    type Error = Error;

    fn init(mut pid: u64, name: Option<String>, (auth, codes): (Arc<NativeObjsAuth>, Arc<Vec<Vec<u8>>>)) -> Result<Self::Process, Self::Error> {
        let pid = pid as usize;
        let (name, vm_name) = if let Some(str) = name {
            let atom = Atom::from(str);
            (Some(atom.clone()), atom)
        } else {
            (None, Atom::from(""))
        };

        if let Some(vm) = JS::new(pid, vm_name, auth, None) {
            //加载初始化字节码
            for code in codes.as_slice() {
                if vm.load(code.as_slice()) {
                    while !vm.is_ran() {
                        pause();
                    }
                    continue;
                }
                return Err(Error::new(ErrorKind::InvalidData, format!("init duktape vm failed, pid: {:?}, name: {:?}, reason: load code failed", pid, name)));
            }

            //初始化进程环境
            let val = vm.new_u32(pid as u32);
            vm.set_global_var("_$pid".to_string(), val);
            if let Some(n) = &name {
                if let Ok(val) = vm.new_str((&n).to_string()) {
                    vm.set_global_var("_$pname".to_string(), val);
                }
            }

            return Ok(DukProcess {
                pid,
                name,
                status: Arc::new(AtomicU8::new(ProcStatus::Init.into())),
                init_call: None,
                priority: DEFAULT_ASYNC_VM_TASK_PRIORITY,
                vm,
                receiver: 0,
            });
        }

        Err(Error::new(ErrorKind::InvalidInput, format!("init duktape vm failed, pid: {:?}, name: {:?}", pid, name)))
    }

    fn pid(&self) -> u64 {
        self.pid as u64
    }

    fn name(&self) -> Option<&str> {
        if let Some(name) = &self.name {
            Some(name.as_str())
        } else {
            None
        }
    }

    fn status(&self) -> ProcStatus {
        self.status.load(Ordering::SeqCst).into()
    }

    fn call(&mut self, module: String, function: String, args: Box<FnOnce(Arc<JS>) -> usize>) -> Result<Self::Output, Self::Error> {
        let init_status = ProcStatus::Init.into();
        let running_status = ProcStatus::Running.into();
        match self.status.compare_and_swap(init_status, running_status, Ordering::SeqCst) {
            init_status => {
                //当前进程可以启动
                let init_call = Atom::from(module + "." + function.as_str());
                self.init_call = Some(init_call.clone());
                self.call_init(init_call, args);
                Ok(())
            },
            status => {
                //无效的进程状态
                Err(Error::new(ErrorKind::Other, format!("invalid duk process status, pid: {:?}, name: {:?}, status: {:?}", self.pid, self.name, status)))
            }
        }
    }

    fn info(&self, info: ProcInfo<(Arc<Vec<u8>>, Vec<usize>)>) -> Result<(), Self::Error> {
        let running_status: u8 = ProcStatus::Running.into();
        match self.status.load(Ordering::SeqCst) {
            running_status => {
                //当前进程正在运行
                let args = Box::new(move |vm: Arc<JS>| {
                    //TODO 测试代码...
                    vm.new_u32(info.source() as u32);
                    vm.new_str(unsafe { String::from_utf8_unchecked(info.payload().0.to_vec()) });
                    2
                });
                push_msg(self.vm.clone(), self.receiver, args, Atom::from(format!("DukProcess Info Task, pid: {:?}, name: {:?}", self.pid, self.name)));
                Ok(())
            },
            status => {
                //当前进程未运行
                Err(Error::new(ErrorKind::Other, format!("duk process not running, pid: {:?}, name: {:?}, status: {:?}", self.pid, self.name, status)))
            }
        }
    }
}

impl DukProcess {
    //调用进程虚拟机的初始函数
    pub fn call_init(&self, init_call: Atom, args: Box<FnOnce(Arc<JS>) -> usize>) {
        let vm = self.vm.clone();

        let func = Box::new(move |_lock| {
            vm.get_link_function((&init_call).to_string());
            let args_size = args(vm.clone());
            vm.call(args_size);
        });

        cast_js_task(TaskType::Async(false), self.priority, None, func, Atom::from(format!("DukProcess Task, pid: {:?}, name: {:?}", self.pid, self.name)));
    }

    //设置进程虚拟机，接收异步消息的回调入口
    pub fn set_receiver(&mut self, receiver: u32) {
        self.receiver = receiver;
    }
}

/*
* 基于Duktape运行时的进程工厂
*/
pub struct DukProcessFactory {
    name:       Atom,                                       //进程工厂名称
    auth:       Arc<NativeObjsAuth>,                        //虚拟机对象授权
    codes:      Arc<Vec<Vec<u8>>>,                          //虚拟机初始化字节码
    pool:       Arc<RwLock<XHashMap<usize, DukProcess>>>,   //进程池
}

impl ProcessFactory for DukProcessFactory {
    type Error = Error;

    fn name(&self) -> &str {
        &self.name
    }

    fn new_process(&self, pid: u64, name: Option<String>) -> Result<(), Self::Error> {
        match DukProcess::init(pid, name.clone(), (self.auth.clone(), self.codes.clone())) {
            Err(e) => Err(e),
            Ok(process) => {
                //初始化进程成功
                let status = process.status.clone(); //获取进程的共享状态
                self.pool.write().insert(pid as usize, process); //将进程加入当前工厂的进程池
                register_process(pid, name, status, self.name.clone()); //在全局进程池中注册
                Ok(())
            }
        }
    }

    //启动指定进程
    fn startup(&self,
               pid: u64,
               module: String,
               function: String,
               args: Args<GenType, GenType, GenType, GenType, GenType, GenType, GenType, GenType>) -> Result<(), Self::Error> {
        if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
            let vm_args = Box::new(move |vm: Arc<JS>| {
                //TODO...
                0
            });
            return process.call(module, function, vm_args);
        }

        Err(Error::new(ErrorKind::Other, format!("duk process startup failed, pid: {:?}, module: {:?}, function: {:?}, reason: process not exists", pid, module, function)))
    }

    fn set_receiver(&self, pid: u64, receiver: GenType) -> Result<(), Self::Error> {
        if let GenType::U32(callback) = receiver {
            if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
                process.set_receiver(callback);
                Ok(())
            } else {
                Err(Error::new(ErrorKind::Other, format!("duk process set receiver failed, pid: {:?}, reason: process not exists", pid)))
            }
        } else {
            Err(Error::new(ErrorKind::Other, format!("duk process set receiver failed, pid: {:?}, reason: invalid receiver", pid)))
        }
    }

    //向指定进程发送消息
    fn send(&self, src: u64, dst: u64, mut msg: GenType) -> Result<(), Self::Error> {
        if let Some(process) = self.pool.read().get(&(dst as usize)) {
            if let GenType::Array(array) = &mut msg {
                if let GenType::ArcBin(bin) = array.remove(0) {
                    if let GenType::Array(vec) = array.remove(0) {
                        let mut objs: Vec<usize> = Vec::with_capacity(vec.len());
                        for val in vec {
                            if let GenType::USize(index) = val {
                                objs.push(index);
                            }
                        }

                        return process.info(ProcInfo::new(src, dst, (bin, objs)));
                    } else {
                        //NativeObject错误，则立即返回错误原因
                        return Err(Error::new(ErrorKind::Other, format!("send msg to duk process failed, src: {:?}, dst: {:?}, reason: invalid objs", src, dst)));
                    }
                } else {
                    //序列化参数错误，则立即返回错误原因
                    return Err(Error::new(ErrorKind::Other, format!("send msg to duk process failed, src: {:?}, dst: {:?}, reason: invalid bin", src, dst)));
                }
            } else {
                //无效的消息，则立即返回错误原因
                return Err(Error::new(ErrorKind::Other, format!("send msg to duk process failed, src: {:?}, dst: {:?}, reason: invalid msg", src, dst)));
            }
        }

        Err(Error::new(ErrorKind::Other, format!("send msg to duk process failed, src: {:?}, dst: {:?}, reason: process not exists", src, dst)))
    }
}

impl DukProcessFactory {
    //构建基于Duktape运行时的进程工厂
    pub fn new(name: Atom, auth: Arc<NativeObjsAuth>, codes: Arc<Vec<Vec<u8>>>) -> Self {
        DukProcessFactory {
            name,
            auth,
            codes,
            pool: Arc::new(RwLock::new(XHashMap::default())),
        }
    }
}