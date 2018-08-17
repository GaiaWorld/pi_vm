use std::boxed::FnBox;
use std::ffi::CString;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{Ordering, AtomicUsize};

use magnetic::mpmc::*;
use magnetic::{Producer, Consumer};
use magnetic::buffer::dynamic::DynamicBuffer;

use pi_base::task::TaskType;
use pi_base::pi_base_impl::cast_js_task;
use pi_lib::handler::Handler;
use pi_lib::atom::Atom;

use adapter::{JSStatus, JSMsg, JS, JSType, pause, js_reply_callback, handle_async_callback, try_js_destroy, dukc_vm_status_check, dukc_vm_status_switch, dukc_new_error, dukc_wakeup, dukc_continue};
use channel_map::VMChannelMap;
use bonmgr::NativeObjsAuth;

/*
* 默认虚拟机异步消息队列最大字节数
*/
const VM_MSG_QUEUE_MAX_SIZE: usize = 0xffffff;

/*
* 虚拟机通道
*/
lazy_static! {
	pub static ref VM_CHANNELS: Arc<RwLock<VMChannelMap>> = Arc::new(RwLock::new(VMChannelMap::new(0)));
}

/*
* 虚拟机工厂
*/
#[derive(Clone)]
pub struct VMFactory {
    size: Arc<AtomicUsize>,                                         //虚拟机池中虚拟机的数量
    codes: Arc<Vec<Arc<Vec<u8>>>>,                                  //字节码列表
    producer: Arc<MPMCProducer<Arc<JS>, DynamicBuffer<Arc<JS>>>>,   //虚拟机生产者
    consumer: Arc<MPMCConsumer<Arc<JS>, DynamicBuffer<Arc<JS>>>>,   //虚拟机消费者
    auth: Arc<NativeObjsAuth>,                                      //虚拟机工厂本地对象授权
}

impl VMFactory {
    //构建一个虚拟机工厂
    pub fn new(mut size: usize, auth: Arc<NativeObjsAuth>) -> Self {
        if size == 0 {
            size = 1;
        }
        let (p, c) = mpmc_queue(DynamicBuffer::new(size).unwrap());
        VMFactory {
            size: Arc::new(AtomicUsize::new(0)),
            codes: Arc::new(Vec::new()),
            producer: Arc::new(p),
            consumer: Arc::new(c),
            auth: auth.clone(),
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
        match self.new_vm(VM_MSG_QUEUE_MAX_SIZE, self.auth.clone()) {
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
    pub fn call(&self, uid: u32, port: Atom, args: Box<FnBox(Arc<JS>) -> usize>, info: Atom) {
        //弹出虚拟机，以保证同一时间只有一个线程访问同一个虚拟机
        match self.consumer.try_pop() {
            Err(_) => {
                //没有空闲虚拟机，则立即构建临时虚拟机
                match self.new_vm(VM_MSG_QUEUE_MAX_SIZE, self.auth.clone()) {
                    None => (),
                    Some(vm) => {
                        let func = Box::new(move || {
                            vm.get_js_function((&port).to_string());
                            let args_size = args(vm.clone());
                            vm.call(args_size);
                        });
                        cast_js_task(TaskType::Sync, 5000000000 + uid as u64, func, info);
                    }
                }
            }
            Ok(vm) => {
                let producer = self.producer.clone();
                let func = Box::new(move || {
                    vm.get_js_function(port.to_string());
                    let args_size = args(vm.clone());
                    vm.call(args_size);
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

    //构建一个虚拟机，加载所有字节码，并提供虚拟机本地对象授权
    fn new_vm(&self, queue_max_size: usize, auth: Arc<NativeObjsAuth>) -> Option<Arc<JS>> {
        match JS::new(queue_max_size, auth.clone()) {
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
                    dukc_new_error(copy_js.get_vm(), CString::new(reason).unwrap().as_ptr());
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
    let count = js.push(JSMsg::new(callback, args, info));
    unsafe {
        let vm = js.get_vm();
        let status = dukc_vm_status_switch(vm, JSStatus::WaitCallBack as i8, JSStatus::SingleTask as i8);
        if status == JSStatus::WaitCallBack as i8 {
            //当前虚拟机等待异步回调，因为其它任务已执行完成，任务结果已经从值栈中弹出，则只需立即执行异步回调函数
            handle_async_callback(js, vm);
        }
    }
    count
}

/*
* 线程安全的获取虚拟机通道灰度值
*/
pub fn get_channels_gray() -> usize {
    let ref lock = &**VM_CHANNELS;
    let channels = lock.read().unwrap();
    (*channels).get_gray()
}

/*
* 线程安全的设置虚拟机通道灰度值
*/
pub fn set_channels_gray(gray: usize) -> usize {
    let ref lock = &**VM_CHANNELS;
    let mut channels = lock.write().unwrap();
    (*channels).set_gray(gray)
}

/*
* 线程安全的获取虚拟机通道异步调用数量
*/
pub fn get_async_request_size() -> usize {
    let ref lock = &**VM_CHANNELS;
    let channels = lock.read().unwrap();
    (*channels).size()
}

/*
* 线程安全的在虚拟机通道注册异步调用
*/
pub fn register_async_request(name: Atom, handler: Arc<Handler<A = Arc<Vec<u8>>, B = Vec<JSType>, C = Option<u32>, D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>) -> Option<Arc<Handler<A = Arc<Vec<u8>>, B = Vec<JSType>, C = Option<u32>, D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>> {
    let ref lock = &**VM_CHANNELS;
    let mut channels = lock.write().unwrap();
    (*channels).set(name, handler)
}

/*
* 线程安全的在虚拟机通道注销异步调用
*/
pub fn unregister_async_request(name: Atom) -> Option<Arc<Handler<A = Arc<Vec<u8>>, B = Vec<JSType>, C = Option<u32>, D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>> {
    let ref lock = &**VM_CHANNELS;
    let mut channels = lock.write().unwrap();
    (*channels).remove(name)
}

/*
* 线程安全的通过虚拟机通道向对端发送异步请求
*/
pub fn async_request(js: Arc<JS>, name: Atom, msg: Arc<Vec<u8>>, native_objs: Vec<usize>, callback: Option<u32>) -> bool {
    let ref lock = &**VM_CHANNELS;
    let channels = lock.read().unwrap();
    (*channels).request(js, name, msg, native_objs, callback)
}
