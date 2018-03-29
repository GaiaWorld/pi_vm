use std::sync::RwLock;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver, RecvError, TryRecvError};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult}; //避免和标准Result冲突，改名为FmtResult

use threadpool::ThreadPool;

use task_pool::TaskPool;
use worker::{CONTROL_SIGN_EXIT, CONTROL_SIGN_SLEEP, CONTROL_SIGN_CONTINUE};
use worker::{STATUS_STOP, STATUS_WAIT, STATUS_RUNNING};
use worker::{Worker, WorkerSign};

/*
* 工作者池
*/
pub struct WorkerPool {
    counter:        u32,                                                        //工作者编号计数器
    receiver:       Receiver<WorkerSign>,                                       //工作者池接收器
    map:            HashMap<u32, (Sender<WorkerSign>, Arc<RwLock<Worker>>)>,    //工作者缓存
    thread_pool:    ThreadPool,                                                 //线程池
}

impl Display for WorkerPool {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "WorkerPool[counter = {}, worker_size = {}, wait_size = {}, active_size = {}, panic_size = {}]", 
        self.counter, self.size(), self.thread_pool.queued_count(), self.thread_pool.active_count(), self.thread_pool.panic_count())
	}
}

impl WorkerPool {
    //构建指定数量工作者的工作者池
    pub fn new(len: usize) -> Self {
        let mut counter: u32 = 0;
        let (p_sender, p_receiver) = channel(); //工作者池通道
        let mut map = HashMap::new();
        for _ in 0..len {
            counter += 1;
            let (w_sender, w_receiver) = channel(); //工作者通道
            map.insert(counter, (
                                    w_sender, 
                                    Arc::new(RwLock::new(Worker::new(counter, p_sender.clone(), w_receiver)))
                                )
            );
        }
        WorkerPool {
            counter:        counter,
            receiver:       p_receiver,
            map:            map,
            thread_pool:    ThreadPool::new(len),
        }
    }

    //获取工作者数量
    pub fn size(&self) -> u32 {
        self.map.len() as u32
    }

    //获取指定状态的工作者编号数组
    pub fn workers(&self, status: u8) -> Vec<u32> {
        let mut vec = Vec::<u32>::new();
        for (uid, pair) in self.map.iter() {
            let (_, ref worker): (Sender<WorkerSign>, Arc<RwLock<Worker>>) = *pair;
            if worker.read().unwrap().get_status() == status {
                vec.push(*uid);
            }
        }
        vec
    }

    //休眠指定工作者
    pub fn sleep(&self, uid: u32) -> bool {
        match self.map.get(&uid) {
            Some(pair) => {
                let (ref sender, ref worker): (Sender<WorkerSign>, Arc<RwLock<Worker>>) = *pair;
                if worker.read().unwrap().get_status() != STATUS_RUNNING {
                    return false;
                }
                sender.send(WorkerSign::Control(CONTROL_SIGN_SLEEP)).is_ok()
            },
            None => false,
        }
    }

    //唤醒指定工作者
    pub fn wakeup(&self, uid: u32) -> bool {
        match self.map.get(&uid) {
            Some(pair) => {
                let (ref sender, ref worker): (Sender<WorkerSign>, Arc<RwLock<Worker>>) = *pair;
                if worker.read().unwrap().get_status() != STATUS_WAIT {
                    return false;
                }
                sender.send(WorkerSign::Control(CONTROL_SIGN_CONTINUE)).is_ok()
            },
            None => false,
        }
    }

    //停止指定工作者
    pub fn stop(&self, uid: u32) -> bool {
        match self.map.get(&uid) {
            Some(pair) => {
                let (ref sender, ref worker): (Sender<WorkerSign>, Arc<RwLock<Worker>>) = *pair;
                if worker.read().unwrap().get_status() == STATUS_STOP {
                    //如果已停止，则忽略
                    return true;
                }
                sender.send(WorkerSign::Control(CONTROL_SIGN_EXIT)).is_ok()
            },
            None => false,
        }
    }

    //启动工作者，启动时需要指定任务池的同步对象
    pub fn start(&mut self, sync: Arc<(Mutex<TaskPool>, Condvar)>, uid: u32) -> bool {
        match self.map.get_mut(&uid) {
            Some(pair) => {
                let (_, ref worker): (Sender<WorkerSign>, Arc<RwLock<Worker>>) = *pair;
                if worker.read().unwrap().get_status() != STATUS_STOP {
                    return false;
                }
                Worker::startup(&mut self.thread_pool, worker.clone(), sync.clone());
                true
            },
            None => false,
        }
    }

    //在指定任务池中，运行工作池，需要指定任务池的同步对象
    pub fn run(&mut self, sync: Arc<(Mutex<TaskPool>, Condvar)>) {
        for (_, pair) in self.map.iter() {
            let (_, ref worker): (Sender<WorkerSign>, Arc<RwLock<Worker>>) = *pair;
            Worker::startup(&mut self.thread_pool, worker.clone(), sync.clone());
        }
    }

    //阻塞并接收工作者信号
    pub fn recv(&self) -> Result<WorkerSign, RecvError> {
        self.receiver.recv()
    }

    //接收工作者信号
    pub fn try_recv(&self) -> Result<WorkerSign, TryRecvError> {
        self.receiver.try_recv()
    }
}