use std::boxed::FnBox;
use std::mem::transmute;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter, Result};

use adapter::JSType;

/*
* 任务类型
*/
#[derive(Copy, Clone, Debug)]
pub enum TaskType {
    Empty,      //空任务
    Async,      //异步任务
    Sync,       //同步任务
    SyncImme,   //同步立即任务
}

/*
* 任务结构
*/
pub struct Task {
    uid:            u32,                //任务唯一id
    task_type:      TaskType,           //任务类型
    priority:       u32,                //任务优先级
    func:           (usize, usize),     //任务函数
    args:           Vec<JSType>,        //任务参数
    start_time:     i64,                //任务开始时间
    finish_time:    i64                 //任务完成时间
}

unsafe impl Sync for Task {} //声明保证多线程安全性

impl Display for Task {
	fn fmt(&self, f: &mut Formatter) -> Result {
		write!(f, "Task[uid = {}, type = {:?}, priority = {}, start_time = {}, finish_time = {}]", 
        self.uid, self.task_type, self.priority, self.start_time, self.finish_time)
	}
}

impl Task {
    pub fn new() -> Self {
        Task {
            uid:            0,
            task_type:      TaskType::Empty,
            priority:       0,
            func:           (0, 0),
            args:           Vec::new(),
            start_time:     0,
            finish_time:    0,
        }
    }

    pub fn copy_to(mut self, dest: &mut Self) -> Self {
        //复制其它成员
        dest.uid = *&self.uid;
        dest.task_type = *&self.task_type;
        dest.priority = *&self.priority;
        dest.func = *&self.func;
        //移动源参数
        for index in 0..self.args.len() {
            dest.args[index] = self.args.remove(index);
        }
        self
    }
    
    pub fn get_uid(&self) -> u32 {
        self.uid
    }
    
    pub fn set_uid(&mut self, uid: u32) {
        self.uid = uid;
    }

    pub fn get_type(&self) -> TaskType {
        self.task_type
    }
    
    pub fn set_type(&mut self, task_type: TaskType) {
        self.task_type = task_type;
    }
    
    pub fn get_priority(&self) -> u32 {
        self.priority
    }
    
    pub fn set_priority(&mut self, priority: u32) {
        self.priority = priority;
    }
    
    pub fn set_func(&mut self, func: Option<Box<FnBox()>>) {
        match func {
            Some(f) => {
                let (x, y): (usize, usize) = unsafe { transmute(f) };
                self.func.0 = x;
                self.func.1 = y;
            },
            None => (),
        }
    }
    
    pub fn get_args(&self) -> Vec<JSType> {
        self.args.to_vec()
    }
    
    pub fn add_args(&mut self, arg: JSType) {
        self.args.push(arg);
    }

    pub fn set_args(&mut self, args: Vec<JSType>) {
        self.args = args;
    }
    
    pub fn get_start_time(&self) -> i64 {
        self.start_time
    }
    
    pub fn set_start_time(&mut self, start_time: i64) {
        self.start_time = start_time;
    }
    
    pub fn get_finish_time(&self) -> i64 {
        self.finish_time
    }
    
    pub fn set_finish_time(&mut self, finish_time: i64) {
        self.finish_time = finish_time;
    }

    pub fn reset(&mut self) {
        self.uid = 0;
        self.task_type = TaskType::Empty;
        self.priority = 0;
        self.func = (0, 0);
        self.args.clear();
        self.start_time = 0;
        self.finish_time = 0;
    }

    pub fn run(&self) {
        if self.func == (0, 0) {
            return;
        }
        let func: Box<FnBox()> = unsafe { transmute(self.func) };
        func();
    }
}

/*
* 任务缓存结构
*/
pub struct TaskCache {
    cache: VecDeque<Task>, //任务缓存
}

impl Display for TaskCache {
	fn fmt(&self, f: &mut Formatter) -> Result {
		write!(f, "TaskCache[size = {}]", self.cache.len())
	}
}

impl TaskCache {
    pub fn new(len: u32) -> Self {
        if len < 1 {
            panic!("invalid task cache size");
        }

        let mut cache = VecDeque::with_capacity(len as usize);
        for _ in 0..len {
            cache.push_back(Task::new());
        }
        TaskCache {
            cache: cache,
        }
    }
    
    pub fn pop(&mut self) -> Task {
        match self.cache.pop_front() {
            Some(e) => e,
            None => Task::new(),
        }
    }
    
    pub fn push(&mut self, mut entry: Task) {
        entry.reset();
        self.cache.push_back(entry);
    }
    
    pub fn clean(&mut self) -> usize {
        //TODO...
        self.size()
    }
    
    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

