use std::boxed::FnBox;
use std::ffi::CString;
use std::sync::{Arc, RwLock};

use fnv::FnvHashMap;
use npnc::bounded::mpmc::{channel as npnc_channel, Producer, Consumer};

use worker::task::TaskType;
use worker::impls::{create_js_task_queue, unlock_js_task_queue, cast_js_task, remove_js_task_queue};
use handler::Handler;
use atom::Atom;

use adapter::{JSStatus, JS, JSType, pause, js_reply_callback, handle_async_callback, try_js_destroy, dukc_vm_status_check, dukc_vm_status_switch, dukc_new_error, dukc_wakeup, dukc_continue};
use channel_map::VMChannelMap;
use bonmgr::NativeObjsAuth;

/*
* 虚拟机任务默认优先级
*/
const JS_TASK_PRIORITY: usize = 100;

/*
* 虚拟机通道
*/
lazy_static! {
	pub static ref VM_CHANNELS: Arc<RwLock<VMChannelMap>> = Arc::new(RwLock::new(VMChannelMap::new(0)));
}

/*
* 虚拟机工厂同步任务队列表
*/
lazy_static! {
	pub static ref VM_FACTORY_QUEUES: Arc<RwLock<FnvHashMap<usize, isize>>> = Arc::new(RwLock::new(FnvHashMap::default()));
}

/*
* 虚拟机工厂
*/
#[derive(Clone)]
pub struct VMFactory {
    codes: Arc<Vec<Arc<Vec<u8>>>>,                  //字节码列表
    producer: Arc<Producer<Arc<JS>>>,               //虚拟机生产者
    consumer: Arc<Consumer<Arc<JS>>>,               //虚拟机消费者
    auth: Arc<NativeObjsAuth>,                      //虚拟机工厂本地对象授权
}

impl VMFactory {
    //构建一个虚拟机工厂
    pub fn new(mut size: usize, auth: Arc<NativeObjsAuth>) -> Self {
        if size == 0 {
            size = 1;
        }
        let (p, c) = npnc_channel(size);
        VMFactory {
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
        self.producer.len()
    }

    //生成一个虚拟机，返回生成前虚拟机池中虚拟机数量，0表示生成失败     
    pub fn produce(&self) -> usize {
        match self.new_vm(self.auth.clone()) {
            None => 0,
            Some(vm) => {
                match self.producer.produce(vm) {
                    Err(e) => {
                        println!("!!!> Vm Factory Produce Failed, e: {:?}", e);
                        0
                    },
                    Ok(_) => self.size(),
                }
            }
        }
    }

    //生成并取出一个虚拟机
    pub fn take(&self) -> Option<Arc<JS>> {
        self.new_vm(self.auth.clone())
    }

    //从虚拟机池中获取一个虚拟机，根据源创建同步任务队列，并调用指定的js全局函数
    pub fn call(&self, src: Option<usize>, port: Atom, args: Box<FnBox(Arc<JS>) -> usize>, info: Atom) {
        //弹出虚拟机，以保证同一时间只有一个线程访问同一个虚拟机
        match self.consumer.consume() {
            Err(_) => {
                //没有空闲虚拟机，则立即构建临时虚拟机
                match self.new_vm(self.auth.clone()) {
                    None => panic!("!!!> Vm Factory Call Error, new vm failed"),
                    Some(vm) => {
                        let vm_copy = vm.clone();
                        let func = Box::new(move |lock: Option<isize>| {
                            if let Some(queue) = lock {
                                //为虚拟机设置当前任务的队列
                                vm_copy.set_tasks(queue);
                            }
                            vm_copy.get_link_function((&port).to_string());
                            let args_size = args(vm_copy.clone());
                            vm_copy.call(args_size);
                        });
                        match src {
                            None => {
                                cast_js_task(TaskType::Async(false), JS_TASK_PRIORITY, None, func, info);
                            },
                            Some(src_id) => {
                                cast_js_task(TaskType::Sync(true), 0, Some(new_queue(src_id)), func, info);
                            },
                        }
                    }
                }
            },
            Ok(vm) => {
                let vm_copy = vm.clone();
                let producer = self.producer.clone();
                let func = Box::new(move |lock: Option<isize>| {
                    if let Some(queue) = lock {
                        //为虚拟机设置当前任务的队列
                        vm_copy.set_tasks(lock.unwrap());
                    }
                    vm_copy.get_link_function(port.to_string());
                    let args_size = args(vm_copy.clone());
                    vm_copy.call(args_size);
                    //调用完成后复用虚拟机
                    match producer.produce(vm_copy) {
                        Err(e) => {
                            println!("!!!> Vm Factory Reused Failed, e: {:?}", e);
                        },
                        Ok(_) => (),
                    }
                });
                match src {
                    None => {
                        cast_js_task(TaskType::Async(false), JS_TASK_PRIORITY, None, func, info);
                    },
                    Some(src_id) => {
                        cast_js_task(TaskType::Sync(true), 0, Some(new_queue(src_id)), func, info);
                    },
                }
            },
        }
    }

    //构建一个虚拟机，加载所有字节码，并提供虚拟机本地对象授权
    fn new_vm(&self, auth: Arc<NativeObjsAuth>) -> Option<Arc<JS>> {
        match JS::new(auth.clone()) {
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
* 阻塞调用错误
*/
#[derive(Debug, Clone)]
pub enum BlockError {
    Unknow(String),
    NewGlobalVar(String),
    SetGlobalVar(String),
}

//线程安全的构建指定源的同步任务队列，如果已存在，则忽略
pub fn new_queue(src: usize) -> isize {
    //检查指定源的同步任务队列是否存在
    {
        let queues = VM_FACTORY_QUEUES.read().unwrap();
        if let Some(q) = (*queues).get(&src) {
            //存在，则返回
            return q.clone();
        }
    }

    //为指定源创建同步任务队列
    {
        let queue = create_js_task_queue(JS_TASK_PRIORITY, false);
        let mut queues = VM_FACTORY_QUEUES.write().unwrap();
        (*queues).insert(src, queue.clone());
        queue
    }
}

//线程安全的移除指定源的同步任务队列，如果不存在，则忽略
pub fn remove_queue(src: usize) -> Option<isize> {
    let mut queues = VM_FACTORY_QUEUES.write().unwrap();
    if let Some(q) = (*queues).remove(&src) {
        if remove_js_task_queue(q) {
            return Some(q);
        }
    }
    None
}

/*
* 线程安全的在阻塞调用中设置全局变量，设置成功后执行下一个操作
* 全局变量构建函数执行成功后，当前值栈必须存在且只允许存在一个值，失败则必须移除在值栈上的构建的所有值
*/
pub fn block_set_global_var(js: Arc<JS>, name: String, var: Box<FnBox(Arc<JS>) -> Result<JSType, String>>, next: Box<FnBox(Result<Arc<JS>, BlockError>)>, info: Atom) {
    let copy_js = js.clone();
    let copy_info = info.clone();
    let func = Box::new(move |_lock| {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 ||
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_set_global_var(copy_js, name, var, next, copy_info);
            } else {
                if dukc_vm_status_check(copy_js.get_vm(), JSStatus::MultiTask as i8) > 0 {
                    //同步任务已阻塞虚拟机，则继续执行下一个操作
                    match var(copy_js.clone()) {
                        Err(reason) => {
                            //构建全局变量错误
                            next(Err(BlockError::NewGlobalVar(reason)));
                        }
                        Ok(value) => {
                            //构建全局变量成功
                            if copy_js.set_global_var(name.clone(), value) {
                                //设置全局变量成功
                                next(Ok(copy_js));
                            } else {
                                //设置全局变量错误
                                next(Err(BlockError::SetGlobalVar(name)));
                            }
                        },
                    }
                } else {
                    //再次检查同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                    block_set_global_var(copy_js, name, var, next, copy_info);
                }
            }
        }
    });

    let queue = js.get_queue();
    cast_js_task(TaskType::Sync(false), 0, Some(queue), func, info); //将任务投递到虚拟机消息队列
    js.add_queue_len(); //增加虚拟机消息队列长度
    //解锁虚拟机的消息队列
    if !unlock_js_task_queue(queue) {
        println!("!!!> Block Set Global Var Error, unlock js task queue failed");
    }
}

/*
* 线程安全的回应阻塞调用
* 返回值构建函数执行完成后，当前值栈必须存在且只允许存在一个值
*/
pub fn block_reply(js: Arc<JS>, result: Box<FnBox(Arc<JS>)>, info: Atom) {
    let copy_js = js.clone();
    let copy_info = info.clone();
    let func = Box::new(move |_lock| {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 || 
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_reply(copy_js, result, copy_info);
            } else {
                let status = dukc_vm_status_switch(copy_js.get_vm(), JSStatus::MultiTask as i8, JSStatus::SingleTask as i8);
                if status == JSStatus::MultiTask as i8 {
                    //同步任务已阻塞虚拟机，则返回指定的值，并唤醒虚拟机继续同步执行
                    dukc_wakeup(copy_js.get_vm(), 0);
                    result(copy_js.clone());
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                } else {
                    //再次检查同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                    block_reply(copy_js, result, copy_info);
                }
            }
        }
    });
    let queue = js.get_queue();
    cast_js_task(TaskType::Sync(false), 0, Some(queue), func, info); //将任务投递到虚拟机消息队列
    js.add_queue_len(); //增加虚拟机消息队列长度
    //解锁虚拟机的消息队列
    if !unlock_js_task_queue(queue) {
        panic!("!!!> Block Reply Error, unlock js task queue failed");
    }
}

/*
* 线程安全的为阻塞调用抛出异常
*/
pub fn block_throw(js: Arc<JS>, reason: String, info: Atom) {
    let copy_js = js.clone();
    let copy_info = info.clone();
    let func = Box::new(move |_lock| {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 || 
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_throw(copy_js, reason, copy_info);
            } else {
                let status = dukc_vm_status_switch(copy_js.get_vm(), JSStatus::MultiTask as i8, JSStatus::SingleTask as i8);
                if status == JSStatus::MultiTask as i8 {
                    //同步任务已阻塞虚拟机，则抛出指定原因的错误，并唤醒虚拟机继续同步执行
                    dukc_wakeup(copy_js.get_vm(), 1);
                    dukc_new_error(copy_js.get_vm(), CString::new(reason).unwrap().as_ptr());
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                } else {
                    //再次检查同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                    block_throw(copy_js, reason, copy_info);
                }
            }
        }
    });
    let queue = js.get_queue();
    cast_js_task(TaskType::Sync(false), 0, Some(queue), func, info); //将任务投递到虚拟机消息队列
    js.add_queue_len(); //增加虚拟机消息队列长度
    //解锁虚拟机的消息队列
    if !unlock_js_task_queue(queue) {
        panic!("!!!> Block Throw Error, unlock js task queue failed");
    }
}

/*
* 线程安全的向虚拟机推送异步回调函数，返回当前虚拟机异步消息队列长度，如果返回0，则表示推送失败
*/
pub fn push_callback(js: Arc<JS>, callback: u32, args: Box<FnBox(Arc<JS>) -> usize>, info: Atom) -> usize {
    let count = JS::push(js.clone(), TaskType::Sync(true), callback, args, info);
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
