use std::thread;
use std::boxed::FnBox;
use std::time::Duration;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{Ordering, AtomicUsize};

use magnetic::mpmc::*;
use magnetic::buffer::dynamic::DynamicBuffer;
use magnetic::{Producer, Consumer};

use task::TaskType;
use task_pool::TaskPool;
use adapter::{JSStatus, JS, try_js_destroy, dukc_vm_status_check, dukc_vm_status_switch, dukc_vm_status_sub, dukc_wakeup, dukc_continue, js_reply_callback};

lazy_static! {
	pub static ref JS_TASK_POOL: Arc<(Mutex<TaskPool>, Condvar)> = Arc::new((Mutex::new(TaskPool::new(10)), Condvar::new()));
}

/*
* 虚拟机工厂
*/
#[derive(Clone)]
pub struct VMFactory {
    //虚拟机池中虚拟机的数量
    size: Arc<AtomicUsize>,
    //字节码列表
    codes: Arc<Vec<Arc<Vec<u8>>>>,
    //虚拟机生产者
    producer: Arc<MPMCProducer<JS, DynamicBuffer<JS>>>,
    //虚拟机消费者
    consumer: Arc<MPMCConsumer<JS, DynamicBuffer<JS>>>,
}

impl VMFactory {
    //构建一个虚拟机工厂
    pub fn new(mut size: usize) -> Self {
        if size == 0 {
            size = 1;
        }
        let (p, c) = mpmc_queue(DynamicBuffer::new(size).unwrap());
        VMFactory {
            size: Arc::new(AtomicUsize::new(0)),
            codes: Arc::new(Vec::new()),
            producer: Arc::new(p),
            consumer: Arc::new(c),
        }
    }

    //为指定虚拟机工厂增加代码，必须使用所有权，以保证运行时不会不安全的增加代码，复制对象将无法增加代码
    pub fn append(mut self, code: Arc<Vec<u8>>) -> Self {
        match Arc::get_mut(&mut self.codes) {
            None => (),
            Some(ref mut vec) => {
                vec.push(code);
            }
        }
        self
    }

    //获取当前虚拟机池中虚拟机数量
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    //生成一个虚拟机，返回生成前虚拟机池中虚拟机数量，0表示生成失败     
    pub fn produce(&self) -> usize {
        match self.new_vm() {
            None => 0,
            Some(vm) => {
                match self.producer.try_push(vm) {
                    Err(_) => 0,
                    Ok(_) => self.size.fetch_add(1, Ordering::Acquire),
                }
            }
        }
    }

    //从虚拟机池中获取一个虚拟机，并调用指定的js全局函数
    pub fn call(&self, uid: u32, args: Box<FnBox(JS) -> JS>, info: &'static str) {
        match self.consumer.try_pop() {
            Err(_) => {
                //没有空闲虚拟机，则立即构建临时虚拟机
                match self.new_vm() {
                    None => (),
                    Some(mut vm) => {
                        let func = Box::new(move || {
                            vm.get_js_function("_$rpc".to_string());
                            vm = args(vm);
                            vm.call(4);
                        });
                        cast_task(TaskType::Sync, 5000000000 + uid as u64, func, info);
                    }
                }
            }
            Ok(mut vm) => {
                let producer = self.producer.clone();
                let func = Box::new(move || {
                    vm.get_js_function("_$rpc".to_string());
                    vm = args(vm);
                    vm.call(4);
                    //调用完成后复用虚拟机
                    match producer.try_push(vm) {
                        Err(_) => (),
                        Ok(_) => (),
                    }
                });
                cast_task(TaskType::Sync, 5000000000 + uid as u64, func, info);
            },
        }
    }

    //构建一个虚拟机，并加载所有字节码
    fn new_vm(&self) -> Option<JS> {
        match JS::new() {
            None => None,
            Some(vm) => {
                for code in self.codes.iter() {
                    if vm.load(code.as_slice()) {
                        while !vm.is_ran() {
                            pause();
                        }
                        continue;
                    }
                    return None;
                }
                Some(vm)
            }
        }
    }
}

/*
* 线程安全的向任务池投递任务
*/
pub fn cast_task(task_type: TaskType, priority: u64, func: Box<FnBox()>, info: &'static str) {
    let &(ref lock, ref cvar) = &**JS_TASK_POOL;
    let mut task_pool = lock.lock().unwrap();
    (*task_pool).push(task_type, priority, func, info);
    cvar.notify_one();
}

/*
* 线程安全的回应阻塞调用
*/
pub fn block_reply(js: Arc<JS>, result: Box<FnBox(Arc<JS>)>, task_type: TaskType, priority: u64, info: &'static str) {
    let copy_js = js.clone();
    let func = Box::new(move || {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 || 
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_reply(copy_js, result, task_type, priority, info);
            } else {
                let status = dukc_vm_status_switch(copy_js.get_vm(), JSStatus::MultiTask as i8, JSStatus::SingleTask as i8);
                if status == JSStatus::MultiTask as i8 {
                    //同步任务已阻塞虚拟机，则返回指定的值，并唤醒虚拟机继续同步执行
                    dukc_wakeup(copy_js.get_vm(), 0);
                    result(copy_js.clone());
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                    //当前异步任务如果没有投递其它异步任务，则当前异步任务成为同步任务，并在当前异步任务完成后回收虚拟机
                    //否则还有其它异步任务，则回收权利交由其它异步任务
                    dukc_vm_status_sub(copy_js.get_vm(), 1);
                } else {
                    try_js_destroy(&copy_js);
                    panic!("cast block reply task failed");
                }
            }
        }
    });
    cast_task(task_type, priority, func, info);
}

/*
* 线程安全的为阻塞调用抛出异常
*/
pub fn block_throw(js: Arc<JS>, reason: String, task_type: TaskType, priority: u64, info: &'static str) {
    let copy_js = js.clone();
    let func = Box::new(move || {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 || 
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_throw(copy_js, reason, task_type, priority, info);
            } else {
                let status = dukc_vm_status_switch(copy_js.get_vm(), JSStatus::MultiTask as i8, JSStatus::SingleTask as i8);
                if status == JSStatus::MultiTask as i8 {
                    //同步任务已阻塞虚拟机，则抛出指定原因的错误，并唤醒虚拟机继续同步执行
                    dukc_wakeup(copy_js.get_vm(), 1);
                    copy_js.new_str(reason);
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                    //当前异步任务如果没有投递其它异步任务，则当前异步任务成为同步任务，并在当前异步任务完成后回收虚拟机
                    //否则还有其它异步任务，则回收权利交由其它异步任务
                    dukc_vm_status_sub(copy_js.get_vm(), 1);
                } else {
                    try_js_destroy(&copy_js);
                    panic!("cast block throw task failed");
                }
            }
        }
    });
    cast_task(task_type, priority, func, info);
}

#[cfg(all(feature="unstable", any(target_arch = "x86", target_arch = "x86_64")))]
#[inline(always)]
pub fn pause() {
    unsafe { asm!("PAUSE") };
}

#[cfg(all(not(feature="unstable"), any(target_arch = "x86", target_arch = "x86_64")))]
#[inline(always)]
pub fn pause() {
    thread::sleep(Duration::from_millis(1));
}

#[cfg(all(not(target_arch = "x86"), not(target_arch = "x86_64")))]
#[inline(always)]
pub fn pause() {
    thread::sleep(Duration::from_millis(1));
}