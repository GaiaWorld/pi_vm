use std::thread;
use std::boxed::FnBox;
use std::cell::RefCell;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::io::{Result, ErrorKind, Error};
use std::env::{current_dir, current_exe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::hash_map::{Entry, DefaultHasher};

use libc::c_char;

use atom::Atom;
use worker::task::TaskType;
use worker::impls::{create_js_task_queue, cast_js_task, unlock_js_task_queue};

use adapter::{JSStatus, JS, dukc_vm_status_check, dukc_vm_status_switch, dukc_vm_status_sub, dukc_callback_count, dukc_top, dukc_to_string, dukc_pop, handle_async_callback};
use pi_vm_impl::{VMFactoryLoader, VMFactory, new_queue, remove_queue};
use bonmgr::{NativeObjsAuth, ptr_jstype};

/*
* shell源最小值
*/
const SHELL_MIN_SRC: usize = 0x100000000;

/*
* shell源最大值
*/
const SHELL_MAX_SRC: usize = 0x100100000;

/*
* shell指令，大小写敏感
*/
const SHELL_COMMAND_CLEAN: &[u8] = b"clean"; //清空所有缓存的已编译脚本
const SHELL_CURRENT_DIR: &[u8] = b"pwd"; //当前工作目录
const SHELL_CURRENT_EXE: &[u8] = b"exe"; //当前执行程序

/*
* shell脚本文件名
*/
const SHELL_SCRIPT_FILE: &'static str = "__SHELL_SCRIPT__";

/*
* shell脚本函数前缀
*/
const SHELL_SCRIPT_FUNCTION_PREFIX: &'static str = "__shell_func_";

/*
* shell设置全局环境函数名
*/
const SHELL_SET_GLOBAL_ENV_FUNC: &'static str = "_$defineGlobal";

/*
* shell设置全局环境文件名
*/
const SHELL_SET_GLOBAL_ENV_FILE_NAME: &'static str = "__DEFINE_GLOBAL__";

/*
* shell设置全局环境代码
*/
const SHELL_SET_GLOBAL_ENV_CODE: &'static str =
    r#"function _$defineGlobal(key, value){
        if(self[key]){
            throw "There has been a global variable " + key;
        }

        self[key] = value;
        console.log("set global env", key, "ok");
    }"#;

/*
* 等待shell同步阻塞返回任务优先级
*/
const SHELL_WAIT_BLOCK_REPLY_TASK_PRIORITY: usize = 100;

/*
* 系统shell管理器
*/
lazy_static! {
	pub static ref SHELL_MANAGER: Arc<RwLock<ShellManager>> = Arc::new(RwLock::new(ShellManager::new()));
}

/*
* shell状态
*/
#[derive(Debug, Clone)]
pub enum ShellStatus {
    Closed = -1,    //已关闭
    Opened,         //已打开
    Connected,      //已连接
}

/*
* shell环境值
*/
#[derive(Debug, Clone)]
enum ShellEnvValue {
    Boolean(bool),
    Integer(u32),
    Float(f64),
    String(String),
    NativeObject(usize, u32), //元信息hash值和指针值
}

/*
* shell全局环境
*/
struct ShellGlobalEnv(HashMap<String, ShellEnvValue>);

/*
* shell管理器
*/
pub struct ShellManager {
    id: usize,                                          //shell分配id
    factory: Option<VMFactory>,                         //shell虚拟机工厂
    shells: HashMap<usize, (ShellStatus, Shell)>,    //shell表
    env: ShellGlobalEnv,                                //shell全局环境
}

unsafe impl Send for ShellManager {}
unsafe impl Sync for ShellManager {}

impl ShellManager {
    //构建shell管理器
    pub fn new() -> Self {
        ShellManager {
            id: SHELL_MIN_SRC,
            factory: None,
            shells: HashMap::new(),
            env: ShellGlobalEnv(HashMap::new()),
        }
    }

    //初始化shell管理器
    pub fn init(&mut self, codes: Option<Vec<Arc<Vec<u8>>>>) {
        if self.factory.is_some() {
            //已初始化，则忽略
            return;
        }

        //使用临时虚拟机，编译全局环境初始化的代码
        let tmp = JS::new(1, Atom::from("tmp vm"), 0, Arc::new(NativeObjsAuth::new(None, None)), None).unwrap();
        let init_code = Arc::new(tmp.compile(SHELL_SET_GLOBAL_ENV_FILE_NAME.to_string(), SHELL_SET_GLOBAL_ENV_CODE.to_string()).unwrap());

        //顺序加载全局环境初始化代码和其它代码
        let mut factory = VMFactory::new("native shell", 0, 0, Arc::new(NativeObjsAuth::new(None, None)));
        factory = factory.append(init_code);
        if let Some(list) = codes {
            for code in list {
                factory = factory.append(code);
            }
        }

        self.factory = Some(factory);
    }

    //获取全局环境数量
    pub fn env_size(&self) -> usize {
        self.env.0.len()
    }

    //增加布尔类型的全局环境
    pub fn add_bool_env(&mut self, key: &str, value: bool) {
        self.env.0.insert(key.to_string(), ShellEnvValue::Boolean(value));
    }

    //增加整数类型的全局环境
    pub fn add_int_env(&mut self, key: &str, value: u32) {
        self.env.0.insert(key.to_string(), ShellEnvValue::Integer(value));
    }

    //增加浮点类型的全局环境
    pub fn add_float_env(&mut self, key: &str, value: f64) {
        self.env.0.insert(key.to_string(), ShellEnvValue::Float(value));
    }

    //增加字符串类型的全局环境
    pub fn add_string_env(&mut self, key: &str, value: &str) {
        self.env.0.insert(key.to_string(), ShellEnvValue::String(value.to_string()));
    }

    //增加NativeObject的全局环境
    pub fn add_natobj_env(&mut self, key: &str, value: usize, hash: u32) {
        self.env.0.insert(key.to_string(), ShellEnvValue::NativeObject(value, hash));
    }

    //shell数量
    pub fn size(&self) -> usize {
        self.shells.len()
    }

    //指定shell的状态
    pub fn status(&self, id: usize) -> ShellStatus {
        if let Some((status, _)) = self.shells.get(&id) {
            return status.clone();
        }
        ShellStatus::Closed
    }

    //指定shell是否正在运行
    pub fn running(&self, id: usize) -> bool {
        if let Some((status, shell)) = self.shells.get(&id) {
            match status {
                ShellStatus::Closed => {
                    return false;
                },
                _ => {
                    return shell.vm.is_ran();
                },
            }
        }
        false
    }

    //打开shell
    pub fn open(&mut self) -> Option<usize> {
        if self.factory.is_none() {
            return None;
        }

        if let Some(vm) = self.factory.as_ref().unwrap().take() {
            let id = self.id;
            self.close(id); //强制关闭已存在的同id的shell

            //构建并初始化shell
            let loader = self.factory.as_ref().unwrap().loader();
            let shell = Shell::new(self.id, vm.clone());
            shell.init(loader, &self.env);
            self.shells.insert(self.id, (ShellStatus::Opened, shell));

            self.id += 1;
            if self.id >= SHELL_MAX_SRC {
                //shell分配id已达上限，则重新分配
                self.id = SHELL_MIN_SRC;
            }

            return Some(id)
        } else {
            return None
        }
    }

    //初始化shell字符输出函数
    pub fn init_char_output(&self, id: usize, output: extern fn(*const c_char)) {
        if let Some((_, shell)) = self.shells.get(&id) {
            shell.vm.init_char_output(output);
        }
    }

    //连接shell，连接成功，返回请求回调
    pub fn connect(&mut self,
                   id: usize,
                   resp: Arc<Fn(Result<Arc<Vec<u8>>>, Option<Box<FnBox(Arc<Vec<u8>>)>>)>) -> Option<Box<FnBox(Arc<Vec<u8>>)>> {

        match self.shells.entry(id) {
            Entry::Vacant(_) => {
                //指定shell不存在
                None
            },
            Entry::Occupied(ref mut entry) => {
                let status = entry.get().0.clone();
                match status {
                    ShellStatus::Opened => {
                        //已打开，且未连接，则连接
                        let value = entry.get_mut();
                        value.0 = ShellStatus::Connected; //设置shell状态为已连接
                        value.1.resp = Some(resp); //设置shell对端指定的响应回调
                        value.1.accept(true); //设置shell为接受
                        Some(value.1.new_request())
                    },
                    _ => {
                        //未打开或已连接
                        None
                    },
                }
            },
        }
    }

    //关闭指定连接
    pub fn disconnect(&mut self, id: usize) {
        match self.shells.entry(id) {
            Entry::Vacant(_) => {
                //指定shell不存在
                return;
            },
            Entry::Occupied(ref mut entry) => {
                let status = entry.get().0.clone();
                match status {
                    ShellStatus::Connected => {
                        //已连接，则关闭
                        let value = entry.get_mut();
                        value.0 = ShellStatus::Opened; //设置shell状态为未连接
                        value.1.resp = None; //移除shell对端的响应回调
                        value.1.accept(false); //设置shell为不接受
                    },
                    _ => {
                        //未连接
                        return;
                    }
                }
            },
        }
    }

    //关闭shell，不立即关闭shell，shell会在运行完当前任务后结束
    pub fn close(&mut self, id: usize) {
        if self.shells.contains_key(&id) {
            //shell存在
            remove_queue(id); //移除虚拟机对应的同步任务队列
            self.shells.remove(&id);
        }
    }
}

/*
* shell
*/
#[derive(Clone)]
pub struct Shell {
    src: usize,                                                                     //shell源
    vm: Arc<JS>,                                                                    //shell虚拟机
    resp: Option<Arc<Fn(Result<Arc<Vec<u8>>>, Option<Box<FnBox(Arc<Vec<u8>>)>>)>>,  //响应回调，参数包括执行结果和下次请求回调
    is_accept: Arc<AtomicBool>,                                                     //是否接受对端请求
    complied: Arc<RefCell<HashMap<u64, String>>>,                                //已编译脚本缓存
}

impl Shell {
    //构建shell
    fn new(src: usize, vm: Arc<JS>) -> Self {
        Shell {
            src,
            vm,
            resp: None,
            is_accept: Arc::new(AtomicBool::new(false)),
            complied: Arc::new(RefCell::new(HashMap::default())),
        }
    }

    //初始化shell的全局环境
    fn init(&self, mut loader: VMFactoryLoader, env: &ShellGlobalEnv) {
        //加载基础字节码
        loader.load_next(&self.vm);
        loader.load_next(&self.vm);

        //设置全局环境
        for key in env.0.keys() {
            if let Some(value) = env.0.get(key) {
                //有环境，则在当前shell虚拟机中调用设置全局环境的函数
                self.vm.get_js_function(SHELL_SET_GLOBAL_ENV_FUNC.to_string());
                self.vm.new_str(key.clone());

                match value {
                    ShellEnvValue::Boolean(v) => {
                        self.vm.new_boolean(*v);
                    },
                    ShellEnvValue::Integer(v) => {
                        self.vm.new_u32(*v);
                    },
                    ShellEnvValue::Float(v) => {
                        self.vm.new_f64(*v);
                    },
                    ShellEnvValue::String(v) => {
                        self.vm.new_str(v.to_string());
                    },
                    ShellEnvValue::NativeObject(v, h) => {
                        ptr_jstype(self.vm.get_objs(), self.vm.clone(), *v, *h);
                    },
                }

                self.vm.call(2);
            }
        }

        //加载剩余字节码
        while loader.load_next(&self.vm) {}
    }

    //线程安全的设置是否已连接，返回上个状态
    fn accept(&self, b: bool) -> bool {
        self.is_accept.swap(b, Ordering::SeqCst)
    }

    //构建请求回调，每次向已连接的shell发送请求，都有唯一的一个请求回调，防止客户端的原因，导致虚拟机无法正常回收
    fn new_request(&self) -> Box<FnBox(Arc<Vec<u8>>)> {
        let shell = self.clone();
        Box::new(move |bin: Arc<Vec<u8>>| {
            cast_shell_task(Arc::new(shell), bin);
        })
    }
}

//投递shell任务
fn cast_shell_task(shell: Arc<Shell>, bin: Arc<Vec<u8>>,) {
    let queue = new_queue(shell.src);
    let func = Box::new(move |lock: Option<isize>| {
        if let Some(queue) = lock {
            //为虚拟机设置当前任务的队列
            shell.vm.set_tasks(queue);
        }

        if !shell.is_accept.load(Ordering::SeqCst) {
            //未连接，则忽略请求
            return;
        }

        //执行请求
        let script = String::from_utf8_lossy(&bin[..]).into_owned();
        if handle_command(shell.clone(), &script) {
            //指令已处理，构建下一次的请求回调，并响应本次请求
            let shell_copy = shell.clone();
            let req: Option<Box<FnBox(Arc<Vec<u8>>)>> = Some(Box::new(move |bin: Arc<Vec<u8>>| {
                if !unlock_js_task_queue(shell_copy.vm.get_tasks()) {
                    panic!("!!!> Unlock js task queue failed, queue: {}", shell_copy.vm.get_tasks());
                }

                cast_shell_task(shell_copy, bin);
            }));
            shell.resp.as_ref().unwrap()(Ok(Arc::new("ok".to_string().into_bytes())), req);
        } else {
            //编译并执行脚本
            if complie_eval(shell.clone(), script) {
                wait_shell_reply(shell); //等待虚拟机执行任务后再响应本次请求
            } else {
                //构建下一次的请求回调，并响应本次请求
                let shell_copy = shell.clone();
                let req: Option<Box<FnBox(Arc<Vec<u8>>)>> = Some(Box::new(move |bin: Arc<Vec<u8>>| {
                    if !unlock_js_task_queue(shell_copy.vm.get_tasks()) {
                        panic!("!!!> Unlock js task queue failed, queue: {}", shell_copy.vm.get_tasks());
                    }

                    cast_shell_task(shell_copy, bin);
                }));
                shell.resp.as_ref().unwrap()(Ok(Arc::new("!!!> Invalid Shell Script".to_string().into_bytes())), req);
            }
        }
    });
    cast_js_task(TaskType::Sync(true), 0, Some(queue), func, Atom::from("shell task"));
}

//等待shell执行完所有同步任务和同步阻塞任务，并响应本次请求
fn wait_shell_reply(shell: Arc<Shell>) {
    let func = Box::new(move |_lock: Option<isize>| {
        //检查当前虚拟机是否已执行完所有同步任务、同步阻塞任务和异步回调任务，注意不允许使用非安全的异步回调相关函数来获取异步任务数量，因为在同步阻塞任务执行时，调用这些函数会导致异常
        if shell.vm.get_queue_len() == 0 && !shell.vm.is_ran() {
            //没有异步回调函数，且当前虚拟机未执行完成，则继续等待
            return wait_shell_reply(shell);
        }

        let r = shell.vm.get_ret();

        //构建下一次的请求回调，并响应本次请求
        let shell_copy = shell.clone();
        let req: Option<Box<FnBox(Arc<Vec<u8>>)>> = Some(Box::new(move |bin: Arc<Vec<u8>>| {
            if !unlock_js_task_queue(shell_copy.vm.get_tasks()) {
                panic!("!!!> Unlock js task queue failed, queue: {}", shell_copy.vm.get_tasks());
            }

            cast_shell_task(shell_copy, bin);
        }));
        match r {
            None => shell.resp.as_ref().unwrap()(Err(Error::new(ErrorKind::InvalidData, "shell execut error")), req),
            Some(str) => shell.resp.as_ref().unwrap()(Ok(Arc::new(str.into_bytes())), req),
        }
    });
    cast_js_task(TaskType::Async(false), SHELL_WAIT_BLOCK_REPLY_TASK_PRIORITY, None, func, Atom::from("shell wait reply task"));
}

//处理指令脚本
fn handle_command(shell: Arc<Shell>, script: &String) -> bool {
    match script.trim().as_bytes() {
        SHELL_COMMAND_CLEAN => {
            let hash_list: Vec<u64> = shell.complied.borrow().keys().map(|x| x.clone()).collect();
            for func_hash in hash_list {
                if let Some(func_script) = shell.complied.borrow_mut().remove(&func_hash) {
                    let func_name = script_to_func_name(func_hash, &func_script);
                    let value = shell.vm.new_undefined();
                    shell.vm.set_global_var(func_name, value);
                }
            }
            true
        },
        SHELL_CURRENT_DIR => {
            if let Ok(path) = current_dir() {
                println!("{}", path.display());
            }
            true
        },
        SHELL_CURRENT_EXE => {
            if let Ok(path) = current_exe() {
                println!("{}", path.display());
            }
            true
        }
        _ => {
            //未定义指令，则忽略
            false
        },
    }
}

//编译并执行脚本
fn complie_eval(shell: Arc<Shell>, script: String) -> bool {
    let mut b = false;
    let func_hash = script_to_hash(&script);
    let func_name = script_to_func_name(func_hash, &script);

    if let Some(last_script) = shell.complied.borrow().get(&func_hash) {
        //脚本hash相同
        if &script == last_script {
            //脚本内容相同
            b = true;
        }
    }

    if !b {
        //脚本未编译，则编译，并缓存
        if let Some(code) = shell.vm.compile(SHELL_SCRIPT_FILE.to_string(), script_to_func(&func_name, &script)) {
            if shell.vm.load(code.as_slice()) {
                while !shell.vm.is_ran() {
                    //加载未完成
                    thread::sleep(Duration::from_millis(1000));
                }

                //缓存已编译的脚本
                shell.complied.borrow_mut().insert(func_hash, script);
            } else {
                //加载脚本失败
                return false;
            }
        } else {
            //编译脚本失败
            return false;
        }
    }

    //执行已编译的脚本
    if shell.vm.set_ret(Some("undefined".to_string())) {
        if shell.vm.get_js_function(func_name) {
            shell.vm.call(0);
            return true;
        }
    }
    false
}

//将脚本转换为脚本函数
fn script_to_func(func_name: &String, script: &String) -> String {
    "function ".to_string() + func_name + "() {\n" + script + "}"
}

//构建脚本函数名
fn script_to_func_name(hash: u64, script: &String) -> String {
    SHELL_SCRIPT_FUNCTION_PREFIX.to_string() + &hash.to_string()
}

//计算脚本hash
fn script_to_hash(script: &String) -> u64 {
    let mut hasher = DefaultHasher::new();
    script.hash(&mut hasher);
    hasher.finish()
}