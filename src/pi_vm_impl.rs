use std::thread;
use std::sync::Arc;
use std::boxed::FnBox;
use std::time::Duration;
use std::sync::atomic::{Ordering, AtomicUsize};

use magnetic::mpmc::*;
use magnetic::{Producer, Consumer};
use magnetic::buffer::dynamic::DynamicBuffer;

use pi_base::task::TaskType;
use pi_base::pi_base_impl::cast_js_task;
use adapter::{JSStatus, JSMsg, JS, try_js_destroy, dukc_vm_status_check, dukc_vm_status_switch, dukc_wakeup, dukc_continue, js_reply_callback};
use pi_lib::atom::Atom;

/*
* 默认虚拟机异步消息队列最大长度
*/
const VM_MSG_QUEUE_MAX_SIZE: u16 = 0xff;

/*
* 虚拟机工厂
*/
#[derive(Clone)]
pub struct VMFactory {
    size: Arc<AtomicUsize>,                             //虚拟机池中虚拟机的数量
    codes: Arc<Vec<Arc<Vec<u8>>>>,                      //字节码列表
    producer: Arc<MPMCProducer<Arc<JS>, DynamicBuffer<Arc<JS>>>>, //虚拟机生产者
    consumer: Arc<MPMCConsumer<Arc<JS>, DynamicBuffer<Arc<JS>>>>, //虚拟机消费者
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
        match self.new_vm(VM_MSG_QUEUE_MAX_SIZE) {
            None => 0,
            Some(vm) => {
                match self.producer.try_push(vm) {
                    Err(_) => 0,
                    Ok(_) => self.size.fetch_add(1, Ordering::Acquire) + 1,
                }
            }
        }
    }

    //从虚拟机池中获取一个虚拟机，并调用指定的js全局函数
    pub fn call(&self, uid: u32, args: Box<FnBox(Arc<JS>)>, info: Atom) {
        //弹出虚拟机，以保证同一时间只有一个线程访问同一个虚拟机
        match self.consumer.try_pop() {
            Err(_) => {
                //没有空闲虚拟机，则立即构建临时虚拟机
                match self.new_vm(VM_MSG_QUEUE_MAX_SIZE) {
                    None => (),
                    Some(vm) => {
                        let func = Box::new(move || {
                            vm.get_js_function("_$rpc".to_string());
                            args(vm.clone());
                            vm.call(4);
                        });
                        cast_js_task(TaskType::Sync, 5000000000 + uid as u64, func, info);
                    }
                }
            }
            Ok(vm) => {
                let producer = self.producer.clone();
                let func = Box::new(move || {
                    vm.get_js_function("_$rpc".to_string());
                    args(vm.clone());
                    vm.call(4);
                    //调用完成后复用虚拟机
                    match producer.try_push(vm) {
                        Err(_) => (),
                        Ok(_) => (),
                    }
                });
                cast_js_task(TaskType::Sync, 5000000000 + uid as u64, func, info);
            },
        }
    }

    //构建一个虚拟机，并加载所有字节码
    fn new_vm(&self, queue_max_size: u16) -> Option<Arc<JS>> {
        match JS::new(queue_max_size) {
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
* 线程安全的回应阻塞调用
*/
pub fn block_reply(js: Arc<JS>, result: Box<FnBox(Arc<JS>)>, task_type: TaskType, priority: u64, info: Atom) {
    let copy_js = js.clone();
    let copy_info = info.clone();
    let func = Box::new(move || {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 || 
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_reply(copy_js, result, task_type, priority, copy_info);
            } else {
                let status = dukc_vm_status_switch(copy_js.get_vm(), JSStatus::MultiTask as i8, JSStatus::SingleTask as i8);
                if status == JSStatus::MultiTask as i8 {
                    //同步任务已阻塞虚拟机，则返回指定的值，并唤醒虚拟机继续同步执行
                    dukc_wakeup(copy_js.get_vm(), 0);
                    result(copy_js.clone());
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                } else {
                    try_js_destroy(&copy_js);
                    panic!("cast block reply task failed");
                }
            }
        }
    });
    cast_js_task(task_type, priority, func, info);
}

/*
* 线程安全的为阻塞调用抛出异常
*/
pub fn block_throw(js: Arc<JS>, reason: String, task_type: TaskType, priority: u64, info: Atom) {
    let copy_js = js.clone();
    let copy_info = info.clone();
    let func = Box::new(move || {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 || 
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_throw(copy_js, reason, task_type, priority, copy_info);
            } else {
                let status = dukc_vm_status_switch(copy_js.get_vm(), JSStatus::MultiTask as i8, JSStatus::SingleTask as i8);
                if status == JSStatus::MultiTask as i8 {
                    //同步任务已阻塞虚拟机，则抛出指定原因的错误，并唤醒虚拟机继续同步执行
                    dukc_wakeup(copy_js.get_vm(), 1);
                    copy_js.new_str(reason);
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                } else {
                    try_js_destroy(&copy_js);
                    panic!("cast block throw task failed");
                }
            }
        }
    });
    cast_js_task(task_type, priority, func, info);
}

/*
* 线程安全的向虚拟机推送异步回调函数，返回当前虚拟机异步消息队列长度，如果返回0，则表示推送失败
*/
pub fn push_callback(js: Arc<JS>, callback: u32, args: Box<FnBox(Arc<JS>) -> usize>, info: Atom) -> usize {
    js.push(JSMsg::new(callback, args, info))
}

#[cfg(all(feature="unstable", any(target_arch = "x86", target_arch = "x86_64")))]
#[inline(always)]
fn pause() {
    unsafe { asm!("PAUSE") };
}

#[cfg(all(not(feature="unstable"), any(target_arch = "x86", target_arch = "x86_64")))]
#[inline(always)]
fn pause() {
    thread::sleep(Duration::from_millis(1));
}

#[cfg(all(not(target_arch = "x86"), not(target_arch = "x86_64")))]
#[inline(always)]
fn pause() {
    thread::sleep(Duration::from_millis(1));
}