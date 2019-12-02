use std::sync::Arc;
use std::io::{Error, ErrorKind};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use hash::XHashMap;
use parking_lot::RwLock;

use handler::GenType;
use atom::Atom;

use proc::{ProcStatus, ProcessFactory};

/*
* 全局进程池
*/
lazy_static! {
    static ref GLOBAL_PROCESS_POOL: ProcessPool = ProcessPool::new();
}

/*
* 进程池
*/
pub struct ProcessPool {
    uid:        Arc<AtomicU64>,                                                                         //进程分配id，进程唯一id从1开始分配，0表示未知进程
    factorys:   Arc<RwLock<XHashMap<Atom, Arc<dyn ProcessFactory<Error = Error>>>>>,                    //进程工厂表
    processes:  Arc<RwLock<XHashMap<u64, (Arc<AtomicU8>, Arc<dyn ProcessFactory<Error = Error>>)>>>,    //进程注册表
    names:      Arc<RwLock<XHashMap<String, (u64, Arc<dyn ProcessFactory<Error = Error>>)>>>,           //进程名称注册表
}

unsafe impl Send for ProcessPool {}
unsafe impl Sync for ProcessPool {}

impl ProcessPool {
    //构建进程池
    pub fn new() -> Self {
        ProcessPool {
            uid: Arc::new(AtomicU64::new(1)),
            factorys: Arc::new(RwLock::new(XHashMap::default())),
            processes: Arc::new(RwLock::new(XHashMap::default())),
            names: Arc::new(RwLock::new(XHashMap::default())),
        }
    }

    //分配新进程的唯一id
    pub fn alloc_pid(&self) -> u64 {
        self.uid.fetch_add(1, Ordering::Relaxed)
    }
}

/*
* 线程安全的设置指定名称的进程工厂
*/
pub fn set_factory(name: Atom, factory: Arc<dyn ProcessFactory<Error = Error>>) {
    GLOBAL_PROCESS_POOL.factorys.write().insert(name, factory);
}

/*
* 线程安全的生成指定类型、名称和MFA的新进程，成功返回进程唯一id
*/
pub fn spawn_process(name: Option<String>,
                     factory_name: Atom,
                     module: String,
                     function: String,
                     init: String,
                     args: GenType) -> Result<u64, Error> {
    if let Some(factory) = GLOBAL_PROCESS_POOL.factorys.read().get(&factory_name) {
        let pid = GLOBAL_PROCESS_POOL.alloc_pid();
        if let Err(e) = factory.new_process(pid, name) {
            //构建指定工厂的进程错误，则立即返回错误原因
            return Err(e);
        }

        if let Err(e) = factory.startup(pid, module, function, init, args) {
            //启动进程错误，则立即返回错误原因
            return Err(e);
        }

        return Ok(pid);
    }

    Err(Error::new(ErrorKind::Other, format!("process factory not exist, name: {:?}", factory_name)))
}

/*
* 线程安全的注册进程
*/
pub fn register_process(pid: u64, name: Option<String>, status: Arc<AtomicU8>, factory_name: Atom) {
    if let Some(factory) = GLOBAL_PROCESS_POOL.factorys.read().get(&factory_name) {
        GLOBAL_PROCESS_POOL.processes.write().insert(pid, (status, factory.clone()));
        if let Some(name) = name {
            //有进程名称，则注册进程名称
            GLOBAL_PROCESS_POOL.names.write().insert(name, (pid, factory.clone()));
        }
    }
}

/*
* 线程安全的通过名称获取进程唯一id，没有注册的进程名称，将返回None
*/
pub fn name_to_pid(name: &String) -> Option<u64> {
    if let Some((pid, _)) = GLOBAL_PROCESS_POOL.names.read().get(name) {
        return Some(*pid);
    }

    None
}

/*
* 线程安全的获取指定进程的运行状态
*/
pub fn get_status(pid: u64) -> Option<ProcStatus> {
    if pid == 0 {
        //无效的进程
        return None;
    }

    if let Some((status, _)) = GLOBAL_PROCESS_POOL.processes.read().get(&pid) {
        return Some(status.load(Ordering::SeqCst).into())
    }

    None
}

/*
* 线程安全的获取指定进程的消息队列长度
*/
pub fn queue_len(pid: u64) -> Option<usize> {
    if pid == 0 {
        //无效的进程
        return None;
    }

    if let Some((_, factory)) = GLOBAL_PROCESS_POOL.processes.read().get(&pid) {
        return factory.queue_len(pid);
    }

    None
}

/*
* 线程安全的设置指定进程的异步消息接收器
*/
pub fn set_receiver(pid: u64, receiver: GenType) -> Result<(), Error> {
    if let Some((_, factory)) = GLOBAL_PROCESS_POOL.processes.read().get(&pid) {
        factory.set_receiver(pid, receiver)
    } else {
        //进程对应的工厂不存在
        Err(Error::new(ErrorKind::Other, format!("set receiver failed, pid: {:?}, process factory not exist", pid)))
    }
}

/*
* 线程安全的取消指定进程的异步消息接收器
*/
pub fn unset_receiver(pid: u64) -> Result<(), Error> {
    if let Some((_, factory)) = GLOBAL_PROCESS_POOL.processes.read().get(&pid) {
        factory.unset_receiver(pid)
    } else {
        //进程对应的工厂不存在
        Err(Error::new(ErrorKind::Other, format!("unset receiver failed, pid: {:?}, process factory not exist", pid)))
    }
}

/*
* 线程安全的设置指定进程的异常捕获器
*/
pub fn set_catcher(pid: u64, catcher: GenType) -> Result<(), Error> {
    if let Some((_, factory)) = GLOBAL_PROCESS_POOL.processes.read().get(&pid) {
        factory.set_catcher(pid, catcher)
    } else {
        //进程对应的工厂不存在
        Err(Error::new(ErrorKind::Other, format!("set catcher failed, pid: {:?}, process factory not exist", pid)))
    }
}

/*
* 线程安全的取消指定进程的异常捕获器
*/
pub fn unset_catcher(pid: u64) -> Result<(), Error> {
    if let Some((_, factory)) = GLOBAL_PROCESS_POOL.processes.read().get(&pid) {
        factory.unset_catcher(pid)
    } else {
        //进程对应的工厂不存在
        Err(Error::new(ErrorKind::Other, format!("unset catcher failed, pid: {:?}, process factory not exist", pid)))
    }
}

/*
* 线程安全的指定进程发送异步消息，src为0表示未知进程, dst必须大于0
*/
pub fn pid_send(src: u64, dst: u64, msg: GenType) -> Result<(), Error> {
    if dst == 0 {
        //无效的目标进程
        return Err(Error::new(ErrorKind::Other, format!("pid send failed, src: {:?}, dst: {:?}, reason: invalid dst", src, dst)));
    }

    if let Some((_, factory)) = GLOBAL_PROCESS_POOL.processes.read().get(&dst) {
        factory.send(src, dst, msg)
    } else {
        //进程对应的工厂不存在
        Err(Error::new(ErrorKind::Other, format!("pid send failed, src: {:?}, dst: {:?}, reason: process factory not exist", src, dst)))
    }
}

/*
* 线程安全的指定进程发送异步消息，src为0表示未知进程
*/
pub fn name_send(src: u64, dst: String, msg: GenType) -> Result<(), Error> {
    if dst == "" {
        //无效的目标进程
        return Err(Error::new(ErrorKind::Other, format!("name send failed, src: {:?}, dst: {:?}, reason: invalid dst", src, dst)));
    }

    if let Some((pid, factory)) = GLOBAL_PROCESS_POOL.names.read().get(&dst) {
        factory.send(src, *pid, msg)
    } else {
        //进程对应的工厂不存在
        Err(Error::new(ErrorKind::Other, format!("name send failed, src: {:?}, dst: {:?}, reason: process factory not exist", src, dst)))
    }
}

/*
* 线程安全的关闭指定进程
*/
pub fn close_process(pid: u64, reason: String) -> Result<(), Error> {
    if pid == 0 {
        //无效的进程
        return Err(Error::new(ErrorKind::Other, format!("close process failed, reason: invalid pid")));
    }

    //从指定工厂中关闭指定进程
    let result;
    if let Some((_, factory)) = GLOBAL_PROCESS_POOL.processes.read().get(&pid) {
        result = factory.close(pid, reason);
    } else {
        //进程对应的工厂不存在
        return Err(Error::new(ErrorKind::Other, format!("close process failed, pid: {:?}, reason: process factory not exist", pid)))
    }

    //清理进程池中的进程信息
    match result {
        Err(e) => return Err(e),
        Ok(Some(name)) => {
            //进程有名称，则从进程名称表中移除进程信息
            GLOBAL_PROCESS_POOL.names.write().remove(&name);
        },
        _ => (),
    }
    GLOBAL_PROCESS_POOL.processes.write().remove(&pid); //从进程表中移除进程信息

    Ok(())
}