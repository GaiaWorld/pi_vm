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
    receiver:   i32,            //虚拟机异步接收消息的回调入口
    catcher:    i32,            //虚拟机捕获异常的回调入口
}

impl Process<(Arc<NativeObjsAuth>, Arc<Vec<Vec<u8>>>), Box<FnOnce(Arc<JS>) -> usize>, GenType> for DukProcess {
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
                catcher: 0,
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

    fn queue_len(&self) -> usize {
        self.vm.get_queue_len()
    }

    fn call(&mut self, module: String, function: String, args: Box<FnOnce(Arc<JS>) -> usize>) -> Result<Self::Output, Self::Error> {
        let init_status = ProcStatus::Init.into();
        let running_status = ProcStatus::Running.into();
        match self.status.compare_and_swap(init_status, running_status, Ordering::SeqCst) {
            init_status => {
                //当前进程可以启动
                let init_call = Atom::from(module.clone() + "." + function.as_str());
                self.init_call = Some(init_call.clone());
                self.call_init(module, function, args);
                Ok(())
            },
            status => {
                //无效的进程状态
                Err(Error::new(ErrorKind::Other, format!("invalid duk process status, pid: {:?}, name: {:?}, status: {:?}", self.pid, self.name, status)))
            }
        }
    }

    fn info(&self, info: ProcInfo<GenType>) -> Result<(), Self::Error> {
        let running_status: u8 = ProcStatus::Running.into();
        match self.status.load(Ordering::SeqCst) {
            running_status => {
                //当前进程正在运行
                let args = Box::new(move |vm: Arc<JS>| {
                    gen_args_to_js_args(vm, Some(info.source()), info.payload())
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
    //调用进程虚拟机的初始函数，将等待函数执行完成后返回
    pub fn call_init(&self, module: String, function: String, args: Box<FnOnce(Arc<JS>) -> usize>) {
        let vm = self.vm.clone();

        //加载指定模块
        vm.get_link_function("Module.require".to_string());
        vm.new_str(module.clone());
        vm.call(1);

        //调用指定模块的初始函数
        let vm_copy = vm.clone();
        let func = Box::new(move |_lock| {
            vm_copy.get_link_function("Module.modules[\"".to_string() + &module + "\"].exports." + &function);
            let args_size = args(vm_copy.clone());
            vm_copy.call(args_size);
        });
        cast_js_task(TaskType::Async(false), self.priority, None, func, Atom::from(format!("DukProcess Task, pid: {:?}, name: {:?}", self.pid, self.name)));

        //等待调用初始函数完成
        while !vm.is_ran() {
            pause();
        }
    }

    //设置进程虚拟机，接收异步消息的回调入口，设置为正数，虚拟机将无法自动退出
    pub fn set_receiver(&mut self, receiver: i32) {
        self.receiver = receiver;
    }

    //取消进程虚拟机，接收异步消息的回调入口，设置为负数，虚拟机将在执行完所有任务后自动退出
    pub fn unset_receiver(&mut self) {
        if self.receiver == 0 {
            self.receiver = i32::min_value();
            return;
        }

        self.receiver = -self.receiver;
    }

    //设置进程虚拟机，捕获异常的回调入口，设置为正数，虚拟机将无法自动退出
    pub fn set_catcher(&mut self, catcher: i32) {
        self.catcher = catcher;
        self.vm.set_catcher(catcher);
    }

    //取消进程虚拟机，捕获异常的回调入口，设置为负数，虚拟机将在执行完所有任务后自动退出
    pub fn unset_catcher(&mut self) {
        if self.catcher == 0 {
            self.catcher = i32::min_value();
            return;
        }

        self.catcher = -self.catcher;
        self.vm.set_catcher(-1);
    }

    //在当前进程中抛出一个异常
    pub fn throw(&self, error: String)
        -> Result<(), <Self as Process<(Arc<NativeObjsAuth>, Arc<Vec<Vec<u8>>>), Box<FnOnce(Arc<JS>) -> usize>, GenType>>::Error> {
        let running_status: u8 = ProcStatus::Running.into();
        match self.status.load(Ordering::SeqCst) {
            running_status => {
                //当前进程正在运行
                let args = Box::new(move |vm: Arc<JS>| {
                    vm.new_str(error);
                    1
                });
                push_msg(self.vm.clone(), self.catcher, args, Atom::from(format!("DukProcess Throw Task, pid: {:?}, name: {:?}", self.pid, self.name)));
                Ok(())
            },
            status => {
                //当前进程未运行
                Err(Error::new(ErrorKind::Other, format!("duk process not running, pid: {:?}, name: {:?}, status: {:?}", self.pid, self.name, status)))
            }
        }
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
               args: GenType) -> Result<(), Self::Error> {
        if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
            let vm_args = Box::new(move |vm: Arc<JS>| {
                gen_args_to_js_args(vm, None, &args)
            });
            return process.call(module, function, vm_args);
        }

        Err(Error::new(ErrorKind::Other, format!("duk process startup failed, pid: {:?}, module: {:?}, function: {:?}, reason: process not exists", pid, module, function)))
    }

    fn queue_len(&self, pid: u64) -> Option<usize> {
        if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
            return Some(process.queue_len());
        }

        None
    }

    fn set_receiver(&self, pid: u64, receiver: GenType) -> Result<(), Self::Error> {
        if let GenType::U32(callback) = receiver {
            if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
                process.set_receiver(callback as i32);
                Ok(())
            } else {
                Err(Error::new(ErrorKind::Other, format!("duk process set receiver failed, pid: {:?}, reason: process not exists", pid)))
            }
        } else {
            Err(Error::new(ErrorKind::Other, format!("duk process set receiver failed, pid: {:?}, reason: invalid receiver", pid)))
        }
    }

    fn unset_receiver(&self, pid: u64) -> Result<(), Self::Error> {
        if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
            process.unset_receiver();
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, format!("duk process unset receiver failed, pid: {:?}, reason: process not exists", pid)))
        }
    }

    fn set_catcher(&self, pid: u64, catcher: GenType) -> Result<(), Self::Error> {
        if let GenType::U32(callback) = catcher {
            if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
                process.set_catcher(callback as i32);
                Ok(())
            } else {
                Err(Error::new(ErrorKind::Other, format!("duk process set catcher failed, pid: {:?}, reason: process not exists", pid)))
            }
        } else {
            Err(Error::new(ErrorKind::Other, format!("duk process set catcher failed, pid: {:?}, reason: invalid receiver", pid)))
        }
    }

    fn unset_catcher(&self, pid: u64) -> Result<(), Self::Error> {
        if let Some(process) = self.pool.write().get_mut(&(pid as usize)) {
            process.unset_catcher();
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, format!("duk process unset catcher failed, pid: {:?}, reason: process not exists", pid)))
        }
    }

    fn send(&self, src: u64, dst: u64, mut msg: GenType) -> Result<(), Self::Error> {
        if let Some(process) = self.pool.read().get(&(dst as usize)) {
            return process.info(ProcInfo::new(src, dst, msg));
        }

        Err(Error::new(ErrorKind::Other, format!("send msg to duk process failed, src: {:?}, dst: {:?}, reason: process not exists", src, dst)))
    }

    fn close(&self, pid: u64, reason: String) -> Result<Option<String>, Self::Error> {
        //移除当前进程的异步消息接收器，并发送关闭消息，以保证进程可以自动回收
        if let Err(e) = self.unset_receiver(pid) {
            return Err(e);
        }
        if let Err(e) = self.send(pid, pid, GenType::Array(vec![])) {
            return Err(e);
        }

        //移除当前进程的异常捕获器，并发送关闭消息，以保证进程可以自动回收
        if let Err(e) = self.unset_catcher(pid) {
            return Err(e);
        }
        if let Err(e) = self.throw(pid, reason) {
            return Err(e);
        }

        //从进程工厂中移除进程，并更新进程状态
        if let Some(process) = self.pool.write().remove(&(pid as usize)) {
            process.status.store(ProcStatus::Closed.into(), Ordering::SeqCst);
            if let Some(name) = process.name {
                Ok(Some((&name).to_string()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
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

    //在指定进程中抛出一个异常
    pub fn throw(&self, pid: u64, error: String) -> Result<(), <Self as ProcessFactory>::Error> {
        if let Some(process) = self.pool.read().get(&(pid as usize)) {
            return process.throw(error);
        }

        Err(Error::new(ErrorKind::Other, format!("trhow error to duk process failed, pid: {:?}, reason: process not exists", pid)))
    }
}

//解析GenType，并构建指定虚拟机的JsType，返回参数数量
fn gen_args_to_js_args(vm: Arc<JS>, src: Option<u64>, args: &GenType) -> usize {
    let mut size = 0;
    if let Some(src) = src {
        //有源进程唯一id
        vm.new_u32(src as u32);
        size += 1;
    }

    if let GenType::Array(args) = args {
        for arg in args {
            match arg {
                GenType::Nil => {
                    vm.new_undefined();
                    size += 1;
                },
                GenType::Bool(val) => {
                    vm.new_boolean(*val);
                    size += 1;
                },
                GenType::F64(val) => {
                    vm.new_f64(*val);
                    size += 1;
                },
                GenType::Str(val) => {
                    if let Err(e) = vm.new_str(val.clone()) {
                        panic!("native string to js string failed, reason: {:?}", e);
                    }
                    size += 1;
                },
                GenType::Bin(val) => {
                    let buf = vm.new_uint8_array(val.len() as u32);
                    buf.from_bytes(val.as_slice());
                    size += 1;
                },
                GenType::Array(array) => {
                    if let GenType::USize(instance) = array[0] {
                        if let GenType::USize(x) = array[1] {
                            let arr = vm.new_array();
                            let mut obj = vm.new_native_object(instance as usize);
                            if !vm.set_index(&arr, 0, &mut obj) {
                                panic!("native object to js native object failed, reason: set array failed");
                            }
                            let mut num = vm.new_u32(x as u32);
                            if !vm.set_index(&arr, 1, &mut num) {
                                panic!("native number to js number failed, reason: set array failed");
                            }
                            size += 1;
                        } else {
                            panic!("native object to js native object failed, reason: invalid number");
                        }
                    } else {
                        panic!("native object to js native object failed, reason: invalid instance");
                    }
                },
                _ => {
                    panic!("parse args failed, reason: invalid gen type");
                },
            }
        }
    }

    size
}