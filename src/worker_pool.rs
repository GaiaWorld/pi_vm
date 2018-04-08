use std::collections::HashMap;
use std::sync::{Arc, Mutex, Condvar};
use std::fmt::{Display, Formatter, Result as FmtResult}; //避免和标准Result冲突，改名为FmtResult

use threadpool::ThreadPool;

use task_pool::TaskPool;
use worker::{WorkerStatus, Worker};

/*
* 工作者池
*/
pub struct WorkerPool {
    counter:        u32,                        //工作者编号计数器
    map:            HashMap<u32, Arc<Worker>>,  //工作者缓存
    thread_pool:    ThreadPool,                 //线程池
}

impl Display for WorkerPool {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "WorkerPool[counter = {}, worker_size = {}, wait_size = {}, active_size = {}, panic_size = {}]", 
        self.counter, self.size(), self.thread_pool.queued_count(), self.thread_pool.active_count(), self.thread_pool.panic_count())
	}
}

impl WorkerPool {
    //构建指定数量工作者的工作者池
    pub fn new(len: usize, slow: u32) -> Self {
        let mut counter: u32 = 0;
        let mut map = HashMap::new();
        for _ in 0..len {
            counter += 1;
            map.insert(counter, Arc::new(Worker::new(counter, slow)));
        }
        WorkerPool {
            counter:        counter,
            map:            map,
            thread_pool:    ThreadPool::new(len),
        }
    }

    //获取工作者数量
    pub fn size(&self) -> u32 {
        self.map.len() as u32
    }

    //获取指定状态的工作者编号数组
    pub fn workers(&self, status: usize) -> Vec<u32> {
        let mut vec = Vec::<u32>::new();
        for (uid, worker) in self.map.iter() {
            if worker.get_status() == status {
                vec.push(*uid);
            }
        }
        vec
    }

    //休眠指定工作者
    pub fn sleep(&self, uid: u32) -> bool {
        match self.map.get(&uid) {
            Some(worker) => {
                worker.set_status(WorkerStatus::Running, WorkerStatus::Wait)
            },
            None => false,
        }
    }

    //唤醒指定工作者
    pub fn wakeup(&self, uid: u32) -> bool {
        match self.map.get(&uid) {
            Some(worker) => {
                worker.set_status(WorkerStatus::Wait, WorkerStatus::Running)
            },
            None => false,
        }
    }

    //停止指定工作者
    pub fn stop(&mut self, uid: u32) -> bool {
        match self.map.get_mut(&uid) {
            Some(worker) => {
                worker.stop()
            },
            None => false,
        }
    }

    //启动工作者，启动时需要指定任务池的同步对象
    pub fn start(&mut self, sync: Arc<(Mutex<TaskPool>, Condvar)>, uid: u32) -> bool {
        match self.map.get_mut(&uid) {
            Some(worker) => {
                if worker.set_status(WorkerStatus::Wait, WorkerStatus::Running) {
                    Worker::startup(&mut self.thread_pool, worker.clone(), sync.clone())
                } else {
                    false
                }
            },
            None => false,
        }
    }

    //在指定任务池中，运行工作池，需要指定任务池的同步对象
    pub fn run(&mut self, sync: Arc<(Mutex<TaskPool>, Condvar)>) {
        for (_, worker) in self.map.iter() {
            if worker.set_status(WorkerStatus::Wait, WorkerStatus::Running) {
                Worker::startup(&mut self.thread_pool, worker.clone(), sync.clone());
            }
        }
    }
}
