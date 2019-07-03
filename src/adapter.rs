use libc::{c_void as c_void_ptr, c_char, int8_t, uint8_t, c_int, uint32_t, int32_t, uint64_t, size_t, c_double, memcpy};
use std::slice::{from_raw_parts_mut, from_raw_parts};
use std::sync::atomic::{Ordering, AtomicUsize, AtomicIsize, AtomicBool};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::string::FromUtf8Error;
use std::ffi::{CStr, CString};
use std::collections::HashMap;
use std::mem::transmute;
use std::time::{Duration, SystemTime, Instant};
use std::cell::RefCell;
use std::boxed::FnBox;
use std::sync::{Arc, RwLock};
use std::ops::Drop;
use std::thread;

#[cfg(not(unix))]
use kernel32;

use rand::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

use crossbeam_queue::{PopError, SegQueue};

use worker::task::TaskType;
use worker::impls::{create_js_task_queue, lock_js_task_queue, unlock_js_task_queue, cast_js_task};
use apm::{allocator::{VM_ALLOCATED, get_max_alloced_limit, is_alloced_limit, vm_alloced_size, all_alloced_size}, counter::{GLOBAL_PREF_COLLECT, PrefCounter, PrefTimer}};
use timer::{TIMER, FuncRuner};
use atom::Atom;
use lfstack::LFStack;

use native_object_impl::*;
use bonmgr::{NativeObjs, NObject, NativeObjsAuth};
use pi_vm_impl::VMFactory;

/*
* 虚拟机消息队列默认优先级
*/
const JS_ASYNC_MSG_QUEUE_PRIORITY: usize = 1000;

/*
* 虚拟机线程全局变量名
*/
const JS_THREAD_GLOBAL_VAR_NAME: &'static str = "__curr_block_thread";

lazy_static! {
    //虚拟机超时时长，单位us, 默认5分钟
    static ref VM_TIMEOUT: AtomicUsize = AtomicUsize::new(300000000);
    //虚拟机工厂注册表
    pub static ref VM_FACTORY_REGISTERS: Arc<RwLock<HashMap<String, VMFactory>>> = Arc::new(RwLock::new(HashMap::new()));
}

lazy_static! {
    //虚拟机初始化异常数量
    static ref VM_INIT_PANIC_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_init_panic_count"), 0).unwrap();
    //虚拟机运行异常数量
    static ref VM_RUN_PANIC_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_run_panic_count"), 0).unwrap();
    //虚拟机等待同步阻塞调用的数量
    static ref VM_WAIT_BLOCK_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_wait_block_count"), 0).unwrap();
    //虚拟机完成同步任务、异步任务或异步回调的数量
    static ref VM_FINISH_TASK_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_finish_task_count"), 0).unwrap();
    //虚拟机弹出异步回调的数量
    static ref VM_POP_CALLBACK_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_pop_callback_count"), 0).unwrap();
}

#[link(name = "dukc")]
extern "C" {
    fn dukc_register_native_object_function_call(func: extern fn(*const c_void_ptr, uint32_t, uint32_t, *const c_void_ptr, *const c_void_ptr) -> c_int);
    fn dukc_register_native_object_free(func: extern fn(*const c_void_ptr, uint32_t));
    fn dukc_heap_create() -> *const c_void_ptr;
    fn dukc_heap_init(vm: *const c_void_ptr, reply: extern fn(*const c_void_ptr, c_int, *const c_char)) -> uint32_t;
    fn dukc_init_char_output(vm: *const c_void_ptr, func: extern fn(*const c_char));
    // fn dukc_vm_create(heap: *const c_void_ptr) -> *const c_void_ptr;
    fn dukc_vm_size(vm: *const c_void_ptr) -> size_t;
    fn dukc_compile_script(vm: *const c_void_ptr, file: *const c_char, code: *const c_char, size: *mut uint32_t, reply: extern fn(*const c_void_ptr, c_int, *const c_char)) -> *const c_void_ptr;
    fn dukc_load_code(vm: *const c_void_ptr, size: uint32_t, bytes: *const c_void_ptr, reply: extern fn(*const c_void_ptr, c_int, *const c_char)) -> uint32_t;
    fn dukc_bind_vm(vm: *const c_void_ptr, handler: *const c_void_ptr);
    // fn dukc_vm_clone(size: uint32_t, bytes: *const c_void_ptr, reply: extern fn(*const c_void_ptr, c_int, *const c_char)) -> *const c_void_ptr;
    fn dukc_vm_run(vm: *const c_void_ptr, reply: extern fn(*const c_void_ptr, c_int, *const c_char));
    fn dukc_vm_global_template(vm: *const c_void_ptr) -> uint32_t;
    fn dukc_vm_global_swap(vm: *const c_void_ptr) -> uint32_t;
    fn dukc_vm_global_clear(vm: *const c_void_ptr) -> uint32_t;
    fn dukc_vm_global_free(vm: *const c_void_ptr) -> uint32_t;
    pub fn dukc_vm_status_check(vm: *const c_void_ptr, value: int8_t) -> uint8_t;
    pub fn dukc_vm_status_switch(vm: *const c_void_ptr, old_status: int8_t, new_status: int8_t) -> int8_t;
    pub fn dukc_vm_status_sub(vm: *const c_void_ptr, value: int8_t) -> int8_t;
    fn dukc_new_null(vm: *const c_void_ptr) -> uint32_t;
    fn dukc_new_undefined(vm: *const c_void_ptr) -> uint32_t;
    fn dukc_new_boolean(vm: *const c_void_ptr, b: uint8_t) -> uint32_t;
    fn dukc_new_number(vm: *const c_void_ptr, num: c_double) -> uint32_t;
    fn dukc_new_string(vm: *const c_void_ptr, str: *const c_char) -> uint32_t;
    fn dukc_new_object(vm: *const c_void_ptr) -> uint32_t;
    fn dukc_get_type(vm: *const c_void_ptr, name: *const c_char) -> uint32_t;
    fn dukc_new_type(vm: *const c_void_ptr, len: uint8_t) -> int32_t;
    fn dukc_set_object_field(vm: *const c_void_ptr, object: uint32_t, key: *const c_char, value: uint32_t) -> uint32_t;
    fn dukc_new_array(vm: *const c_void_ptr) -> uint32_t;
    fn dukc_set_array_index(vm: *const c_void_ptr, array: uint32_t, index: uint32_t, value: uint32_t) -> uint32_t;
    fn dukc_new_array_buffer(vm: *const c_void_ptr, length: uint32_t) -> uint32_t;
    fn dukc_new_uint8_array(vm: *const c_void_ptr, length: uint32_t) -> uint32_t;
    fn dukc_new_native_object(vm: *const c_void_ptr, ptr: uint64_t) -> uint32_t;
    pub fn dukc_new_error(vm: *const c_void_ptr, reason: *const c_char) -> uint32_t;
    pub fn dukc_remove_value(vm: *const c_void_ptr, value: uint32_t);
    fn dukc_get_value_type(vm: *const c_void_ptr, value: uint32_t) -> uint8_t;
    fn dukc_get_boolean(vm: *const c_void_ptr, value: uint32_t) -> uint8_t;
    fn dukc_get_number(vm: *const c_void_ptr, value: uint32_t) -> c_double;
    fn dukc_get_string(vm: *const c_void_ptr, value: uint32_t) -> *const c_char;
    fn dukc_get_object_field(vm: *const c_void_ptr, object: uint32_t, key: *const c_char) -> uint32_t;
    fn dukc_get_array_length(vm: *const c_void_ptr, array: uint32_t) -> uint32_t;
    fn dukc_get_array_index(vm: *const c_void_ptr, array: uint32_t, index: uint32_t) -> uint32_t;
    fn dukc_get_buffer_length(vm: *const c_void_ptr, value: uint32_t) -> uint32_t;
    fn dukc_get_buffer(vm: *const c_void_ptr, value: uint32_t) -> *const c_void_ptr;
    fn dukc_get_native_object_instance(vm: *const c_void_ptr, value: uint32_t) -> uint64_t;
    fn dukc_get_js_function(vm: *const c_void_ptr, func: *const c_char) -> uint32_t;
    pub fn dukc_link_js_function(vm: *const c_void_ptr, func: *const c_char) -> uint32_t;
    fn dukc_check_js_function(vm: *const c_void_ptr, func: *const c_char) -> uint32_t;
    pub fn dukc_get_callback(vm: *const c_void_ptr, index: uint32_t) -> uint32_t ;
    pub fn dukc_call(vm: *const c_void_ptr, len: uint8_t, reply: extern fn(*const c_void_ptr, c_int, *const c_char));
    pub fn dukc_throw(vm: *const c_void_ptr, reason: *const c_char);
    pub fn dukc_wakeup(vm: *const c_void_ptr, error: c_int) -> uint32_t;
    pub fn dukc_continue(vm: *const c_void_ptr, reply: extern fn(*const c_void_ptr, c_int, *const c_char));
    pub fn dukc_switch_context(vm: *const c_void_ptr);
    pub fn dukc_callback_count(vm: *const c_void_ptr) -> uint32_t;
    pub fn dukc_remove_callback(vm: *const c_void_ptr, index: uint32_t) -> uint32_t;
    fn dukc_set_global_var(vm: *const c_void_ptr, key: *const c_char) -> uint32_t;
    fn dukc_invoke(vm: *const c_void_ptr, len: uint8_t) -> int32_t;
    fn dukc_eval(vm: *const c_void_ptr, script: *const c_char) -> int32_t;
    pub fn dukc_top(vm: *const c_void_ptr) -> int32_t;
    pub fn dukc_to_string(vm: *const c_void_ptr, offset: int32_t) -> *const c_char;
    fn dukc_dump_stack(vm: *const c_void_ptr) -> *const c_char;
    fn dukc_stack_frame(vm: *const c_void_ptr, index: uint32_t) -> *const c_char;
    pub fn dukc_pop(vm: *const c_void_ptr);
    fn dukc_vm_destroy(vm: *const c_void_ptr);
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

/*
* js返回回调函数
*
* 当前异步任务如果没有投递其它异步任务，则当前异步任务成为同步任务，并在当前异步任务完成后执行消息队列中的回调函数，如果没有异步消息，则回收虚拟机
*  否则还有其它异步任务，则回收权利交由其它异步任务
*/
#[no_mangle]
pub extern "C" fn js_reply_callback(handler: *const c_void_ptr, status: c_int, err: *const c_char) {
    if handler.is_null() {
        //处理初始化异常
        if status != 0 {
            VM_INIT_PANIC_COUNT.sum(1);

            println!("===> JS Init Error, status: {}, err: {}",
                     status, unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() });
        }
        return;
    }

    let js: Arc<JS>;
    let vm: *const c_void_ptr;
    unsafe {
        js = JS::from_raw(handler);
        vm = js.get_vm();

        //处理执行异常
        if status != 0 {
            //有异常，则重置虚拟机线程全局变量，保证虚拟机可以继续运行
            let null = js.new_null();
            js.set_global_var(JS_THREAD_GLOBAL_VAR_NAME.to_string(), null);

            VM_RUN_PANIC_COUNT.sum(1);

            println!("===> JS Run Error, vm: {:?}, err: {}", js,
                     CStr::from_ptr(err).to_string_lossy().into_owned());
        }

        js.update_last_heap_size(); //在js当前任务执行完成后，更新虚拟机堆大小和内存占用
        js.queue.size.fetch_sub(1, Ordering::SeqCst); //减少消息队列长度
        if dukc_vm_status_check(vm, JSStatus::WaitBlock as i8) > 0 {
            //当前虚拟机任务已执行完成且当前虚拟机状态是等待状态，则需要改变状态，保证虚拟机异步任务被执行
            dukc_vm_status_sub(vm, 1);

            VM_WAIT_BLOCK_COUNT.sum(1);
        } else if dukc_vm_status_check(vm, JSStatus::SingleTask as i8) > 0 {
            //当前虚拟机同步任务、异步任务或异步回调已执行完成，且当前虚拟机状态是同步状态，则处理消息队列
            if js.ret.borrow().is_some() {
                *js.ret.borrow_mut() = js.stack_top_string(); //返回值缓存不为空，则将当前执行结果更新返回值缓存
            }
            dukc_pop(vm); //移除上次同步任务、异步任务或回调函数的执行结果
            handle_async_callback(js.clone(), vm);

            VM_FINISH_TASK_COUNT.sum(1);
        }
    }
    Arc::into_raw(js);
}

/*
* 处理异步回调，只有虚拟机当前同步任务、异步任务或异步回调已执行完成，才允许开始处理其它异步回调，特别的如果正在处理异步任务时，调用任何关于异步回调的非安全函数，都会导致异常
*/
pub unsafe fn handle_async_callback(js: Arc<JS>, vm: *const c_void_ptr) {
    //检查消息队列是否为空，如果不为空则继续执行异步任务或异步回调任务
    let mut is_collect = false;
    if js.queue.size.load(Ordering::SeqCst) == 0 {
        //消息队列为空
        if dukc_callback_count(vm) == 0 && dukc_vm_status_check(vm, JSStatus::SingleTask as i8) > 0 {
            //没有已注册的异步回调函数且当前异步任务已完成，则需要将执行结果弹出值栈并改变状态, 保证虚拟机回收
            dukc_vm_status_sub(vm, 1);
            is_collect = true;
        } else {
            //有已注册的异步回调函数，则需要等待消息异步推送到消息队列，保证虚拟机异步回调函数被执行
            dukc_vm_status_switch(vm, JSStatus::SingleTask as i8, JSStatus::WaitCallBack as i8);
        }
    } else if dukc_callback_count(vm) > 0 {
        //消息队列不为空、有已注册的异步回调函数、且消息队列被锁，则释放锁，以保证开始执行消息队列中的异步任务或异步回调任务
        let queue = js.get_queue();
        if !unlock_js_task_queue(queue) {
            println!("!!!> Handle Callback Error, unlock js task queue failed, queue: {:?}", queue);
        }

        VM_POP_CALLBACK_COUNT.sum(1);
    } else {
        //消息队列不为空，且未注册异步回调函数，表示同步任务或异步任务执行完成且没有异步回调任务，
        //则需要将执行结果弹出值栈并改变状态, 保证当前虚拟机回收
        dukc_vm_status_sub(vm, 1);
        is_collect = true;
    }

    if js.exist_tasks() {
        //解锁当前虚拟机锁住的同步任务队列, 保证当前虚拟机回收或其它虚拟机执行下一个任务
        let tasks = js.get_tasks();
        if !unlock_js_task_queue(tasks) {
            println!("!!!> Handle Callback Error, unlock js task queue failed, tasks: {:?}", tasks);
        }
    }

    //更新当前虚拟机最近运行时间
    js.update_last_time();

    if is_collect {
        //当前虚拟机可以整理
        collect_vm(js);
    }
}

//获取系统UTC，单位us
pub fn now_utc() -> usize {
    if let Ok(d) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        d.as_micros() as usize
    } else {
        0
    }
}

//整理虚拟机，处理虚拟机丢弃和复用
fn collect_vm(js: Arc<JS>) {
    if let Some((lock, factory)) = js.collection.clone() {
        if lock.load(Ordering::SeqCst) {
            //回收器已解锁，则检查是否需要复用
            match js.check_reuse() {
                0 => {
                    //需要丢弃当前虚拟机
                    factory.throw(1);
                    println!("===> Vm Throw Ok, vm: {:?}", js);
                    return;
                },
                state => {
                    //需要继续整理当前虚拟机，并复用
                    let copy = js.clone();
                    if js.clear_global() {
                        //清理成功，则重置当前虚拟机的全局环境
                        if js.alloc_global() {
                            //虚拟机已重置全局环境
                            if state == 1 {
                                //需要释放当前虚拟机可回收内存
                                if js.free_global() {
                                    println!("===> Vm Free Ok, vm: {:?}", js);
                                } else {
                                    println!("!!!> Vm Collection Error, vm: {:?}, e: free global failed", js);
                                }
                            }

                            js.queue.size.store(0, Ordering::Relaxed); //重置虚拟机当前消息队列
                            js.reused_count.fetch_add(1, Ordering::Relaxed); //增加虚拟机复用次数
                            factory.reuse(js); //复用当前虚拟机
                        } else {
                            println!("!!!> Vm Collection Error, vm: {:?}, e: alloc global failed", copy);
                        }
                    } else {
                        //复用预处理失败，则立即丢弃当前虚拟机
                        println!("!!!> Vm Collection Error, vm: {:?}, e: clear global failed", copy);
                    }
                }
            }
        }
    }
}

/*
* 初始化注入NativeObject关联函数
*/
pub fn register_native_object() {
    unsafe {
        dukc_register_native_object_function_call(native_object_function_call);
        dukc_register_native_object_free(native_object_function_free);
    }
}

/*
* 执行njsc测试代码
*/
pub fn dukc_test_main() {
    // unsafe { test_main(); }
}

/*
* 显示加载动态库
*/
#[cfg(not(unix))]
pub fn load_lib_backtrace() {
    unsafe { kernel32::LoadLibraryA(CString::new("backtrace").unwrap().as_ptr()); }
}

/*
* js状态
*/
pub enum JSStatus {
    Destroy = -1,
    NoTask,
    SingleTask,
    MultiTask,
    WaitBlock,
    WaitCallBack,
}

/*
* js消息队列
*/
#[derive(Clone)]
struct JSMsgQueue {
    id:     Arc<AtomicIsize>,   //虚拟机消息队列
    size:   Arc<AtomicUsize>,   //虚拟机消息队列长度
}

/*
* js运行环境
*/
#[derive(Clone)]
pub struct JS {
    vm:                 usize,                                      //虚拟机
    tasks:              Arc<AtomicIsize>,                           //虚拟机任务队列
    queue:              JSMsgQueue,                                 //虚拟机消息队列
    auth:               Arc<NativeObjsAuth>,                        //虚拟机本地对象授权
    objs:               NativeObjs,                                 //虚拟机本地对象表
    objs_ref:           Arc<RefCell<HashMap<usize, NObject>>>,      //虚拟机本地对象引用表
    ret:                Arc<RefCell<Option<String>>>,               //虚拟机执行栈返回结果缓存
    id:                 usize,                                      //虚拟机id
    name:               Atom,                                       //虚拟机名
    reused_count:       Arc<AtomicUsize>,                           //虚拟机当前复用次数
    last_heap_size:     Arc<AtomicIsize>,                           //虚拟机最近堆大小
    rng:                Arc<RefCell<SmallRng>>,                     //虚拟机随机数生成器
    collection:         Option<(Arc<AtomicBool>, Arc<VMFactory>)>,  //虚拟机回收器
    last_time:          Arc<AtomicUsize>,                           //虚拟机最近运行时间
}

/*
* 尝试destroy虚拟机
*/
pub unsafe fn try_js_destroy(js: &JS) {
    if js.vm == 0 {
        return;
    }

    let old_status = dukc_vm_status_switch(js.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::Destroy as i8);
    if old_status == JSStatus::NoTask as i8 {
        //当前js虚拟机无任务，则可以destroy
        println!("===> Vm Destroy Ok, vm: {:?}", js);
        VM_ALLOCATED.fetch_sub(js.last_heap_size.load(Ordering::Relaxed), Ordering::Relaxed); //减少虚拟机占用内存
        dukc_vm_destroy(js.vm as *const c_void_ptr);
    }
}

impl Drop for JS {
    fn drop(&mut self) {
        unsafe { try_js_destroy(self); }
    }
}

impl Debug for JS {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "JS[id = {}, name = {:?}, vm = {}, tasks = {}, queue = {}, finish = {}, count = {}, last size = {}, current size = {}]",
               self.id, (&self.name).to_string(), self.vm,
               self.get_tasks(), self.get_queue_len(), self.is_ran(),
               self.reused_count.load(Ordering::Relaxed),
               self.last_heap_size.load(Ordering::Relaxed),
               self.heap_size())
    }
}

impl JS {
    //构建一个虚拟机
    pub fn new(vm_id: usize,
               name: Atom,
               auth: Arc<NativeObjsAuth>,
               collection: Option<(Arc<AtomicBool>, Arc<VMFactory>)>) -> Option<Arc<Self>> {
        let ptr: *const c_void_ptr;
        unsafe { ptr = dukc_heap_create() }
        if ptr.is_null() {
            None
        } else {
            unsafe {
                if dukc_heap_init(ptr, js_reply_callback) == 0 {
                    dukc_vm_destroy(ptr);
                    return None;
                }
                dukc_vm_run(ptr, js_reply_callback);
                dukc_pop(ptr); //在初始化时需要弹出执行的结果
            }
            let id = create_js_task_queue(JS_ASYNC_MSG_QUEUE_PRIORITY, true); //为指定虚拟机创建对应的消息队列
            //初始化时锁住虚拟机消息队列
            if !lock_js_task_queue(id) {
                panic!("!!!> New Vm Error, lock async callback queue failed");
            }
            let arc = Arc::new(JS {
                vm: ptr as usize,
                tasks: Arc::new(AtomicIsize::new(0)),
                queue: JSMsgQueue {
                    id: Arc::new(AtomicIsize::new(id)),
                    size: Arc::new(AtomicUsize::new(0)),
                },
                auth: auth.clone(),
                objs: NativeObjs::new(),
                objs_ref: Arc::new(RefCell::new(HashMap::new())),
                ret: Arc::new(RefCell::new(None)),
                id: vm_id,
                name,
                reused_count: Arc::new(AtomicUsize::new(0)),
                last_heap_size: Arc::new(AtomicIsize::new(0)),
                rng: Arc::new(RefCell::new(SmallRng::from_entropy())),
                collection,
                last_time: Arc::new(AtomicUsize::new(now_utc())),
            });
            unsafe {
                let handler = Arc::into_raw(arc.clone()) as *const c_void_ptr;
                dukc_bind_vm(ptr, handler);
                Arc::from_raw(handler); //保证被clone的js的释放
            }
            Some(arc)
        }
    }

    //从指针构建指定虚拟机
    pub unsafe fn from_raw(ptr: *const c_void_ptr) -> Arc<Self> {
        Arc::from_raw(ptr as *const JS)
    }

    //向指定虚拟机的消息队列中推送消息
    pub fn push(js: Arc<JS>, task_type: TaskType, callback: u32, args: Box<FnBox(Arc<JS>) -> usize>, info: Atom) -> usize {
        let js_copy = js.clone();
        let func = Box::new(move |_lock| {
            let vm: *const c_void_ptr;
            //不需要改变虚拟机状态，以保证当前虚拟机可以线程安全的执行回调函数
            unsafe {
                vm = js_copy.get_vm();
                if dukc_get_callback(vm, callback) == 0 {
                    //当前回调函数不存在，则立即退出当前同步任务，以获取下一个异步消息
                    return;
                }
                dukc_remove_callback(vm, callback); //移除虚拟机注册的指定回调函数
            }

            //将回调函数的参数压栈，并执行回调函数
            let args_len = (args)(js_copy.clone());
            unsafe { dukc_call(vm, args_len as u8, js_reply_callback); }
        });
        let size = js.queue.size.fetch_add(1, Ordering::SeqCst) + 1; //增加消息队列长度，并返回
        cast_js_task(task_type, 0, Some(js.get_queue()), func, info); //向指定虚拟机的消息队列推送异步回调任务
        size
    }

    //获取内部虚拟机
    pub unsafe fn get_vm(&self) -> *const c_void_ptr {
        self.vm as *const c_void_ptr
    }

    //获取虚拟机堆大小
    pub fn heap_size(&self) -> usize {
        unsafe { dukc_vm_size(self.vm as *const c_void_ptr) }
    }

    //获取虚拟机复用次数
    pub fn reused_count(&self) -> usize {
        self.reused_count.load(Ordering::Relaxed)
    }

    //更新虚拟机上次堆大小，并更新所有虚拟机占用内存大小
    pub fn update_last_heap_size(&self) {
        let cur_size = self.heap_size() as isize;
        let last_size = self.last_heap_size.swap(cur_size, Ordering::Relaxed);

        if last_size > cur_size {
            //当前虚拟机堆变小了，从所有虚拟机占用内存中减去内存减量
            VM_ALLOCATED.fetch_sub(last_size - cur_size, Ordering::Relaxed);
        } if last_size < cur_size {
            //当前虚拟机堆变大了，在所有虚拟机占用内存中增加内存增量
            VM_ALLOCATED.fetch_add(cur_size - last_size, Ordering::Relaxed);
        }
    }

    //获取虚拟机上次运行时间
    pub fn last_time(&self) -> usize {
        self.last_time.load(Ordering::Relaxed)
    }

    //设置虚拟机上次运行时间，单位ms，返回上次运行时间
    pub fn set_last_time(&self, time: usize) -> usize {
        self.last_time.swap(time * 1000, Ordering::SeqCst)
    }

    //更新虚拟机上次运行时间，并返回上次与本次运行间隔时长，注意不是本次运行时长
    pub fn update_last_time(&self) -> isize {
        let now = now_utc();
        let last = self.last_time.swap(now, Ordering::SeqCst);
        now as isize - last as isize
    }

    //初始化虚拟机字符输出
    pub fn init_char_output(&self, output: extern fn(*const c_char)) {
        unsafe {
            dukc_init_char_output(self.vm as *const c_void_ptr, output);
        }
    }

    //解锁虚拟机回收器
    pub fn unlock_collection(&self) {
        if let Some((lock, _)) = &self.collection {
            //有回收器，则解锁
            lock.swap(true, Ordering::SeqCst);
        }
    }

    //为当前虚拟机创建全局环境模板，如果已存在，则忽略
    pub fn new_global_template(&self) -> bool {
        unsafe {
            let status = dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机状态错误，无法创建
                false
            } else {
                let result = dukc_vm_global_template(self.vm as *const c_void_ptr) != 0;
                dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::SingleTask as i8, JSStatus::NoTask as i8);
                result
            }
        }
    }

    //为当前虚拟机分配新的全局环境
    pub fn alloc_global(&self) -> bool {
        unsafe {
            let status = dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机状态错误，无法替换
                false
            } else {
                let result = dukc_vm_global_swap(self.vm as *const c_void_ptr) != 0;
                dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::SingleTask as i8, JSStatus::NoTask as i8);
                result
            }
        }
    }

    //为当前虚拟机清理全局环境
    pub fn clear_global(&self) -> bool {
        unsafe {
            let status = dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机状态错误，无法清理
                false
            } else {
                let result = dukc_vm_global_clear(self.vm as *const c_void_ptr) != 0;
                dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::SingleTask as i8, JSStatus::NoTask as i8);
                result
            }
        }
    }

    //执行当前虚拟机gc
    pub fn free_global(&self) -> bool {
        unsafe {
            let status = dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机状态错误，无法清理
                false
            } else {
                let result = dukc_vm_global_free(self.vm as *const c_void_ptr) != 0;
                dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::SingleTask as i8, JSStatus::NoTask as i8);
                result
            }
        }
    }

    //判断虚拟机是否绑定了同步任务队列
    pub fn exist_tasks(&self) -> bool {
        self.tasks.load(Ordering::SeqCst) != 0
    }

    //获取虚拟机同步任务队列
    pub fn get_tasks(&self) -> isize {
        self.tasks.load(Ordering::SeqCst)
    }

    //设置虚拟机同步任务队列
    pub fn set_tasks(&self, tasks: isize) {
        self.tasks.swap(tasks, Ordering::SeqCst);
    }

    //获取虚拟机消息队列
    pub fn get_queue(&self) -> isize {
        self.queue.id.load(Ordering::SeqCst)
    }

    //获取虚拟机消息队列长度
    pub fn get_queue_len(&self) -> usize {
        self.queue.size.load(Ordering::SeqCst)
    }

    //增加虚拟机消息队列长度
    pub fn add_queue_len(&self) -> usize {
        self.queue.size.fetch_add(1, Ordering::SeqCst)
    }

    //减少虚拟机消息队列长度
    pub fn deduct_queue_len(&self) -> usize {
        self.queue.size.fetch_sub(1, Ordering::SeqCst)
    }

    //获取指定虚拟机的本地对象授权
    pub fn get_auth(&self) -> Arc<NativeObjsAuth> {
        self.auth.clone()
    }

    //获取虚拟机本地对象表
    pub fn get_objs(&self) -> Arc<RefCell<HashMap<usize, NObject>>> {
        self.objs.0.clone()
    }

    //获取虚拟机本地对象引用表
    pub fn get_objs_ref(&self) -> Arc<RefCell<HashMap<usize, NObject>>> {
        self.objs_ref.clone()
    }

    //获取虚拟机最近执行结果
    pub fn get_ret(&self) -> Option<String> {
        if self.ret.borrow().is_none() {
            return None;
        }

        Some(self.ret.borrow().as_ref().unwrap().clone())
    }

    //设置虚拟机最近执行结果
    pub fn set_ret(&self, ret: Option<String>) -> bool {
        if !self.is_ran() {
            //运行中，则忽略
            return false;
        }

        *self.ret.borrow_mut() = ret;
        true
    }

    //判断js虚拟机是否完成运行
    pub fn is_ran(&self) -> bool {
        unsafe { dukc_vm_status_check(self.vm as *const c_void_ptr, JSStatus::NoTask as i8) > 0 }
    }

    //编译指定脚本
    pub fn compile(&self, file: String, script: String) -> Option<Vec<u8>> {
        let mut len = 0u32;
        let size: *mut u32 = &mut len;
        unsafe {
            let status = dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机正在destroy或有其它任务
                None
            } else {
                self.add_queue_len(); //增加当前虚拟机消息队列长度
                let bytes = dukc_compile_script(self.vm as *const c_void_ptr, CString::new(file).unwrap().as_ptr(), CString::new(script).unwrap().as_ptr(), size, js_reply_callback);
                if bytes.is_null() {
                    return None;
                }
                Some(from_raw_parts(bytes as *mut u8, len as usize).to_vec())
            }
        }
    }

    //加载指定代码
    pub fn load(&self, codes: &[u8]) -> bool {
        let size = codes.len() as u32;
        let bytes = codes.as_ptr() as *const c_void_ptr;
        unsafe {
            let status = dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
             if status == JSStatus::SingleTask as i8 {
                //当前虚拟机正在destroy或有其它任务
                false
            } else {
                //加载失败才会回调，所以无需增加当前虚拟机消息队列长度
                if dukc_load_code(self.vm as *const c_void_ptr, size, bytes, js_reply_callback) == 0 {
                    return false;
                }
                self.add_queue_len(); //增加当前虚拟机消息队列长度
                dukc_vm_run(self.vm as *const c_void_ptr, js_reply_callback);
                true
            }
        }
    }

    //运行js虚拟机
    pub fn run(&self) {
        unsafe {
            let status = dukc_vm_status_switch(self.vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机状态错误，无法运行
                println!("invalid vm status with run");
            } else {
                //增加当前虚拟机消息队列长度，并开始执行运行
                self.add_queue_len();
                dukc_vm_run(self.vm as *const c_void_ptr, js_reply_callback);
            }
        }
    }

    //构建undefined
    pub fn new_undefined(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_undefined(self.vm as *const c_void_ptr) }
        JSType {
            type_id: JSValueType::Undefined as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建null
    pub fn new_null(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_null(self.vm as *const c_void_ptr) }
        JSType {
            type_id: JSValueType::Null as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建boolean
    pub fn new_boolean(&self, b: bool) -> JSType {
        let ptr: u32;
        unsafe {
            if b {
                ptr = dukc_new_boolean(self.vm as *const c_void_ptr, 1u8);
            } else {
                ptr = dukc_new_boolean(self.vm as *const c_void_ptr, 0u8);
            }
        }
        JSType {
            type_id: JSValueType::Boolean as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i8
    pub fn new_i8(&self, num: i8) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i16
    pub fn new_i16(&self, num: i16) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i32
    pub fn new_i32(&self, num: i32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i64
    pub fn new_i64(&self, num: i64) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u8
    pub fn new_u8(&self, num: u8) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u16
    pub fn new_u16(&self, num: u16) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u32
    pub fn new_u32(&self, num: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u64
    pub fn new_u64(&self, num: u64) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建f32
    pub fn new_f32(&self, num: f32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建f64
    pub fn new_f64(&self, num: f64) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void_ptr, num) }
        JSType {
            type_id: JSValueType::Number as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建字符串，注意rust的字符串默认是UTF8编码，而JS是UTF16编码
    pub fn new_str(&self, str: String) -> Result<JSType, String> {
        match CString::new(str) {
            Err(e) => {
                Err(e.to_string())
            },
            Ok(cstring) => {
                let ptr: u32;
                unsafe { ptr = dukc_new_string(self.vm as *const c_void_ptr, cstring.as_ptr()) }
                Ok(JSType {
                    type_id: JSValueType::String as u8,
                    is_drop: false,
                    vm: self.vm,
                    value: ptr as usize,
                })
            }
        }
    }

    //构建对象
    pub fn new_object(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_object(self.vm as *const c_void_ptr) }
        JSType {
            type_id: JSValueType::Object as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取指定类型
    pub fn get_type(&self, name: String) -> bool {
        unsafe { dukc_get_type(self.vm as *const c_void_ptr, CString::new(name).unwrap().as_ptr()) != 0 }
    }

    //构建指定类型的对象，构建失败返回undefined
    pub fn new_type(&self, name: String, len: usize) -> JSType {
        let ptr: i32;
        let t = match name.as_str() {
            "Array" => JSValueType::Array as u8,
            "ArrayBuffer" => JSValueType::ArrayBuffer as u8,
            "Uint8Array" => JSValueType::Uint8Array as u8,
            _ => JSValueType::Object as u8,
        };
        unsafe { ptr = dukc_new_type(self.vm as *const c_void_ptr, len as u8) }
        if ptr < 0 {
            self.new_undefined()
        } else {
            JSType {
                type_id: t,
                is_drop: false,
                vm: self.vm,
                value: ptr as usize,
            }
        }
    }

    //设置指定对象的域
    pub fn set_field(&self, object: &JSType, key: String, value: &mut JSType) -> bool {
        if (self.vm != object.vm) || (self.vm != value.vm) {
            //如果对象和值不是在指定虚拟机上创建的，则忽略
            return false;
        }
        unsafe {
            if dukc_set_object_field(self.vm as *const c_void_ptr, object.value as u32, CString::new(key).unwrap().as_ptr(),
                value.value as u32) == 0 {
                    return false;
            }
            if value.is_drop {
                //已使用，则设置为不自动释放
                value.is_drop = false;
            }
            true
        }
    }

    //构建数组
    pub fn new_array(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_array(self.vm as *const c_void_ptr) }
        JSType {
            type_id: JSValueType::Array as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //设置指定数组指定偏移的值
    pub fn set_index(&self, array: &JSType, index: u32, value: &mut JSType) -> bool {
        if (self.vm != array.vm) || (self.vm != value.vm) {
            //如果数组和值不是在指定虚拟机上创建的，则忽略
            return false;
        }
        unsafe { if dukc_set_array_index(self.vm as *const c_void_ptr, array.value as u32, index, value.value as u32) == 0 {
            return false;
        }}
        if value.is_drop {
                //已使用，则设置为不自动释放
                value.is_drop = false;
            }
        true
    }

    //构建ArrayBuffer
    pub fn new_array_buffer(&self, length: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_array_buffer(self.vm as *const c_void_ptr, length) }
        JSType {
            type_id: JSValueType::ArrayBuffer as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建Uint8Array
    pub fn new_uint8_array(&self, length: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_uint8_array(self.vm as *const c_void_ptr, length) }
        JSType {
            type_id: JSValueType::Uint8Array as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建NativeObject
    pub fn new_native_object(&self, instance: usize) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_native_object(self.vm as *const c_void_ptr, instance as u64) }
        JSType {
            type_id: JSValueType::NativeObject as u8,
            is_drop: false,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取指定函数
    pub fn get_js_function(&self, func: String) -> bool {
        unsafe { if dukc_get_js_function(self.vm as *const c_void_ptr, CString::new(func).unwrap().as_ptr()) == 0 {
            return false;
        }}
        true
    }

    //链式获取指定函数
    pub fn get_link_function(&self, func: String) -> bool {
        unsafe { if dukc_link_js_function(self.vm as *const c_void_ptr, CString::new(func).unwrap().as_ptr()) == 0 {
            return false;
        }}
        true
    }

    //链式检查指定函数
    pub fn check_function(&self, func: String) -> bool {
        unsafe { if dukc_check_js_function(self.vm as *const c_void_ptr, CString::new(func).unwrap().as_ptr()) == 0 {
            return false;
        }}
        true
    }

    //调用指定函数
    pub fn call(&self, len: usize) {
        let vm = self.vm;
        unsafe {
            let status = dukc_vm_status_switch(vm as *const c_void_ptr, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机正在destroy或有其它任务
                println!("invalid vm status with call");
            } else {
                //增加当前虚拟机消息队列长度，并开始执行任务
                self.add_queue_len();
                dukc_call(self.vm as *const c_void_ptr, len as u8, js_reply_callback);
            }
        }
    }

    //设置指定全局变量的值，需要传递值的所有权，所以只读的值不允许设置为全局变量
    pub fn set_global_var(&self, key: String, value: JSType) -> bool {
        unsafe {
            if dukc_set_global_var(self.vm as *const c_void_ptr, CString::new(key).unwrap().as_ptr()) == 0 {
                return false;
            }
            if value.is_drop {
                //已使用，则设置为不自动释放
                let mut value_mut = value;
                value_mut.is_drop = false;
            }
            true
        }
    }

    //调用指定函数，并返回
    pub fn invoke(&self, len: usize) -> JSType {
        let ptr: i32;
        let vm = self.vm as *const c_void_ptr;
        unsafe {
            ptr = dukc_invoke(vm, len as u8);
            if ptr < 0 {
                JSType {
                    type_id: JSValueType::None as u8,
                    is_drop: false, //执行函数失败没有返回值，不需要回收
                    vm: self.vm,
                    value: 0,
                }
            } else {
                let t = dukc_get_value_type(vm, ptr as u32);
                JSType {
                    type_id: t,
                    is_drop: true, //执行函数成功的返回值，需要被回收
                    vm: self.vm,
                    value: ptr as usize,
                }
            }
        }
    }

    //执行指定脚本，返回值无法绑定全局变量，为了使用安全返回只读值
    pub fn eval(&self, script: String) -> AJSType {
        let ptr: i32;
        let vm = self.vm as *const c_void_ptr;
        unsafe {
            ptr = dukc_eval(vm, CString::new(script).unwrap().as_ptr());
            if ptr <= 0 {
                println!("{:?}, {:?}", ptr, self.dump_stack());
                Arc::new(JSType {
                    type_id: JSValueType::None as u8,
                    is_drop: false, //执行脚本失败没有返回值，不需要回收
                    vm: self.vm,
                    value: 0,
                })
            } else {
                println!("{:?}, {:?}", ptr, self.dump_stack());
                let t = dukc_get_value_type(vm, ptr as u32);
                Arc::new(JSType {
                    type_id: t,
                    is_drop: true, //执行脚本成功的返回值，需要被回收
                    vm: self.vm,
                    value: ptr as usize,
                })
            }
        }
    }

    //获取当前虚拟机栈顶数据信息
    pub fn stack_top_string(&self) -> Option<String> {
        let value;
        unsafe {
            value = dukc_top(self.vm as *const c_void_ptr);
            if value < 0 {
                None
            } else {
                let ptr = dukc_to_string(self.vm as *const c_void_ptr, value);
                if ptr.is_null() {
                    return None;
                }

                Some(CStr::from_ptr(ptr as *const c_char).to_string_lossy().into_owned())
            }
        }
    }

    //获取当前虚拟机堆栈信息
    pub fn dump_stack(&self) -> String {
        unsafe { CStr::from_ptr(dukc_dump_stack(self.vm as *const c_void_ptr)).to_string_lossy().into_owned() }
    }

    //获取当前虚拟机指定栈帧信息
    pub fn stack_frame(&self, index: u32) -> Option<(String, isize)> {
        unsafe {
            let ptr = dukc_stack_frame(self.vm as *const c_void_ptr, index);
            if ptr.is_null() {
                return None;
            }

            let frame = CStr::from_ptr(ptr as *const c_char).to_string_lossy().into_owned();
            let vec: Vec<&str> = frame.split(';').collect();
            Some((vec[0].to_string(), vec[1].parse().unwrap()))
        }
    }

    //检查当前虚拟机是否需要启动全局虚拟机整理、返回丢弃0、释放1或忽略2
    pub fn check_reuse(&self) -> usize {
        if let Some((_, factory)) = &self.collection {
            //设置了回收器，则满足条件时会清理当前虚拟机
            if is_alloced_limit() {
                //当前已分配内存已达最大堆限制，则启动全局虚拟机堆整理
                register_global_vm_heap_collect_timer(0);
            }

            let max_heap_size = factory.max_heap_size();
            if (max_heap_size > 0) && self.heap_size() >= max_heap_size {
                //已达虚拟机最大堆限制
                let max_reused_count = factory.max_reused_count();
                if (max_reused_count > 0) && self.reused_count() >= max_reused_count {
                    //已达虚拟机最大执行次数限制，则丢弃
                    0
                } else {
                    //未达虚拟机最大执行次数限制，则释放
                    1
                }
            } else {
                //未达虚拟机最大堆限制，则忽略
                2
            }
        } else {
            //未设置回收器，则会丢弃当前虚拟机
            0
        }
    }

    //判断当前虚拟机是否需要被丢弃
    pub fn is_throw(&self) -> usize {
        if let Some((_, factory)) = &self.collection {
            //设置了回收器，则有机率丢弃
            let heap_limit = factory.heap_size();
            let heap_size = self.heap_size();
            let mut vm_heap_size = vm_alloced_size();
            let max_heap_limit = get_max_alloced_limit();

            if vm_alloced_size() < 0 {
                //如果当前堆最大大小小于0，则重置为0
                VM_ALLOCATED.store(0, Ordering::Relaxed);
                vm_heap_size = 0;
            }

            let n: f64;
            if heap_size >= heap_limit {
                //当前堆大小超过堆限制，则堆大小余量比例为0
                n = 0.0;
            } else {
                //当前堆大小未超过堆限制，则余量为(1 - 当前堆限制比例)
                n = 1.0 - heap_size as f64 / heap_limit as f64;
            }

            let s: f64;
            if is_alloced_limit() {
                //当前所有虚拟机内存占用超过最大堆限制，则内存占用比例为1
                s = 1.0;
            } else {
                //当前所有虚拟机内存占用未超过最大堆限制
                s = vm_heap_size as f64 / max_heap_limit as f64;
            }

            //求虚拟机丢弃机率
            let r = s.powf(8.0).powf(n);
            if self.rng.borrow_mut().gen_bool(r) {
                if is_alloced_limit() {
                    //丢弃当前虚拟机
                    0
                } else {
                    //丢弃当前虚拟机，且创建新的虚拟机
                    1
                }
            } else {
                //复用
                2
            }
        } else {
            //没有设置回收器，则一定会被丢弃
            0
        }
    }
}

/*
* 值类型
*/
pub enum JSValueType {
    None = 0x0,
    Undefined,
    Null,
    Boolean,
    Number,
    String,
    Object,
    NativeObject = 0x3c,
    Array,
    ArrayBuffer,
    Uint8Array,
}

/*
* 只读js类型，不允许用于js运行，只用于rust读取
*/
type AJSType = Arc<JSType>;

/*
* js类型
*/
#[derive(Clone)]
pub struct JSType {
    type_id:    u8,
    is_drop:    bool,
    vm:         usize,
    value:      usize,
}

/*
* 尝试destroy虚拟机的值
*/
pub unsafe fn try_value_destroy(js: &JSType) {
    if !js.is_drop {
        return;
    }
    dukc_remove_value(js.vm as *const c_void_ptr, js.value as u32);
}

impl Drop for JSType {
    fn drop(&mut self) {
        unsafe { try_value_destroy(self); }
    }
}

impl JSType {
    //构建一个指定js类型
    pub unsafe fn new(type_id: u8, is_drop: bool, vm: *const c_void_ptr, ptr: *const c_void_ptr) -> Self {
        JSType {
            type_id: type_id,
            is_drop: is_drop,
            vm: vm as usize,
            value: ptr as usize,
        }
    }

    //获取指定类型的类型id
    fn get_type_id(&self, value: u32) -> u8 {
        unsafe { dukc_get_value_type(self.vm as *const c_void_ptr, value) as u8 }
    }

    //获取内部值
    pub fn get_value(&self) -> usize {
        self.value
    }

    //判断是否是无效值
	pub fn is_none(&self) -> bool {
        if self.type_id == JSValueType::None as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是undefined
	pub fn is_undefined(&self) -> bool {
        if self.type_id == JSValueType::Undefined as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是null
    pub fn is_null(&self) -> bool {
        if self.type_id == JSValueType::Null as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是boolean
	pub fn is_boolean(&self) -> bool {
        if self.type_id == JSValueType::Boolean as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是数字
	pub fn is_number(&self) -> bool {
        if self.type_id == JSValueType::Number as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是字符串
	pub fn is_string(&self) -> bool {
        if self.type_id == JSValueType::String as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是对象
	pub fn is_object(&self) -> bool {
        if self.type_id == JSValueType::Object as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是数组
	pub fn is_array(&self) -> bool {
        if self.type_id == JSValueType::Array as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是ArrayBuffer
	pub fn is_array_buffer(&self) -> bool {
        if self.type_id == JSValueType::ArrayBuffer as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是Uint8Array
	pub fn is_uint8_array(&self) -> bool {
        if self.type_id == JSValueType::Uint8Array as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是NativeObject
	pub fn is_native_object(&self) -> bool {
        if self.type_id == JSValueType::NativeObject as u8 {
            true
        } else {
            false
        }
    }

    //获取boolean
    pub fn get_boolean(&self) -> bool {
        let num: u8;
        unsafe { num = dukc_get_boolean(self.vm as *const c_void_ptr, self.value as u32) }
        if num == 0 {
            false
        } else {
            true
        }
    }

    //获取i8
    pub fn get_i8(&self) -> i8 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as i8 }
    }

    //获取i16
	pub fn get_i16(&self) -> i16 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as i16 }
    }

    //获取i32
	pub fn get_i32(&self) -> i32 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as i32 }
    }

    //获取i64
	pub fn get_i64(&self) -> i64 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as i64 }
    }

    //获取u8
	pub fn get_u8(&self) -> u8 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as u8 }
    }

    //获取u16
	pub fn get_u16(&self) -> u16 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as u16 }
    }

    //获取u32
	pub fn get_u32(&self) -> u32 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as u32 }
    }

    //获取u64
	pub fn get_u64(&self) -> u64 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as u64 }
    }

    //获取f32
	pub fn get_f32(&self) -> f32 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as f32 }
    }

    //获取f64
	pub fn get_f64(&self) -> f64 {
        unsafe { dukc_get_number(self.vm as *const c_void_ptr, self.value as u32) as f64 }
    }

    //获取字符串
	pub fn get_str(&self) -> String {
        unsafe { CStr::from_ptr(dukc_get_string(self.vm as *const c_void_ptr, self.value as u32)).to_string_lossy().into_owned() }
    }

    //获取对象指定域的值，注意获取的值在读取后需要立即调用dukc_remove_value函数移除掉
	pub fn get_field(&self, key: String) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_get_object_field(self.vm as *const c_void_ptr, self.value as u32, CString::new(key).unwrap().as_ptr()) }
        let is_drop = if self.get_type_id(ptr) == JSValueType::None as u8 {
            false //无值则不需要自运drop
        } else {
            true //对象成员自动drop
        };
        JSType {
            type_id: self.get_type_id(ptr),
            is_drop: is_drop,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取数组长度
    pub fn get_array_length(&self) -> usize {
        unsafe { dukc_get_array_length(self.vm as *const c_void_ptr, self.value as u32) as usize }
    }

    //获取数组指定偏移的值，注意获取的值在读取后需要立即调用dukc_remove_value函数移除掉
	pub fn get_index(&self, index: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_get_array_index(self.vm as *const c_void_ptr, self.value as u32, index) }
        let is_drop = if self.get_type_id(ptr) == JSValueType::None as u8 {
            false //无值则不需要自运drop
        } else {
            true //数组成员需要自动drop
        };
        JSType {
            type_id: self.get_type_id(ptr),
            is_drop: is_drop,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取指定Buffer的引用
    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            let length = dukc_get_buffer_length(self.vm as *const c_void_ptr, self.value as u32) as usize;
            let buffer = dukc_get_buffer(self.vm as *const c_void_ptr, self.value as u32);
            from_raw_parts(buffer as *const u8, length)
        }
    }

    //获取指定Buffer的引用
    pub unsafe fn to_bytes_mut(&mut self) -> &mut [u8] {
        let length = dukc_get_buffer_length(self.vm as *const c_void_ptr, self.value as u32) as usize;
        let buffer = dukc_get_buffer(self.vm as *const c_void_ptr, self.value as u32);
        from_raw_parts_mut(buffer as *mut u8, length)
    }

    //获取指定Buffer的复制
	pub fn into_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    //重置指定的Buffer
	pub fn from_bytes(&self, bytes: &[u8]) {
        unsafe {
            let length = dukc_get_buffer_length(self.vm as *const c_void_ptr, self.value as u32) as usize;
            let buffer = dukc_get_buffer(self.vm as *const c_void_ptr, self.value as u32);
            memcpy(buffer as *mut c_void_ptr, bytes.as_ptr() as *const c_void_ptr, length);
        }
    }

    //获取指定的Buffer
    pub fn into_buffer(&self) -> JSBuffer {
        unsafe {
            let length = dukc_get_buffer_length(self.vm as *const c_void_ptr, self.value as u32) as usize;
            let buffer = dukc_get_buffer(self.vm as *const c_void_ptr, self.value as u32);
            JSBuffer::new(buffer as *mut c_void_ptr, length)
        }
    }

    //获取NativeObject
	pub fn get_native_object(&self) -> usize {
        unsafe { dukc_get_native_object_instance(self.vm as *const c_void_ptr, self.value as u32) as usize }
    }

    //获取类型值的字符串描述
    pub fn to_string(&self) -> Option<String> {
        unsafe {
            let ptr = dukc_to_string(self.vm as *const c_void_ptr, self.value as i32);
            if ptr.is_null() {
                return None;
            }

            Some(CStr::from_ptr(ptr as *const c_char).to_string_lossy().into_owned())
        }
    }
}

/*
* Js Buffer
*/
pub struct JSBuffer {
    buffer: *mut c_void_ptr,
    len: usize,
}

impl JSBuffer {
    //构建JSBuffer
    pub fn new(ptr: *mut c_void_ptr, len: usize) -> Self {
        JSBuffer {
            buffer: ptr,
            len: len,
        }
    }

    //获取buffer字符数
    pub fn len(&self) -> usize {
        self.len
    }

    //在指定位置读小端i8
    pub fn read_i8(&self, offset: usize) -> i8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i8::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i8)) }
    }

    //在指定位置读小端i16
    pub fn read_i16(&self, offset: usize) -> i16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i16::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i16)) }
    }

    //在指定位置读小端i32
    pub fn read_i32(&self, offset: usize) -> i32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i32::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i32)) }
    }

    //在指定位置读小端i64
    pub fn read_i64(&self, offset: usize) -> i64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i64::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i64)) }
    }

    //在指定位置读小端u8
    pub fn read_u8(&self, offset: usize) -> u8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u8::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u8)) }
    }

    //在指定位置读小端u16
    pub fn read_u16(&self, offset: usize) -> u16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u16::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u16)) }
    }

    //在指定位置读小端u32
    pub fn read_u32(&self, offset: usize) -> u32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u32::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u32)) }
    }

    //在指定位置读小端u64
    pub fn read_u64(&self, offset: usize) -> u64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u64::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u64)) }
    }

    //在指定位置读小端f32
    pub fn read_f32(&self, offset: usize) -> f32 {
        unsafe { transmute::<u32, f32>(self.read_u32(offset)) }
    }

    //在指定位置读小端f64
    pub fn read_f64(&self, offset: usize) -> f64 {
        unsafe { transmute::<u64, f64>(self.read_u64(offset)) }
    }

    //在指定位置读大端i8
    pub fn read_i8_be(&self, offset: usize) -> i8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i8::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i8)) }
    }

    //在指定位置读大端i16
    pub fn read_i16_be(&self, offset: usize) -> i16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i16::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i16)) }
    }

    //在指定位置读大端i32
    pub fn read_i32_be(&self, offset: usize) -> i32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i32::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i32)) }
    }

    //在指定位置读大端i64
    pub fn read_i64_be(&self, offset: usize) -> i64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i64::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i64)) }
    }

    //在指定位置读大端u8
    pub fn read_u8_be(&self, offset: usize) -> u8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u8::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u8)) }
    }

    //在指定位置读大端u16
    pub fn read_u16_be(&self, offset: usize) -> u16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u16::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u16)) }
    }

    //在指定位置读大端u32
    pub fn read_u32_be(&self, offset: usize) -> u32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u32::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u32)) }
    }

    //在指定位置读大端u64
    pub fn read_u64_be(&self, offset: usize) -> u64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u64::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u64)) }
    }

    //在指定位置读大端f32
    pub fn read_f32_be(&self, offset: usize) -> f32 {
        unsafe { transmute::<u32, f32>(self.read_u32_be(offset)) }
    }

    //在指定位置读小端f64
    pub fn read_f64_be(&self, offset: usize) -> f64 {
        unsafe { transmute::<u64, f64>(self.read_u64_be(offset)) }
    }

    //在指定位置读字节数组
    pub fn read(&self, offset: usize, len: usize) -> &[u8] {
        if offset + len > self.len {
            panic!("access out of range");
        }
        unsafe {
            from_raw_parts(self.buffer.wrapping_offset(offset as isize) as *const u8, len)
        }
    }

    //在指定位置读UTF8字符串
    pub fn to_string(&self, offset: usize, len: usize) -> Result<String, FromUtf8Error> {
        if offset + len > self.len {
            panic!("access out of range");
        }

        let mut vec = Vec::new();
        vec.resize(len, 0);
        unsafe {
            vec.copy_from_slice(from_raw_parts(self.buffer.wrapping_offset(offset as isize) as *const u8, len));
        }
        String::from_utf8(vec)
    }

    //在指定位置写小端i8
    pub fn write_i8(&mut self, offset: usize, v: i8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i8) = v.to_le() }
        last as isize
    }

    //在指定位置写小端i16
    pub fn write_i16(&mut self, offset: usize, v: i16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i16) = v.to_le() }
        last as isize
    }

    //在指定位置写小端i32
    pub fn write_i32(&mut self, offset: usize, v: i32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i32) = v.to_le() }
        last as isize
    }

    //在指定位置写小端i64
    pub fn write_i64(&mut self, offset: usize, v: i64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i64) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u8
    pub fn write_u8(&mut self, offset: usize, v: u8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u8) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u16
    pub fn write_u16(&mut self, offset: usize, v: u16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u16) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u32
    pub fn write_u32(&mut self, offset: usize, v: u32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u32) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u64
    pub fn write_u64(&mut self, offset: usize, v: u64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u64) = v.to_le() }
        last as isize
    }

    //在指定位置写小端f32
    pub fn write_f32(&mut self, offset: usize, v: f32) -> isize {
        self.write_u32(offset, unsafe { transmute::<f32, u32>(v) })
    }

    //在指定位置写小端f64
    pub fn write_f64(&mut self, offset: usize, v: f64) -> isize {
        self.write_u64(offset, unsafe { transmute::<f64, u64>(v) })
    }

    //在指定位置写大端i8
    pub fn write_i8_be(&mut self, offset: usize, v: i8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i8) = v.to_be() }
        last as isize
    }

    //在指定位置写大端i16
    pub fn write_i16_be(&mut self, offset: usize, v: i16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i16) = v.to_be() }
        last as isize
    }

    //在指定位置写大端i32
    pub fn write_i32_be(&mut self, offset: usize, v: i32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i32) = v.to_be() }
        last as isize
    }

    //在指定位置写大端i64
    pub fn write_i64_be(&mut self, offset: usize, v: i64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i64) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u8
    pub fn write_u8_be(&mut self, offset: usize, v: u8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u8) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u16
    pub fn write_u16_be(&mut self, offset: usize, v: u16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u16) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u32
    pub fn write_u32_be(&mut self, offset: usize, v: u32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u32) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u64
    pub fn write_u64_be(&mut self, offset: usize, v: u64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u64) = v.to_be() }
        last as isize
    }

    //在指定位置写大端f32
    pub fn write_f32_be(&mut self, offset: usize, v: f32) -> isize {
        self.write_u32_be(offset, unsafe { transmute::<f32, u32>(v) })
    }

    //在指定位置写大端f64
    pub fn write_f64_be(&mut self, offset: usize, v: f64) -> isize {
        self.write_u64_be(offset, unsafe { transmute::<f64, u64>(v) })
    }

    //从指定位置写字节数组
    pub fn write(&mut self, offset: usize, v: &[u8]) -> isize {
        let len = v.len();
        let last = offset + len;
        if  last > self.len {
            return -1;
        }
        unsafe { memcpy(self.buffer.wrapping_offset(offset as isize), v.as_ptr() as *const c_void_ptr, len); }
        last as isize
    }
}

/*
* 线程安全的设置虚拟机超时时长，单位ms，返回上次超时时长
*/
pub fn set_vm_timeout(timeout: usize) -> usize {
    VM_TIMEOUT.swap(timeout * 1000, Ordering::SeqCst) / 1000
}

/*
* 线程安全的注册全局虚拟机堆整理定时器
*/
pub fn register_global_vm_heap_collect_timer(collect_timeout: usize) {
    let vm_timeout = VM_TIMEOUT.load(Ordering::Relaxed);
    let runner = FuncRuner::new(Box::new(move || {
        let start_time = Instant::now();
        let mut current_heap_size = all_alloced_size();
        let mut max_heap_limit = get_max_alloced_limit();
        if !is_alloced_limit() {
            //当前已分配内存未达最大堆限制，则忽略本次整理，并注册下次整理
            if collect_timeout > 0 {
                register_global_vm_heap_collect_timer(collect_timeout);
            }

            println!("===> Vm Global Collect Ignore, current: {}, limit: {}, time: {:?}", current_heap_size, max_heap_limit, Instant::now() - start_time);
            return;
        }

        //当前已分配内存已达最大堆限制，则开始虚拟机工厂的超时整理
        let timeout_count = Arc::new(AtomicUsize::new(0));
        let mut caches = Vec::new();
        for factory in VM_FACTORY_REGISTERS.read().unwrap().values() {
            let now = now_utc();
            let timeout_count_copy = timeout_count.clone();
            let cache = Arc::new(SegQueue::new());
            let cache_copy = cache.clone();

            factory.collect(Arc::new(move |vm: &mut Arc<JS>| {
                //整理当前虚拟机工厂内，所有空闲的虚拟机
                if (vm_timeout > 0) && (now - vm.last_time()) >= vm_timeout {
                    //虚拟机已超时，则丢弃
                    timeout_count_copy.fetch_add(1, Ordering::Relaxed);
                    true
                } else {
                    //虚拟机未超时，则将虚拟机引用加入整理缓存，并忽略
                    cache_copy.push(vm.clone());
                    false
                }
            }));

            caches.push((factory.max_reused_count(), cache)); //保存当前虚拟机工厂的整理缓存
        }

        let mut last_heap_size = current_heap_size;
        current_heap_size = all_alloced_size();
        if !is_alloced_limit() {
            //当前已分配内存未达最大堆限制，则将缓存的虚拟机引用释放，结束本次整理，并注册下次整理
            for i in 0..caches.len() {
                let (_, cache) = &caches[i];
                while let Ok(vm) = cache.pop() {}
            }

            if collect_timeout > 0 {
                register_global_vm_heap_collect_timer(collect_timeout);
            }

            println!("===> Vm Global Timeout Collect Finish, count: {}, before: {}, after: {}, limit: {}, time: {:?}",
                     timeout_count.load(Ordering::Relaxed), last_heap_size, current_heap_size, max_heap_limit, Instant::now() - start_time);
            return;
        }

        //当前已分配内存仍然已达最大堆限制，则开始虚拟机工厂的空闲虚拟机丢弃整理
        let mut throw_count = 0;
        for i in 0..caches.len() {
            let (_, cache) = &caches[i];
            while let Ok(vm) = cache.pop() {
                //设置虚拟机上次运行时间为0，以保证虚拟机在下次全局整理中被丢弃，并释放虚拟机引用
                vm.set_last_time(0);
                throw_count += 1;
            }
        }

        //丢弃整理完成，则结束本次整理，并注册下次整理
        if collect_timeout > 0 {
            register_global_vm_heap_collect_timer(collect_timeout);
        }

        last_heap_size = current_heap_size;
        current_heap_size = all_alloced_size();
        println!("===> Vm Global Throw Collect Finish, count: {}, before: {}, after: {}, limit: {}, time: {:?}",
                 throw_count, last_heap_size, current_heap_size, max_heap_limit, Instant::now() - start_time);
    }));

    TIMER.set_timeout(runner, collect_timeout as u32);
}
