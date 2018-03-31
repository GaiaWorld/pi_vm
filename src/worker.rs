use std::time;
use std::thread;
use std::sync::RwLock;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::fmt::{Display, Formatter, Result};

use threadpool::ThreadPool;

use task_pool::TaskPool;
use task::Task;

/*
* 控制信号
*/
pub const CONTROL_SIGN_EXIT:         u8 = 0;     //退出
pub const CONTROL_SIGN_SLEEP:        u8 = 1;     //休眠
pub const CONTROL_SIGN_CONTINUE:     u8 = 2;     //继续

/*
* 状态信号
*/
pub const STATUS_STOP:     u8 = 0;     //停止
pub const STATUS_WAIT:     u8 = 1;     //等待
pub const STATUS_RUNNING:  u8 = 2;     //运行中

/*
* 工作者信号
*/
#[derive(Copy, Clone, Debug)]
pub enum WorkerSign {
    Control(u8),    //控制信号
}

/*
* 工作者
*/
#[derive(Debug)]
pub struct Worker {
    uid:        u32,                    //工作者编号
    status:     u8,                     //工作者状态
    sender:     Sender<WorkerSign>,     //发送器
    receiver:   Receiver<WorkerSign>,   //接收器
}

unsafe impl Sync for Worker {} //声明保证多线程安全性

impl Display for Worker {
	fn fmt(&self, f: &mut Formatter) -> Result {
		write!(f, "Worker[uid = {}, status = {}]", self.uid, self.status)
	}
}

impl Worker {
    //创建一个工作者
    pub fn new(uid: u32, sender: Sender<WorkerSign>, receiver: Receiver<WorkerSign>) -> Self {
        Worker {
            uid:        uid,
            status:     STATUS_STOP,
            sender:     sender,
            receiver:   receiver,
        }
    }

    //启动
    pub fn startup(pool: &mut ThreadPool, worker: Arc<RwLock<Worker>>, sync: Arc<(Mutex<TaskPool>, Condvar)>) {
        if worker.read().unwrap().status == STATUS_RUNNING {
            return;
        }

        pool.execute(move|| {
            let mut task = Task::new();
            Worker::work_loop(worker, sync, &mut task);
        });
    }

    //工作循环
    fn work_loop(worker: Arc<RwLock<Worker>>, sync: Arc<(Mutex<TaskPool>, Condvar)>, task: &mut Task) {
        worker.write().unwrap().status = STATUS_RUNNING;
        loop {
            //处理控制信号
            match worker.read().unwrap().receiver.try_recv() {
                Ok(sign) => {
                    match sign {
                        WorkerSign::Control(CONTROL_SIGN_CONTINUE) => {
                            //继续处理任务
                            worker.write().unwrap().status = STATUS_RUNNING;
                        },
                        WorkerSign::Control(CONTROL_SIGN_SLEEP) => {
                            //暂停处理任务，继续等待控制信号
                            worker.write().unwrap().status = STATUS_WAIT;
                            thread::sleep(time::Duration::from_millis(1));
                            continue;
                        }
                        WorkerSign::Control(CONTROL_SIGN_EXIT) => {
                            //退出当前循环
                            worker.write().unwrap().status = STATUS_STOP;
                            break;
                        },
                        _ => (), //忽略其它信号
                    }
                },
                Err(e) => {
                    match e {
                        TryRecvError::Empty => {
                            //没有收到控制信号
                            match worker.read().unwrap().status {
                                STATUS_STOP => break, //退出当前循环
                                STATUS_WAIT => {
                                    //继续等待控制信号
                                    thread::sleep(time::Duration::from_millis(1));
                                    continue;
                                },
                                _ => (), //继续处理任务
                            }
                        },
                        TryRecvError::Disconnected => {
                            //已断开通道
                            //TODO...
                        },
                    }
                },
            }
            worker.read().unwrap().work(&sync, task);
        }
    }

    //获取当前状态
    pub fn get_status(&self) -> u8 {
        self.status
    }

    //工作
    fn work(&self, sync: &Arc<(Mutex<TaskPool>, Condvar)>, task: &mut Task) {
        //同步块
        {
            let &(ref lock, ref cvar) = &**sync;
            let mut task_pool = lock.lock().unwrap();
            while (*task_pool).size() == 0 {
                //等待任务
                task_pool = cvar.wait(task_pool).unwrap();
            }
            (*task_pool).pop(task); //获取任务
        }
        task.run(); //执行任务
    }
}