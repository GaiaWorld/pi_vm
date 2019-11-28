use std::sync::Arc;
use std::error::Error;
use std::sync::atomic::{AtomicU8, Ordering};

use handler::{Args, GenType};

/*
* 进程运行状态
*/
#[derive(Debug, Clone)]
pub enum ProcStatus {
    Init = 0,       //初始化
    Running,        //正在运行，运行中的内部状态
    Scheduling,     //调度中
    Pending,        //挂起中
    Closed,         //已关闭
}

impl From<u8> for ProcStatus {
    fn from(status: u8) -> Self {
        match status {
            0 => ProcStatus::Init,
            1 => ProcStatus::Running,
            2 => ProcStatus::Scheduling,
            3 => ProcStatus::Pending,
            4 => ProcStatus::Closed,
            _ => panic!("invalid status value"),
        }
    }
}

impl From<ProcStatus> for u8 {
    fn from(status: ProcStatus) -> Self {
        match status {
            ProcStatus::Init => 0,
            ProcStatus::Running => 1,
            ProcStatus::Scheduling => 2,
            ProcStatus::Pending => 3,
            ProcStatus::Closed => 4,
            _ => panic!("invalid status"),
        }
    }
}

impl From<Arc<AtomicU8>> for ProcStatus {
    fn from(status: Arc<AtomicU8>) -> Self {
        match status.load(Ordering::SeqCst) {
            0 => ProcStatus::Init,
            1 => ProcStatus::Running,
            2 => ProcStatus::Scheduling,
            3 => ProcStatus::Pending,
            4 => ProcStatus::Closed,
            _ => panic!("invalid status value"),
        }
    }
}

impl From<ProcStatus> for Arc<AtomicU8> {
    fn from(status: ProcStatus) -> Self {
        match status {
            ProcStatus::Init => Arc::new(AtomicU8::new(0)),
            ProcStatus::Running => Arc::new(AtomicU8::new(1)),
            ProcStatus::Scheduling => Arc::new(AtomicU8::new(2)),
            ProcStatus::Pending => Arc::new(AtomicU8::new(3)),
            ProcStatus::Closed => Arc::new(AtomicU8::new(4)),
            _ => panic!("invalid status"),
        }
    }
}

/*
* 进程消息
*/
pub struct ProcInfo<Payload: 'static> {
    src:        u64,        //消息源
    dst:        u64,        //消息目标
    payload:    Payload,    //消息负载
}

impl<Payload: 'static> ProcInfo<Payload> {
    //构建进程消息
    pub fn new(src: u64, dst: u64, payload: Payload) -> Self {
        ProcInfo {
            src,
            dst,
            payload,
        }
    }

    //获取消息源
    pub fn source(&self) -> u64 {
        self.src
    }

    //获取消息目标
    pub fn dest(&self) -> u64 {
        self.dst
    }

    //获取消息负载
    pub fn payload(&self) -> &Payload {
        &self.payload
    }
}

/*
* 进程，抽象的底层计算资源，用于执行脚本。为脚本运行分配、调度和回收，计算和存储资源。
* 进程之间有唯一且统一的通讯方式。源进程可以向目标进程异步发送一个消息，成功发送的消息被缓冲到目标进程的一个顺序队列中等待被调度执行。
*/
pub trait Process<Options: 'static, Args: 'static, Payload: 'static>: 'static {
    type Process: 'static;
    type Output: 'static;
    type Error: Error;

    //初始化进程所需要的计算和存储资源，并返回进程
    fn init(pid: u64, name: Option<String>, options: Options) -> Result<Self::Process, Self::Error>;

    //获取当前进程的唯一id
    fn pid(&self) -> u64;

    //获取当前进程的名称
    fn name(&self) -> Option<&str>;

    //获取当前进程的运行状态
    fn status(&self) -> ProcStatus;

    //启动进程，并调用指定脚本，执行完成则返回执行结果
    fn call(&mut self, module: String, function: String, args: Args) -> Result<Self::Output, Self::Error>;

    //当前进程接收到的消息
    fn info(&self, info: ProcInfo<Payload>) -> Result<(), Self::Error>;
}

/*
* 进程工厂，隔离不同进程实现的差异
*/
pub trait ProcessFactory: 'static {
    type Error: Error;

    //获取进程工厂的名称
    fn name(&self) -> &str;

    //构建一个进程
    fn new_process(&self, pid: u64, name: Option<String>) -> Result<(), Self::Error>;

    //启动指定进程
    fn startup(&self,
               pid: u64,
               module: String,
               function: String,
               args: Args<GenType, GenType, GenType, GenType, GenType, GenType, GenType, GenType>)
        -> Result<(), Self::Error>;

    //设置指定进程的异步消息接收器
    fn set_receiver(&self, pid: u64, receiver: GenType) -> Result<(), Self::Error>;

    //向指定进程发送消息
    fn send(&self, src: u64, dst: u64, msg: GenType) -> Result<(), Self::Error>;
}