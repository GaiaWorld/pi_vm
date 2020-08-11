use std::ffi::CString;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicIsize, Ordering};

use libc::c_char;
use crossbeam_channel::{Sender, Receiver, unbounded};

use worker::task::TaskType;
use worker::impls::{create_js_task_queue, unlock_js_task_queue, cast_js_task, remove_js_task_queue};
use handler::Handler;
use atom::Atom;
use apm::allocator::{get_max_alloced_limit, is_alloced_limit, all_alloced_size};
use apm::counter::{GLOBAL_PREF_COLLECT, PrefCounter, PrefTimer};
use lfstack::{CollectResult, LFStack};

use adapter::{VM_FACTORY_REGISTERS, JSStatus, JS, JSType, pause, js_reply_callback, handle_async_callback, try_js_destroy, dukc_vm_status_check, dukc_vm_status_switch, dukc_new_error, dukc_wakeup, dukc_continue, now_utc};
use channel_map::VMChannelMap;
use bonmgr::NativeObjsAuth;
use std::sync::atomic::Ordering::SeqCst;

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
	pub static ref VM_FACTORY_QUEUES: Arc<RwLock<HashMap<usize, isize>>> = Arc::new(RwLock::new(HashMap::new()));
}

lazy_static! {
    //虚拟机数量
    static ref VM_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_count"), 0).unwrap();
    //虚拟机构建总时长
    static ref VM_NEW_TIME: PrefTimer = GLOBAL_PREF_COLLECT.new_static_timer(Atom::from("vm_new_time"), 0).unwrap();
    //虚拟机加载总时长
    static ref VM_LOAD_TIME: PrefTimer = GLOBAL_PREF_COLLECT.new_static_timer(Atom::from("vm_load_time"), 0).unwrap();
    //虚拟机调用数量
    static ref VM_CALL_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_call_count"), 0).unwrap();
    //虚拟机推送异步回调数量
    static ref VM_PUSH_CALLBACK_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_push_callback_count"), 0).unwrap();
    //虚拟机异步请求数量
    static ref VM_ASYNC_REQUEST_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_async_request_count"), 0).unwrap();
}

/*
* 虚拟机工厂字节码加载器
*/
#[derive(Clone)]
pub struct VMFactoryLoader {
    offset: usize,                  //字节码偏移
    top:    usize,                  //字节码顶指针
    codes:  Arc<Vec<Arc<Vec<u8>>>>, //字节码缓存
}

impl VMFactoryLoader {
    //虚拟机加载下个字节码，返回false，表示已加载所有代码
    pub fn load_next(&mut self, vm: &Arc<JS>) -> bool {
        if self.offset >= self.top {
            //已加载完成
            return false;
        }

        if vm.load(self.codes[self.offset].as_slice()) {
            while !vm.is_ran() {
                pause();
            }
        }

        self.offset += 1; //更新字节码偏移

        true
    }
}

/*
* 虚拟机工厂
*/
#[derive(Clone)]
pub struct VMFactory {
    name:               Atom,                                                                   //虚拟机工厂名
    is_reused:          bool,                                                                   //虚拟机工厂的虚拟机是否可以复用
    limit_capacity:     Arc<AtomicUsize>,                                                       //虚拟机限制容量，可以在限流时用于控制虚拟机工厂最大虚拟机数量，并通过限流整理进行动态调度
    size:               Arc<AtomicUsize>,                                                       //虚拟机工厂当前虚拟机数量
    alloc_id:           Arc<AtomicUsize>,                                                       //虚拟机分配id
    max_reused_count:   usize,                                                                  //虚拟机最大执行次数，当达到虚拟机最大堆限制后才会检查
    heap_size:          usize,                                                                  //虚拟机堆大小
    max_heap_size:      usize,                                                                  //虚拟机最大堆大小，当达到限制后释放可回收的内存
    codes:              Arc<Vec<Arc<Vec<u8>>>>,                                                 //字节码列表
    mods:               Arc<Vec<String>>,                                                       //虚拟机工厂依赖的模块名列表
    pool:               Arc<LFStack<Arc<JS>>>,                                                  //虚拟机池
    scheduling_count:   Arc<AtomicUsize>,                                                       //虚拟机工厂调度次数，调度包括任务队列等待和虚拟机执行
    auth:               Arc<NativeObjsAuth>,                                                    //虚拟机工厂本地对象授权
    vm_buf_sent:        Sender<Arc<JS>>,                                                        //虚拟机临时缓冲发送器
    vm_buf_recv:        Receiver<Arc<JS>>,                                                      //虚拟机临时缓冲接收器
    queue_sent:         Sender<(Option<usize>, Atom, Box<FnOnce(Arc<JS>) -> usize>, Atom)>,     //虚拟机工厂等待调度的任务队列发送器
    queue_recv:         Receiver<(Option<usize>, Atom, Box<FnOnce(Arc<JS>) -> usize>, Atom)>,   //虚拟机工厂等待调度的任务队列接收器
    refuse_count:       Arc<AtomicUsize>,                                                       //虚拟机工厂拒绝任务次数
}

unsafe impl Send for VMFactory {}
unsafe impl Sync for VMFactory {}

impl Drop for VMFactory {
    fn drop(&mut self) {
        self.pool.clear(); //清空虚拟机池
        self.vm_buf_recv.try_iter().collect::<Vec<(Option<usize>, Atom, Box<FnOnce(Arc<JS>) -> usize>, Atom)>>(); //清空等待调度的任务队列
    }
}

impl VMFactory {
    //构建一个虚拟机工厂
    pub fn new(name: &str,
               mut size: usize,
               max_reused_count: usize,
               heap_size: usize,
               max_heap_size: usize,
               auth: Arc<NativeObjsAuth>) -> Self {
        let mut is_reused = true; //默认可复用
        if size == 0 {
            //设置为不可复用
            is_reused = false;
        }

        let (vm_buf_sent, vm_buf_recv) = unbounded();
        let (queue_sent, queue_recv) = unbounded();
        VMFactory {
            name: Atom::from(name),
            is_reused,
            limit_capacity: Arc::new(AtomicUsize::new(0)),
            size: Arc::new(AtomicUsize::new(0)),
            alloc_id: Arc::new(AtomicUsize::new(0)),
            max_reused_count,
            heap_size,
            max_heap_size,
            codes: Arc::new(Vec::new()),
            mods: Arc::new(Vec::new()),
            pool: Arc::new(LFStack::new()),
            scheduling_count: Arc::new(AtomicUsize::new(0)),
            auth: auth.clone(),
            vm_buf_sent,
            vm_buf_recv,
            queue_sent,
            queue_recv,
            refuse_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    //为指定虚拟机工厂增加代码，必须使用所有权，以保证运行时不会不安全的增加代码，复制对象将无法增加代码
    pub fn append(mut self, code: Arc<Vec<u8>>) -> Self {
        match Arc::get_mut(&mut self.codes) {
            None => (),
            Some(vec) => {
                vec.push(code);
            }
        }
        self
    }

    //为指定虚拟机工厂增加指定模块的代码，必须使用所有权，以保证运行时不会不安全的增加代码，复制对象将无法增加代码
    pub fn append_depend(mut self, module: String) -> Self {
        match Arc::get_mut(&mut self.mods) {
            None => (),
            Some(mods) => {
                mods.push(module);
            },
        }
        self
    }

    //判断虚拟机工厂是否依赖指定模块
    pub fn is_depend(&self, module: &String) -> bool {
        self.mods.contains(module)
    }

    //获取虚拟机工厂名
    pub fn name(&self) -> String {
        (*self.name).to_string()
    }

    //获取虚拟机池的限制容量
    pub fn limit_capacity(&self) -> usize {
        self.limit_capacity.load(Ordering::Relaxed)
    }

    //初始化虚拟机池的限制容量，限制容量初始化为当前虚拟机工厂的虚拟机数量
    pub fn init_limit_capacity(&self) {
        if self.limit_capacity.load(Ordering::Relaxed) == 0 {
            //当前未初始化，则初始化
            self.limit_capacity.store(self.size(), Ordering::SeqCst);
        }
    }

    //增加虚拟机池的限制容量，每次增加1
    pub fn add_limit_capacity(&self) {
        self.limit_capacity.fetch_add(1, Ordering::SeqCst);
    }

    //减少虚拟机池的限制容量，每次减少1
    pub fn sub_limit_capacity(&self) {
        self.limit_capacity.fetch_sub(1, Ordering::SeqCst);
    }

    //获取当前虚拟机池中虚拟机数量
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    //获取当前虚拟机池中空闲虚拟机数量
    pub fn free_pool_size(&self) -> usize {
        self.pool.size()
    }

    //获取当前虚拟机临时缓冲区中空闲虚拟机数量
    pub fn free_buf_size(&self) -> usize {
        self.vm_buf_recv.len()
    }

    //获取虚拟机最大执行次数
    pub fn max_reused_count(&self) -> usize {
        self.max_reused_count
    }

    //获取虚拟机堆限制
    pub fn heap_size(&self) -> usize {
        self.heap_size
    }

    //获取虚拟机最大堆限制
    pub fn max_heap_size(&self) -> usize {
        self.max_heap_size
    }

    //获取虚拟机工厂调度次数
    pub fn scheduling_count(&self) -> usize {
        self.scheduling_count.load(Ordering::Relaxed)
    }

    //重置虚拟机工厂调度次数，返回上次调度次数
    pub fn reset_scheduling_count(&self) -> usize {
        self.scheduling_count.swap(0, Ordering::SeqCst)
    }

    //获取虚拟机工厂，任务调度队列的长度
    pub fn queue_len(&self) -> usize {
        self.queue_recv.len()
    }

    //获取虚拟机工厂，任务拒绝的次数
    pub fn refuse_count(&self) -> usize {
        self.refuse_count.load(Ordering::Relaxed)
    }

    //重置虚拟机工厂，任务拒绝的次数
    pub fn reset_refuse_count(&self) {
        self.refuse_count.store(0, Ordering::SeqCst);
    }

    //生成指定数量的虚拟机，不会检查是否达到虚拟机工厂限制容量上限，由外部调用者在需要时检查，返回生成前虚拟机池中虚拟机数量
    pub fn produce(&self, count: usize) -> Result<usize, String> {
        let factory_name = (&self.name).to_string();
        if !VM_FACTORY_REGISTERS.read().unwrap().contains_key(&factory_name) {
            //注册虚拟机工厂
            VM_FACTORY_REGISTERS.write().unwrap().insert(factory_name, self.clone());
        }

        if count == 0 {
            return Ok(count);
        }

        for _ in 0..count {
            match self.new_vm(self.auth.clone()) {
                None => {
                    return Err(format!("vm factory, new vm failed, factory: {:?}",
                                       (&self.name).to_string()))
                },
                Some(vm) => {
                    let r = vm.free_global(); //预生成的虚拟机，将强制GC
                    info!("===> Vm Factory Produce Ok, gc: {},  vm: {:?}", r, vm);
                    self.pool.push(vm); //阻塞的推入虚拟机
                }
            }
        }

        return Ok(self.size());
    }

    //生成指定数量的虚拟机，只在整理时使用，不会检查是否达到虚拟机工厂限制容量上限，由外部调用者在需要时检查，返回生成前虚拟机池中虚拟机数量
    pub fn collect_produce(&self) -> Result<usize, String> {
        match self.new_vm(self.auth.clone()) {
            None => {
                return Err(format!("vm factory, new vm failed, factory: {:?}",
                                   (&self.name).to_string()))
            },
            Some(vm) => {
                let r = vm.free_global(); //预生成的虚拟机，将强制GC
                info!("===> Vm Factory Produce Ok by Collect, gc: {},  vm: {:?}", r, vm);
                self.pool.push(vm); //阻塞的推入虚拟机
            }
        }

        return Ok(self.size());
    }

    //复用指定虚拟机
    pub fn reuse(&self, vm: Arc<JS>) {
        if let Ok((src, port, args, info)) = self.queue_recv.try_recv() {
            //当前虚拟机工厂的任务调度队列中有待运行的任务，则立即使用当前虚拟机，异步运行此任务
            self.async_run(vm, src, port, args, info);
        } else {
            //当前虚拟机工厂的任务调度队列中没有待运行的任务，则将当前虚拟机还给当前虚拟机工厂
            if let Err(_) = self.pool.try_push(vm.clone()) {
                //虚拟机池已阻塞，则将空闲虚拟机加入虚拟机临时缓冲区
                self.vm_buf_sent.send(vm);
            }
        }
    }

    //丢弃指定数量的虚拟机，返回最近虚拟机池中虚拟机数量
    pub fn throw(&self, count: usize) -> usize {
        self.size.fetch_sub(count, Ordering::SeqCst)
    }

    //重置指定数量的虚拟机，返回生成前虚拟机池中虚拟机数量
    pub fn reset(&self, count: usize) -> Result<usize, String> {
        self.size.fetch_sub(count, Ordering::SeqCst);
        self.produce(count)
    }

    //生成并取出一个无法复用的虚拟机，但未加载字节码
    pub fn take(&self) -> Option<Arc<JS>> {
        JS::new(self.alloc_id.fetch_add(1, Ordering::Relaxed), self.name.clone(), self.auth.clone(), None)
    }

    //获取虚拟机工厂字节码加载器
    pub fn loader(&self) -> VMFactoryLoader {
        VMFactoryLoader {
            offset: 0,
            top: self.codes.len(),
            codes: self.codes.clone(),
        }
    }

    //从虚拟机池中获取一个虚拟机，根据源创建同步任务队列，并调用指定的js全局函数
    pub fn call(&self, src: Option<usize>, port: Atom, args: Box<FnOnce(Arc<JS>) -> usize>, info: Atom) {
        //弹出虚拟机，以保证同一时间只有一个线程访问同一个虚拟机
        match self.pool.try_pop() {
            Ok(vm) => {
                //有空闲虚拟机，则运行
                self.async_run(vm, src, port, args, info);
            },
            _ => {
                //当前虚拟机池没有空闲虚拟机，或当前虚拟机池已阻塞
                if let Ok(vm) = self.vm_buf_recv.try_recv() {
                    //虚拟机临时缓冲区，有空闲虚拟机，则运行
                    self.async_run(vm, src, port, args, info);
                } else {
                    //虚拟机临时缓冲区，没有空闲虚拟机
                    if is_alloced_limit() {
                        //当前进程内存已达到最大堆限制，则拒绝任务立即执行，并将任务加入当前虚拟机的任务调度队列中，记录当前拒绝的次数
                        self.queue_sent.send((src, port, args, info));
                        self.refuse_count.fetch_add(1, Ordering::Relaxed);
                    } else {
                        //当前进程内存未达到最大堆限制，则立即构建新的虚拟机
                        match self.new_vm(self.auth.clone()) {
                            None => {
                                self.scheduling_count.fetch_add(1, Ordering::Relaxed); //增加虚拟机工厂调用次数
                                panic!("Vm Factory Call Error, new vm failed, factory: {:?}",
                                       (&self.name).to_string());
                            },
                            Some(vm) => {
                                //构建完成，则运行
                                self.async_run(vm, src, port, args, info);
                            },
                        }
                    }
                }
            }
        }

        self.scheduling_count.fetch_add(1, Ordering::Relaxed); //增加虚拟机工厂调度次数
    }

    //整理虚拟机工厂的虚拟机池
    pub fn collect(&self, handler: Arc<Fn(&mut Arc<JS>) -> CollectResult>) {
        self.pool.collect_from_bottom(handler); //从栈底开始整理
    }

    //清空整理时被移除的栈帧
    pub fn clear_collected(&self) {
        self.pool.clear();
    }

    //构建一个虚拟机，加载所有字节码，并提供虚拟机本地对象授权，不会检查是否达到虚拟机工厂限制容量上限
    fn new_vm(&self, auth: Arc<NativeObjsAuth>) -> Option<Arc<JS>> {
        let start = VM_NEW_TIME.start();

        let mut curr_size = self.size();
        loop {
            match self.size.compare_and_swap(curr_size, curr_size + 1, Ordering::SeqCst) {
                new_curr_size if new_curr_size == curr_size => {
                    //原子增加当前虚拟机数量成功，则继续构建虚拟机
                    break;
                },
                new_curr_size  => {
                    //原子增加当前虚拟机数量失败，则从新的当前虚拟机数量开始重试
                    curr_size = new_curr_size;
                    pause();
                }
            }
        }

        let result = if !self.is_reused {
            //构建一个无法复用的虚拟机
            JS::new(self.alloc_id.fetch_add(1, Ordering::Relaxed), self.name.clone(), auth.clone(), None)
        } else {
            //构建一个可以复用的虚拟机
            JS::new(self.alloc_id.fetch_add(1, Ordering::Relaxed), self.name.clone(), auth.clone(), Some((Arc::new(AtomicBool::new(false)), Arc::new(self.clone()))))
        };

        match result {
            None => None,
            Some(vm) => {
                VM_NEW_TIME.timing(start);
                let start = VM_LOAD_TIME.start();

                //为当前虚拟机加载当前虚拟机工厂绑定的所有字节码
                for code in self.codes.iter() {
                    if vm.load(code.as_slice()) {
                        while !vm.is_ran() {
                            pause();
                        }
                        continue;
                    }
                    return None;
                }

                //如果是可以复用的虚拟机，则需要创建全局对象模板，并替换当前全局对象
                if self.is_reused {
                    if !vm.new_global_template() {
                        warn!("!!!> Vm Factory Create Vm Error, new global template failed, factory: {:?}",
                                 (&self.name).to_string());
                        return None;
                    }

                    if !vm.alloc_global() {
                        warn!("!!!> Vm Factory Create Vm Error, alloc global failed, factory: {:?}",
                                 (&self.name).to_string());
                        return None;
                    }

                    vm.unlock_collection(); //解锁回收器，必须在虚拟机初始化、加载代码、运行代码等操作后解锁
                }

                vm.update_last_heap_size(); //更新初始化后虚拟机的堆大小和内存占用

                info!("===> Vm Factory Create Vm Ok, factory: {:?}, vm: {:?}",
                         (&self.name).to_string(), vm);

                VM_LOAD_TIME.timing(start);
                VM_COUNT.sum(1);

                Some(vm)
            }
        }
    }

    //异步运行指定虚拟机
    fn async_run(&self, vm: Arc<JS>, src: Option<usize>, port: Atom, args: Box<FnOnce(Arc<JS>) -> usize>, info: Atom) {
        let vm_copy = vm.clone();
        let func = Box::new(move |lock: Option<isize>| {
            if let Some(queue) = lock {
                //为虚拟机设置当前任务的队列，将会重置可复用虚拟机的当前任务队列
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

        VM_CALL_COUNT.sum(1);
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
pub fn block_set_global_var(js: Arc<JS>, name: String, var: Box<FnOnce(Arc<JS>) -> Result<JSType, String>>, next: Box<FnOnce(Result<Arc<JS>, BlockError>)>, info: Atom) {
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
        warn!("!!!> Block Set Global Var Error, unlock js task queue failed");
    }
}

/*
* 线程安全的回应阻塞调用
* 返回值构建函数执行完成后，当前值栈必须存在且只允许存在一个值
*/
pub fn block_reply(js: Arc<JS>, result: Box<FnOnce(Arc<JS>)>, info: Atom) {
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
        warn!("!!!> Block Reply Error, unlock js task queue failed");
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
                    let reason_ptr = CString::into_raw(CString::new(reason).unwrap());
                    dukc_wakeup(copy_js.get_vm(), 1);
                    dukc_new_error(copy_js.get_vm(), reason_ptr as *const c_char);
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                    CString::from_raw(reason_ptr);
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
        warn!("!!!> Block Throw Error, unlock js task queue failed");
    }
}

/*
* 线程安全的向虚拟机推送异步回调函数，延迟任务必须返回任务句柄，其它任务根据是否是动态任务确定是否返回任务句柄
*/
pub fn push_callback(js: Arc<JS>, callback: u32, args: Box<FnOnce(Arc<JS>) -> usize>, timeout: Option<u32>, info: Atom) -> Option<isize> {
    VM_PUSH_CALLBACK_COUNT.sum(1);

    if timeout.is_some() {
        //推送延迟异步任务，禁止直接执行异步任务
        JS::callback(js.clone(), TaskType::Sync(true), callback, args, timeout, info)
    } else {
        //推送异步任务，禁止直接执行异步任务
        JS::callback(js.clone(), TaskType::Sync(true), callback, args, timeout, info)
    }
}

/*
* 线程安全的向虚拟机推送异步消息，正数表示使用指定的回调执行消息，负数表示移除指定的回调
*/
pub fn push_msg(js: Arc<JS>, callback: u32, args: Box<FnOnce(Arc<JS>) -> usize>, info: Atom) -> Option<isize> {
    JS::push(js.clone(), TaskType::Sync(true), callback, args, info) //禁止直接执行异步消息
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
    VM_ASYNC_REQUEST_COUNT.sum(1);

    let ref lock = &**VM_CHANNELS;
    let channels = lock.read().unwrap();
    (*channels).request(js, name, msg, native_objs, callback)
}
