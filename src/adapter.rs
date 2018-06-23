use libc::{c_void, c_char, int8_t, uint8_t, c_int, /* int32_t, */ uint32_t, uint64_t, c_double, memcpy};
use std::slice::{from_raw_parts_mut, from_raw_parts};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::string::FromUtf8Error;
use std::ffi::{CStr, CString};
use std::mem::transmute;
use std::boxed::FnBox;
use std::ops::Drop;
use std::sync::Arc;

use magnetic::mpsc::*;
use magnetic::{Producer, Consumer};
use magnetic::buffer::dynamic::DynamicBuffer;

#[cfg(not(unix))]
use kernel32;

use pi_lib::atom::Atom;
use pi_base::task::TaskType;
use pi_base::pi_base_impl::cast_js_task;

use native_object_impl::*;

#[link(name = "libdukc")]
extern "C" {
    fn dukc_register_native_object_function_call(func: extern fn(*const c_void, uint32_t, uint32_t, *const c_void, *const c_void) -> c_int);
    fn dukc_register_native_object_free(func: extern fn(*const c_void, uint32_t));
    fn dukc_heap_create() -> *const c_void;
    fn dukc_heap_init(vm: *const c_void, reply: extern fn(*const c_void, c_int, *const c_char)) -> uint32_t;
    // fn dukc_vm_create(heap: *const c_void) -> *const c_void;
    fn dukc_compile_script(vm: *const c_void, file: *const c_char, code: *const c_char, size: *mut uint32_t, reply: extern fn(*const c_void, c_int, *const c_char)) -> *const c_void;
    fn dukc_load_code(vm: *const c_void, size: uint32_t, bytes: *const c_void) -> uint32_t;
    fn dukc_bind_vm(vm: *const c_void, handler: *const c_void);
    // fn dukc_vm_clone(size: uint32_t, bytes: *const c_void, reply: extern fn(*const c_void, c_int, *const c_char)) -> *const c_void;
    fn dukc_vm_run(vm: *const c_void, reply: extern fn(*const c_void, c_int, *const c_char));
    pub fn dukc_vm_status_check(vm: *const c_void, value: int8_t) -> uint8_t;
    pub fn dukc_vm_status_switch(vm: *const c_void, old_status: int8_t, new_status: int8_t) -> int8_t;
    pub fn dukc_vm_status_sub(vm: *const c_void, value: int8_t) -> int8_t;
    fn dukc_new_null(vm: *const c_void) -> uint32_t;
    fn dukc_new_undefined(vm: *const c_void) -> uint32_t;
    fn dukc_new_boolean(vm: *const c_void, b: uint8_t) -> uint32_t;
    fn dukc_new_number(vm: *const c_void, num: c_double) -> uint32_t;
    fn dukc_new_string(vm: *const c_void, str: *const c_char) -> uint32_t;
    fn dukc_new_object(vm: *const c_void) -> uint32_t;
    fn dukc_set_object_field(vm: *const c_void, object: uint32_t, key: *const c_char, value: uint32_t) -> uint32_t;
    fn dukc_new_array(vm: *const c_void) -> uint32_t;
    fn dukc_set_array_index(vm: *const c_void, array: uint32_t, index: uint32_t, value: uint32_t) -> uint32_t;
    fn dukc_new_array_buffer(vm: *const c_void, length: uint32_t) -> uint32_t;
    fn dukc_new_uint8_array(vm: *const c_void, length: uint32_t) -> uint32_t;
    fn dukc_new_native_object(vm: *const c_void, ptr: uint64_t) -> uint32_t;
    pub fn dukc_remove_value(vm: *const c_void, value: uint32_t);
    fn dukc_get_value_type(vm: *const c_void, value: uint32_t) -> uint8_t;
    fn dukc_get_boolean(vm: *const c_void, value: uint32_t) -> uint8_t;
    fn dukc_get_number(vm: *const c_void, value: uint32_t) -> c_double;
    fn dukc_get_string(vm: *const c_void, value: uint32_t) -> *const c_char;
    fn dukc_get_object_field(vm: *const c_void, object: uint32_t, key: *const c_char) -> uint32_t;
    fn dukc_get_array_length(vm: *const c_void, array: uint32_t) -> uint32_t;
    fn dukc_get_array_index(vm: *const c_void, array: uint32_t, index: uint32_t) -> uint32_t;
    fn dukc_get_buffer_length(vm: *const c_void, value: uint32_t) -> uint32_t;
    fn dukc_get_buffer(vm: *const c_void, value: uint32_t) -> *const c_void;
    fn dukc_get_native_object_instance(vm: *const c_void, value: uint32_t) -> uint64_t;
    fn dukc_get_js_function(vm: *const c_void, func: *const c_char) -> uint32_t;
    pub fn dukc_get_callback(vm: *const c_void, index: uint32_t) -> uint32_t ;
    pub fn dukc_call(vm: *const c_void, len: uint8_t, reply: extern fn(*const c_void, c_int, *const c_char));
    pub fn dukc_throw(vm: *const c_void, reason: *const c_char);
    pub fn dukc_wakeup(vm: *const c_void, error: c_int) -> uint32_t;
    pub fn dukc_continue(vm: *const c_void, reply: extern fn(*const c_void, c_int, *const c_char));
    pub fn dukc_switch_context(vm: *const c_void);
    pub fn dukc_remove_callback(vm: *const c_void, index: uint32_t) -> uint32_t;
    // fn dukc_top(vm: *const c_void) -> int32_t;
    // fn dukc_to_string(vm: *const c_void, offset: int32_t) -> *const c_char;
    fn dukc_pop(vm: *const c_void);
    fn dukc_vm_destroy(vm: *const c_void);
}

/*
* js返回回调函数
*
* 当前异步任务如果没有投递其它异步任务，则当前异步任务成为同步任务，并在当前异步任务完成后执行异步消息队列中的回调函数，如果没有异步消息，则回收虚拟机
*  否则还有其它异步任务，则回收权利交由其它异步任务
*/
#[no_mangle]
pub extern "C" fn js_reply_callback(handler: *const c_void, status: c_int, err: *const c_char) {
    if handler.is_null() {
        //处理初始化异常
        if status != 0 {
            println!("===> JS Init Error, status: {}, err: {}", status, unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() });
        }
        return;
    }

    let js: Arc<JS>;
    let vm: *const c_void;
    unsafe {
        js = JS::from_raw(handler);
        vm = js.get_vm();

        //处理执行异常
        if status != 0 {
            println!("===> JS Run Error, vm: {}, queue size: {}, status: {}, err: {}", 
                                                                                vm as usize, 
                                                                                js.queue.size.load(Ordering::Relaxed), 
                                                                                status, 
                                                                                CStr::from_ptr(err).to_string_lossy().into_owned());
        }

        if dukc_vm_status_check(vm, JSStatus::WaitBlock as i8) > 0 {
            //当前虚拟机任务已执行完成且当前虚拟机状态是等待状态，则需要改变状态，保证虚拟机异步任务被执行
            dukc_vm_status_sub(vm, 1);
        } else if dukc_vm_status_check(vm, JSStatus::SingleTask as i8) > 0 {
            //当前虚拟机同步任务、异步任务或异步回调已执行完成，且当前虚拟机状态是同步状态，则处理异步消息队列
            let msg: JSMsg;

            dukc_pop(vm); //移除上次同步任务、异步任务或回调函数的执行结果
            println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!call finish, callback start");
            loop {
                match js.pop() {
                    None => {
                        println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!callback empty");
                        //异步消息队列为空，则需要将执行结果弹出值栈并改变状态，保证虚拟机回收或执行下一个任务
                        dukc_vm_status_sub(vm, 1);
                        Arc::into_raw(js);
                        return;
                    },
                    Some(m) => {
                        //异步消息队列不为空，则获取回调函数，不需要改变状态，以保证当前虚拟机可以线程安全的执行回调函数
                        if dukc_get_callback(js.get_vm(), m.index) == 0 {
                            //当前回调函数不存在，则获取下一个异步消息
                            continue;
                        }
                        dukc_remove_callback(js.get_vm(), m.index); //移除回调函数
                        msg = m;
                        break;
                    },
                }
            }
            callback(js.clone(), msg);
        } else {
            //当前虚拟机为其它状态，则忽略
            Arc::into_raw(js);
            return;
        }
    }
    Arc::into_raw(js);
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
}

/*
* js异步消息
*/
pub struct JSMsg {
    index: u32,                         //异步回调函数的编号
    args: Box<FnBox(Arc<JS>) -> usize>, //异步回调函数的参数
    info: Atom,                         //消息
}

impl JSMsg {
    //构建一个异步消息
    pub fn new(index: u32, args: Box<FnBox(Arc<JS>) -> usize>, info: Atom) -> Self {
        JSMsg {
            index: index,
            args: args,
            info: info,
        }
    }
}

/*
* js异步消息队列
*/
#[derive(Clone)]
struct JSMsgQueue {
    size: Arc<AtomicUsize>,                                     //虚拟机异步消息队列长度
    producer: Arc<MPSCProducer<JSMsg, DynamicBuffer<JSMsg>>>,   //虚拟机异步消息队列生产者
    consumer: Arc<MPSCConsumer<JSMsg, DynamicBuffer<JSMsg>>>,   //虚拟机异步消息队列消费者
}

/*
* js运行环境
*/
#[derive(Clone)]
pub struct JS {
    vm: usize,                  //虚拟机
    queue: JSMsgQueue,          //虚拟机异步消息队列
}

/*
* 尝试destroy虚拟机
*/
pub fn try_js_destroy(js: &JS) {
    if js.vm == 0 {
        return;
    }

    unsafe {
        let old_status = dukc_vm_status_switch(js.vm as *const c_void, JSStatus::NoTask as i8, JSStatus::Destroy as i8);
        if old_status == JSStatus::NoTask as i8 {
            //当前js虚拟机无任务，则可以destroy
            dukc_vm_destroy(js.vm as *const c_void);
        }
    }
}

impl Drop for JS {
    fn drop(&mut self) {
        try_js_destroy(self);
    }
}

impl JS {
    //构建一个虚拟机
    pub fn new(queue_max_size: u16) -> Option<Arc<Self>> {
        let ptr: *const c_void;
        unsafe { ptr = dukc_heap_create() }
        if ptr.is_null() {
            None
        } else {
            let (p, c) = mpsc_queue(DynamicBuffer::new(queue_max_size as usize).unwrap());
            unsafe {
                if dukc_heap_init(ptr, js_reply_callback) == 0 {
                    dukc_vm_destroy(ptr);
                    return None;
                }
                dukc_vm_run(ptr, js_reply_callback);
            }
            let arc = Arc::new(JS {
                vm: ptr as usize,
                queue: JSMsgQueue {
                    size: Arc::new(AtomicUsize::new(0)),
                    producer: Arc::new(p),
                    consumer: Arc::new(c),
                },
            });
            unsafe { dukc_bind_vm(ptr, Arc::into_raw(arc.clone()) as *const c_void); }
            Some(arc)
        }
    }

    //从指针构建指定虚拟机
    pub unsafe fn from_raw(ptr: *const c_void) -> Arc<Self> {
        Arc::from_raw(ptr as *const JS)
    }

    //获取内部虚拟机
    pub unsafe fn get_vm(&self) -> *const c_void {
        self.vm as *const c_void
    }

    //判断js虚拟机是否完成运行
    pub fn is_ran(&self) -> bool {
        unsafe { dukc_vm_status_check(self.vm as *const c_void, JSStatus::NoTask as i8) > 0 }
    }

    //编译指定脚本
    pub fn compile(&self, file: String, script: String) -> Option<Vec<u8>> {
        let mut len = 0u32;
        let size: *mut u32 = &mut len;
        unsafe {
            let bytes = dukc_compile_script(self.vm as *const c_void, CString::new(file).unwrap().as_ptr(), CString::new(script).unwrap().as_ptr(), size, js_reply_callback);
            if bytes.is_null() {
                return None;                
            }
            Some(from_raw_parts(bytes as *mut u8, len as usize).to_vec())
        }
    }

    //加载指定代码
    pub fn load(&self, codes: &[u8]) -> bool {
        let size = codes.len() as u32;
        let bytes = codes.as_ptr() as *const c_void;
        unsafe {
            if dukc_load_code(self.vm as *const c_void, size, bytes) == 0 {
                return false;
            }
        }
        self.run();
        true
    }

    //运行js虚拟机
    pub fn run(&self) {
        unsafe { 
            let status = dukc_vm_status_switch(self.vm as *const c_void, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机状态错误，无法运行
                println!("invalid vm status with run");
            } else {
                dukc_vm_run(self.vm as *const c_void, js_reply_callback);
            }
        }
    }

    //构建undefined
    pub fn new_undefined(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_undefined(self.vm as *const c_void) }
        JSType {
            type_id: JSValueType::Undefined as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建null
    pub fn new_null(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_null(self.vm as *const c_void) }
        JSType {
            type_id: JSValueType::Null as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建boolean
    pub fn new_boolean(&self, b: bool) -> JSType {
        let ptr: u32;
        unsafe {
            if b {
                ptr = dukc_new_boolean(self.vm as *const c_void, 1u8); 
            } else {
                ptr = dukc_new_boolean(self.vm as *const c_void, 0u8); 
            }
        }
        JSType {
            type_id: JSValueType::Boolean as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i8
    pub fn new_i8(&self, num: i8) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i16
    pub fn new_i16(&self, num: i16) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i32
    pub fn new_i32(&self, num: i32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i64
    pub fn new_i64(&self, num: i64) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u8
    pub fn new_u8(&self, num: u8) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u16
    pub fn new_u16(&self, num: u16) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u32
    pub fn new_u32(&self, num: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u64
    pub fn new_u64(&self, num: u64) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建f32
    pub fn new_f32(&self, num: f32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建f64
    pub fn new_f64(&self, num: f64) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_number(self.vm as *const c_void, num) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建字符串，注意rust的字符串默认是UTF8编码，而JS是UTF16编码
    pub fn new_str(&self, str: String) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_string(self.vm as *const c_void, CString::new(str).unwrap().as_ptr()) }
        JSType {
            type_id: JSValueType::String as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建对象
    pub fn new_object(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_object(self.vm as *const c_void) }
        JSType {
            type_id: JSValueType::Object as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }
    
    //设置指定对象的域
    pub fn set_field(&self, object: &JSType, key: String, value: &JSType) -> bool {
        if (self.vm != object.vm) || (self.vm != value.vm){
            //如果对象和值不是在指定虚拟机上创建的，则忽略
            return false;
        }
        unsafe { 
            if dukc_set_object_field(self.vm as *const c_void, object.value as u32, CString::new(key).unwrap().as_ptr(), 
                value.value as u32) == 0 {
                    return false;
            }
            true
        }
    }

    //构建数组
    pub fn new_array(&self) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_array(self.vm as *const c_void) }
        JSType {
            type_id: JSValueType::Array as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //设置指定数组指定偏移的值
    pub fn set_index(&self, array: &JSType, index: u32, value: &JSType) -> bool {
        if (self.vm != array.vm) || (self.vm != value.vm){
            //如果数组和值不是在指定虚拟机上创建的，则忽略
            return false;
        }
        unsafe { if dukc_set_array_index(self.vm as *const c_void, array.value as u32, index, value.value as u32) == 0 {
            return false;
        }}
        return true;
    }

    //构建ArrayBuffer
    pub fn new_array_buffer(&self, length: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_array_buffer(self.vm as *const c_void, length) }
        JSType {
            type_id: JSValueType::ArrayBuffer as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建Uint8Array
    pub fn new_uint8_array(&self, length: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_uint8_array(self.vm as *const c_void, length) }
        JSType {
            type_id: JSValueType::Uint8Array as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建NativeObject
    pub fn new_native_object(&self, instance: usize) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_new_native_object(self.vm as *const c_void, instance as u64) }
        JSType {
            type_id: JSValueType::NativeObject as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取指定函数
    pub fn get_js_function(&self, func: String) -> bool {
        unsafe { if dukc_get_js_function(self.vm as *const c_void, CString::new(func).unwrap().as_ptr()) == 0 {
            return false;
        }}
        return true;
    }

    //调用指定函数
    pub fn call(&self, len: usize) {
        let vm = self.vm;
        unsafe {
            let status = dukc_vm_status_switch(vm as *const c_void, JSStatus::NoTask as i8, JSStatus::SingleTask as i8);
            if status == JSStatus::SingleTask as i8 {
                //当前虚拟机正在destroy或有任务
                println!("invalid vm status with call");
            } else {
                dukc_call(self.vm as *const c_void, len as u8, js_reply_callback);
            }
        }
    }

    //从异步消息队列中弹出消息
    pub fn pop(&self) -> Option<JSMsg> {
        match self.queue.consumer.try_pop() {
            Err(_) => None,
            Ok(msg) => {
                self.queue.size.fetch_sub(1, Ordering::Acquire);
                Some(msg)
            },
        }    
    }

    //向异步消息队列中推送消息
    pub fn push(&self, msg: JSMsg) -> usize {
        match self.queue.producer.try_push(msg) {
            Err(_) => 0,
            Ok(_) => self.queue.size.fetch_add(1, Ordering::Acquire) + 1,
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
* js类型
*/
#[derive(Clone)]
pub struct JSType {
    type_id:    u8,
    vm:         usize,
    value:      usize,
}

impl JSType {
    //构建一个指定js类型
    pub unsafe fn new(type_id: u8, vm: *const c_void, ptr: *const c_void) -> Self {
        JSType {
            type_id: type_id,
            vm: vm as usize,
            value: ptr as usize,
        }
    }

    //获取指定类型的类型id
    fn get_type_id(&self, value: u32) -> u8 {
        unsafe { dukc_get_value_type(self.vm as *const c_void, value) as u8 }
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
        unsafe { num = dukc_get_boolean(self.vm as *const c_void, self.value as u32) }
        if num == 0 {
            false
        } else {
            true
        }
    }

    //获取i8
    pub fn get_i8(&self) -> i8 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as i8 }
    }

    //获取i16
	pub fn get_i16(&self) -> i16 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as i16 }
    }

    //获取i32
	pub fn get_i32(&self) -> i32 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as i32 }
    }

    //获取i64
	pub fn get_i64(&self) -> i64 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as i64 }
    }

    //获取u8
	pub fn get_u8(&self) -> u8 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as u8 }
    }

    //获取u16
	pub fn get_u16(&self) -> u16 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as u16 }
    }

    //获取u32
	pub fn get_u32(&self) -> u32 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as u32 }
    }

    //获取u64
	pub fn get_u64(&self) -> u64 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as u64 }
    }

    //获取f32
	pub fn get_f32(&self) -> f32 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as f32 }
    }

    //获取f64
	pub fn get_f64(&self) -> f64 {
        unsafe { dukc_get_number(self.vm as *const c_void, self.value as u32) as f64 }
    }

    //获取字符串
	pub fn get_str(&self) -> String {
        unsafe { CStr::from_ptr(dukc_get_string(self.vm as *const c_void, self.value as u32)).to_string_lossy().into_owned() }
    }

    //获取对象指定域的值，注意获取的值在读取后需要立即调用dukc_remove_value函数移除掉
	pub fn get_field(&self, key: String) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_get_object_field(self.vm as *const c_void, self.value as u32, CString::new(key).unwrap().as_ptr()) }
        JSType {
            type_id: self.get_type_id(ptr),
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取数组长度
    pub fn get_array_length(&self) -> usize {
        unsafe { dukc_get_array_length(self.vm as *const c_void, self.value as u32) as usize }
    }

    //获取数组指定偏移的值，注意获取的值在读取后需要立即调用dukc_remove_value函数移除掉
	pub fn get_index(&self, index: u32) -> JSType {
        let ptr: u32;
        unsafe { ptr = dukc_get_array_index(self.vm as *const c_void, self.value as u32, index) }
        JSType {
            type_id: self.get_type_id(ptr),
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取指定Buffer的引用
    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            let length = dukc_get_buffer_length(self.vm as *const c_void, self.value as u32) as usize;
            let buffer = dukc_get_buffer(self.vm as *const c_void, self.value as u32);
            from_raw_parts(buffer as *const u8, length)
        }
    }

    //获取指定Buffer的引用
    pub unsafe fn to_bytes_mut(&mut self) -> &mut [u8] {
        let length = dukc_get_buffer_length(self.vm as *const c_void, self.value as u32) as usize;
        let buffer = dukc_get_buffer(self.vm as *const c_void, self.value as u32);
        from_raw_parts_mut(buffer as *mut u8, length)
    }

    //获取指定Buffer的复制
	pub fn into_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    //重置指定的Buffer
	pub fn from_bytes(&self, bytes: &[u8]) {
        unsafe {
            let length = dukc_get_buffer_length(self.vm as *const c_void, self.value as u32) as usize;
            let buffer = dukc_get_buffer(self.vm as *const c_void, self.value as u32);
            memcpy(buffer as *mut c_void, bytes.as_ptr() as *const c_void, length);
        }
    }

    //获取指定的Buffer
    pub fn into_buffer(&self) -> JSBuffer {
        unsafe {
            let length = dukc_get_buffer_length(self.vm as *const c_void, self.value as u32) as usize;
            let buffer = dukc_get_buffer(self.vm as *const c_void, self.value as u32);
            JSBuffer::new(buffer as *mut c_void, length)
        }
    }

    //获取NativeObject
	pub fn get_native_object(&self) -> usize {
        unsafe { dukc_get_native_object_instance(self.vm as *const c_void, self.value as u32) as usize }
    }
}

/*
* Js Buffer
*/
pub struct JSBuffer {
    buffer: *mut c_void,
    len: usize,
}

impl JSBuffer {
    //构建JSBuffer
    pub fn new(ptr: *mut c_void, len: usize) -> Self {
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
        unsafe { memcpy(self.buffer.wrapping_offset(offset as isize), v.as_ptr() as *const c_void, len); }
        last as isize
    }
}

/*
* 线程安全的执行异步消息队列中的回调函数
*/
#[inline]
fn callback(js: Arc<JS>, msg: JSMsg) {
    let info = msg.info.clone();
    let func = Box::new(move || {
        let args_len = (msg.args)(js.clone());
         println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!callback, args_len: {}", args_len);
        unsafe { dukc_call(js.get_vm(), args_len as u8, js_reply_callback); }
    });
    cast_js_task(TaskType::Async, 5000000000, func, info);
}

