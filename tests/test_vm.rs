#![feature(duration_extras)]

#[cfg(test)]
extern crate worker;
extern crate util;
extern crate atom;
extern crate task_pool;
extern crate timer;
extern crate handler;
extern crate pi_vm;
extern crate apm;

use std::mem;
use std::thread;
use std::ffi::CString;
use std::sync::atomic::AtomicUsize;
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex, Condvar};

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate env_logger;

extern crate rand;

use rand::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

use handler::{Env, GenType, Handler, Args};
use timer::TIMER;
use atom::Atom;
use task_pool::TaskPool;
use worker::task::TaskType;
use worker::worker::WorkerType;
use worker::worker_pool::WorkerPool;
use worker::impls::{TASK_POOL_TIMER, JS_WORKER_WALKER, JS_TASK_POOL, create_js_task_queue, lock_js_task_queue, unlock_js_task_queue, cast_js_task};
use pi_vm::pi_vm_impl::{VMFactory, block_reply, block_throw, push_callback, register_async_request};
use pi_vm::adapter::{load_lib_backtrace, register_native_object, dukc_remove_value, dukc_top, JS, JSType, set_vm_timeout};
use pi_vm::channel_map::VMChannel;
use pi_vm::proc::{Process, ProcInfo, ProcessFactory};
use apm::allocator::set_max_alloced_limit;
use pi_vm::bonmgr::{CallResult, NativeObjsAuth, FnMeta, BON_MGR};
use pi_vm::proc_pool::{set_factory, spawn_process, name_to_pid, set_receiver, set_catcher, close_process, pid_send, name_send};
use pi_vm::duk_proc::{DukProcess, DukProcessFactory};

// // #[test]
// fn njsc_test() {
//     load_lib_backtrace();
//     register_native_object();
//     njsc_test_main();
// }

// #[test]
fn test_vm_performance() {
    register_native_object();

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_clone_performance.js".to_string(), "function call(x, y, z) { var r = [0, 0, 0]; r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); };".to_string());
    assert!(opts.is_some());
    let codes = opts.unwrap();
    let time = Instant::now();
    for vm_id in 0..10000 {
        JS::new(vm_id, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None).unwrap().load(codes.as_slice());
    }
    let finish_time = time.elapsed();
    println!("!!!!!!load time: {}", finish_time.as_secs() * 1000000 + (finish_time.subsec_micros() as u64));

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_run_performance.js".to_string(), "var x = 0; for(var n = 0; n < 100000000; n++) { x++; }".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
    let time = Instant::now();
    js.run();
    let finish_time = time.elapsed();
    println!("!!!!!!run time: {}", finish_time.as_secs() * 1000000 + (finish_time.subsec_micros() as u64));
}

#[test]
fn base_test() {
    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    println!("js heap size: {}", js.heap_size());
    let opts = js.compile("base_test.js".to_string(), "var obj = {a: 10, c: true, d: {a: 0.9999999, c: \"ADSFkfaf中()**&^$111\", d: [new Uint8Array(), new ArrayBuffer(), function(x) { return x; }]}}; console.log(\"!!!!!!obj:\", obj);".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
    println!("js heap size: {}", js.heap_size());
    let val = js.new_null();
    assert!(val.is_null());
    let val = js.new_undefined();
    assert!(val.is_undefined());
    let val = js.new_boolean(true);
    assert!(val.is_boolean() && val.get_boolean());
    let val = js.new_boolean(false);
    assert!(val.is_boolean() && !val.get_boolean());
    let val = js.new_i8(0x7fi8);
    assert!(val.is_number() && val.get_i8() == 0x7fi8);
    let val = js.new_i16(0x7fffi16);
    assert!(val.is_number() && val.get_i16() == 0x7fffi16);
    let val = js.new_i32(0x7fffffffi32);
    assert!(val.is_number() && val.get_i32() == 0x7fffffffi32);
    let val = js.new_i64(0x7199254740992i64);
    assert!(val.is_number() && val.get_i64() == 0x7199254740992i64);
    let val = js.new_u8(255u8);
    assert!(val.is_number() && val.get_u8() == 255u8);
    let val = js.new_u16(65535u16);
    assert!(val.is_number() && val.get_u16() == 65535u16);
    let val = js.new_u32(0xffffffffu32);
    assert!(val.is_number() && val.get_u32() == 0xffffffffu32);
    let val = js.new_u64(9007199254740992u64);
    assert!(val.is_number() && val.get_u64() == 9007199254740992u64);
    let val = js.new_f32(0.0173136f32);
    assert!(val.is_number() && val.get_f32() == 0.0173136f32);
    let val = js.new_f64(921.1356737853f64);
    assert!(val.is_number() && val.get_f64() == 921.1356737853f64);

    let val = js.new_str("Hello World".to_string()).unwrap();
    assert!(val.is_string() && val.get_str() == "Hello World".to_string());
    let val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string()).unwrap();
    assert!(val.is_string() && val.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());

    let object = js.new_object();
    assert!(object.is_object());
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string()).unwrap();
    js.set_field(&object, "x".to_string(), &mut val);
    {
        let tmp = object.get_field("x".to_string());
        assert!(object.is_object() && tmp.is_string() && tmp.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    }
    {
        let tmp = object.get_field("c".to_string());
        assert!(object.is_object() && tmp.is_none()); //key不存在
    }

    assert!(js.get_type("Array".to_string()));
    js.new_u8(10);
    let array = js.new_type("Array".to_string(), 1);
    assert!(array.is_array() && array.get_array_length() == 10);
    let mut object = js.new_object();
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string()).unwrap();
    js.set_field(&object, "x".to_string(), &mut val);
    js.set_index(&array, 3, &mut object);
    assert!(js.set_global_var("$array".to_string(), array));

    {
        let val = js.eval("var _obj = {};_obj;".to_string());
        assert!(val.is_object());   
    }

    {
        assert!(js.check_function("Math.log10".to_string()));
        let n = js.new_u16(1000);
        let val = js.invoke(1);
        assert!(val.is_number());
    }

    let array = js.new_array();
    assert!(array.is_array() && array.get_array_length() == 0);
    let mut object = js.new_object();
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string()).unwrap();
    js.set_field(&object, "x".to_string(), &mut val);
    js.set_index(&array, 3, &mut object);
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string()).unwrap();
    js.set_index(&array, 30, &mut val); //数组自动扩容
    {
        let tmp = array.get_index(3);
        assert!(array.is_array() && tmp.is_object() && tmp.get_field("x".to_string()).get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    }
    {
        let tmp = array.get_index(30);
        assert!(array.is_array() && tmp.is_string() && tmp.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    }
    {
        let tmp = array.get_index(0);
        assert!(array.is_array() && tmp.is_none()); //index不存在
    }

    let val = js.new_array_buffer(32);
    let mut tmp = val.into_vec();
    assert!(val.is_array_buffer() && tmp.capacity() == 32 && tmp.len() == 32);
    println!("buffer: {:?}", tmp);
    for i in 0..tmp.len() {
        tmp[i] = 10;
    }
    val.from_bytes(tmp.as_slice());
    let tmp = val.to_bytes();
    assert!(val.is_array_buffer() && tmp.len() == 32);
    println!("buffer: {:?}", tmp);
    let mut tmp = val.into_buffer();
    assert!(val.is_array_buffer() && tmp.len() == 32);
    tmp.write_i8(0, 0x7f);
    assert!(tmp.read_i8(0) == 0x7f);
    tmp.write_i16(1, 0x7fff);
    assert!(tmp.read_i16(1) == 0x7fff);
    tmp.write_i32(3, 0x7fffffff);
    assert!(tmp.read_i32(3) == 0x7fffffff);
    tmp.write_i64(7, 0x7fffffffffffffff);
    assert!(tmp.read_i64(7) == 0x7fffffffffffffff);
    tmp.write_u8(15, 0xff);
    assert!(tmp.read_u8(15) == 0xff);
    tmp.write_u16(16, 0xffff);
    assert!(tmp.read_u16(16) == 0xffff);
    tmp.write_u32(18, 0xffffffff);
    assert!(tmp.read_u32(18) == 0xffffffff);
    tmp.write_u64(22, 0xffffffffffffffff);
    assert!(tmp.read_u64(22) == 0xffffffffffffffff);
    tmp.write_f32(18, 0.7891312);
    assert!(tmp.read_f32(18) == 0.7891312);
    tmp.write_f64(22, 0.999999999999);
    assert!(tmp.read_f64(22) == 0.999999999999);
    println!("buffer: {:?}", tmp.read(0, 32));
    tmp.write_i8_be(0, 0x7f);
    assert!(tmp.read_i8_be(0) == 0x7f);
    tmp.write_i16_be(1, 0x7fff);
    assert!(tmp.read_i16_be(1) == 0x7fff);
    tmp.write_i32_be(3, 0x7fffffff);
    assert!(tmp.read_i32_be(3) == 0x7fffffff);
    tmp.write_i64_be(7, 0x7fffffffffffffff);
    assert!(tmp.read_i64_be(7) == 0x7fffffffffffffff);
    tmp.write_u8_be(15, 0xff);
    assert!(tmp.read_u8_be(15) == 0xff);
    tmp.write_u16_be(16, 0xffff);
    assert!(tmp.read_u16_be(16) == 0xffff);
    tmp.write_u32_be(18, 0xffffffff);
    assert!(tmp.read_u32_be(18) == 0xffffffff);
    tmp.write_u64_be(22, 0xffffffffffffffff);
    assert!(tmp.read_u64_be(22) == 0xffffffffffffffff);
    tmp.write_f32_be(18, 0.7891312);
    assert!(tmp.read_f32_be(18) == 0.7891312);
    tmp.write_f64_be(22, 0.999999999999);
    assert!(tmp.read_f64_be(22) == 0.999999999999);
    println!("buffer: {:?}", tmp.read(0, 32));
    tmp.write(0, &[100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100]);
    println!("buffer: {:?}", tmp.read(0, 32));

    let mut val = js.new_uint8_array(10);
    assert!(val.is_uint8_array());
    {
        let tmp = unsafe { val.to_bytes_mut() };
        assert!(tmp.len() == 10);
        println!("buffer: {:?}", tmp);
        for i in 0..tmp.len() {
            tmp[i] = 255;
        }
    }
    let tmp = val.to_bytes();
    assert!(val.is_uint8_array() && tmp.len() == 10);
    println!("buffer: {:?}", tmp);

    let val = js.new_native_object(0xffffffffusize);
    println!("stack: {}", js.dump_stack());
    assert!(val.is_native_object() && val.get_native_object() == 0xffffffffusize);
    println!("js heap size: {}", js.heap_size());
}

//测试从虚拟机工厂进行虚拟机js执行
#[test]
fn test_vm_factory() {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    TIMER.run();
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    set_max_alloced_limit(1073741824);
    set_vm_timeout(30000);

    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "var tmp = 0; function call(x, y) { var buf = undefined; if (buf != undefined) { console.log(\"buf len:\", buf.byteLength); throw(new Error(\"invalid global\")); } buf = new ArrayBuffer(256 * 1024 * 1024); console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", y length: \" + y.length + \", tmp: \" + tmp); tmp += 1; throw(\"test call throw\"); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    //要测试虚拟机复用，需要将factory capacity设置为大于0，且produce生成的虚拟机数量应该大于0
    //如果需要测试虚拟机不复用，需要将factory capacity和produce都设置为0
    let factory = VMFactory::new("test vm", 3, 27, 1073741824, 1073741824, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(3) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            println!("!!!!!!factory vm len: {:?}", len);
            let now = Instant::now();
            for _ in 0..32 {
                let func = Box::new(move |js: Arc<JS>| {
                    js.new_str("Hello World".to_string()).unwrap();
                    js.new_f32(0.999999);
                    2usize
                });
                factory.call(None,
                             Atom::from("call"),
                             func,
                             Atom::from("test factory call task"));
                thread::sleep(Duration::from_millis(1000));
            }
            println!("!!!!!!time: {:?}", Instant::now() - now);
        },
    }
    thread::sleep(Duration::from_millis(100000));
}

//注册本地函数
fn register_native_function(id: u32, fun: fn(Arc<JS>, Vec<JSType>) -> Option<CallResult>) {
    BON_MGR.regist_fun_meta(FnMeta::CallArg(fun), id);
}

//测试从虚拟机工厂进行虚拟机同步调用
#[test]
fn test_vm_factory_sync_call() {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    TIMER.run();
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    set_max_alloced_limit(1073741824);
    set_vm_timeout(30000);

    //初始化同步调用的环境
    register_native_object();
    register_native_function(0x1, js_test_vm_factory_sync_call);

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "function call(x, y) { var buf = undefined; if (buf != undefined) { console.log(\"buf len:\", buf.byteLength); throw(new Error(\"invalid global\")); } buf = new ArrayBuffer(256 * 1024 * 1024); var r = NativeObject.call(0x1, [true, 10, \"Hello World!\"]); console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", r: \" + r); throw(\"test sync throw\"); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    //要测试虚拟机复用，需要将factory capacity设置为大于0，且produce生成的虚拟机数量应该大于0
    //如果需要测试虚拟机不复用，需要将factory capacity和produce都设置为0
    let factory = VMFactory::new("test vm", 3, 27, 1073741824, 1073741824, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(3) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            println!("!!!!!!factory vm len: {:?}", len);
            let now = Instant::now();
            for _ in 0..32 {
                let func = Box::new(move |js: Arc<JS>| {
                    js.new_str("Hello World".to_string()).unwrap();
                    js.new_f32(0.999999);
                    2usize
                });
                factory.call(None,
                             Atom::from("call"),
                             func,
                             Atom::from("test factory call task"));
                thread::sleep(Duration::from_millis(1000));
            }
            println!("!!!!!!time: {:?}", Instant::now() - now);
        },
    }
    thread::sleep(Duration::from_millis(100000));
}

fn js_test_vm_factory_sync_call(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    js.new_u32(0xffffffff);
    Some(CallResult::Ok)
}

//测试从虚拟机工厂进行虚拟机阻塞调用
#[test]
fn test_vm_factory_block_call() {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    TIMER.run();
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    set_max_alloced_limit(1073741824);
    set_vm_timeout(30000);

    //初始化阻塞调用的环境
    register_native_object();
    register_native_function(0x1, js_test_vm_factory_block_call);
    register_native_function(0x10, js_test_vm_factory_block_throw);

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "function call(x, y) { var buf = undefined; if (buf != undefined) { console.log(\"buf len:\", buf.byteLength); throw(new Error(\"invalid global\")); } buf = new ArrayBuffer(256 * 1024 * 1024); NativeObject.call(0x1, [true, 10, \"Hello World!\"]); var r = __thread_yield(); console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", r: \" + r); NativeObject.call(0x10, [10]); r = __thread_yield(); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    //要测试虚拟机复用，需要将factory capacity设置为大于0，且produce生成的虚拟机数量应该大于0
    //如果需要测试虚拟机不复用，需要将factory capacity和produce都设置为0
    let factory = VMFactory::new("test vm", 3, 27, 1073741824, 1073741824, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(3) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            println!("!!!!!!factory vm len: {:?}", len);
            let now = Instant::now();
            for _ in 0..32 {
                let func = Box::new(move |js: Arc<JS>| {
                    js.new_str("Hello World".to_string()).unwrap();
                    js.new_f32(0.999999);
                    2usize
                });
                factory.call(None,
                             Atom::from("call"),
                             func,
                             Atom::from("test factory call task"));
                thread::sleep(Duration::from_millis(1000));
            }
            println!("!!!!!!time: {:?}", Instant::now() - now);
        },
    }
    thread::sleep(Duration::from_millis(100000));
}

fn js_test_vm_factory_block_call(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    let result = Box::new(|vm: Arc<JS>| {
        vm.new_u32(0xffffffff);
    });
    block_reply(js, result, Atom::from("block reply task"));
    None
}

fn js_test_vm_factory_block_throw(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    block_throw(js, "test block throw".to_string(), Atom::from("block throw task"));
    None
}

//测试从虚拟机工厂进行虚拟机异步回调
#[test]
fn test_vm_factory_async_callback() {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    TIMER.run();
    TASK_POOL_TIMER.run();
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    set_max_alloced_limit(1073741824);
    set_vm_timeout(30000);

    //初始化阻塞调用的环境
    register_native_object();
    register_native_function(0x1, js_test_vm_factory_block_call);
    register_native_function(0x10, js_test_vm_factory_block_throw);
    register_native_function(0x100, js_async_callback_register_push);

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "function callback(x, y, z) { console.log(\"!!!!!!async callback ok, x:\", x, \", y:\", y, \", z:\", z); NativeObject.call(0x1, [true, 10, \"Hello Callback!\"]); var r = __thread_yield(); console.log(\"!!!!!!block call in callback, result:\", r); NativeObject.call(0x10, [10]); r = __thread_yield(); } function call(x, y) { var buf = undefined; if (buf != undefined) { console.log(\"buf len:\", buf.byteLength); throw(new Error(\"invalid global\")); } buf = new ArrayBuffer(256 * 1024 * 1024); var index = callbacks.register(callback); var handle = NativeObject.call(0x100, [index, 1000]); console.log(\"!!!!!!async callback index:\", index, \", handle:\", handle); NativeObject.call(0x1, [true, 10, \"Hello World!\"]); var r = __thread_yield(); console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", r: \" + r); NativeObject.call(0x10, [10]); r = __thread_yield(); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    //要测试虚拟机复用，需要将factory capacity设置为大于0，且produce生成的虚拟机数量应该大于0
    //如果需要测试虚拟机不复用，需要将factory capacity和produce都设置为0
    let factory = VMFactory::new("test vm", 3, 27, 1073741824, 1073741824, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(3) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            println!("!!!!!!factory vm len: {:?}", len);
            let now = Instant::now();
            for _ in 0..32 {
                let func = Box::new(move |js: Arc<JS>| {
                    js.new_str("Hello World".to_string()).unwrap();
                    js.new_f32(0.999999);
                    2usize
                });
                factory.call(None,
                             Atom::from("call"),
                             func,
                             Atom::from("test factory call task"));
                thread::sleep(Duration::from_millis(2000));
            }
            println!("!!!!!!time: {:?}", Instant::now() - now);
        },
    }
    thread::sleep(Duration::from_millis(100000));
}

fn js_async_callback_register_push(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let callback = args[0].get_u32();
    let timeout = args[1].get_u32();

    let func = Box::new(move |vm: Arc<JS>| -> usize {
        vm.new_u32(callback);
        vm.new_u32(callback);
        vm.new_u32(callback);
        3
    });
    if let Some(handle) = push_callback(js.clone(), args[0].get_u32(), func, Some(timeout), Atom::from("register callback task")) {
        js.new_i32(handle as i32);
        Some(CallResult::Ok)
    } else {
        Some(CallResult::Err("set timeout failed".to_string()))
    }
}

//测试虚拟机同步加载模块
#[test]
fn test_vm_sync_load_mod() {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    TIMER.run();
    TASK_POOL_TIMER.run();
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    set_max_alloced_limit(1073741824);
    set_vm_timeout(30000);

    //初始化同步调用的环境
    register_native_object();
    register_native_function(0x1, js_test_vm_sync_load_mod);

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_load_mod.js".to_string(), "console.log(\"loading module...\"); var wait_load = NativeObject.call(0x1, [\"./test/mod\"]); var loaded = wait_load({}); console.log(\"load module ok, loaded:\", loaded); var mod0_test0 = loaded.test0(); console.log(\"bind module function ok, function:\", mod0_test0); x = 10000000000; y = 999999999; function test_call() { console.log(\"!!!!!!local:\", mod0_test0); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    let factory = VMFactory::new("test vm", 3, 27, 1073741824, 1073741824, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(1) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            let func = Box::new(move |js: Arc<JS>| {
                0usize
            });
            factory.call(None,
                         Atom::from("test_call"),
                         func,
                         Atom::from("test sync load module task"));
        },
    }
    thread::sleep(Duration::from_millis(100000));
}

fn js_test_vm_sync_load_mod(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    //为了保证模块的封装函数，可以是匿名的，且不绑定到全局环境中，需要用括号将封装函数括起来
    let opts = opts.unwrap().compile("test_mod_0.js".to_string(), "(function(exports) { mod0_num = 0xffffffff; var x = 1000; exports.test0 = function() { console.log(\"!!!!!!mod0.test0 called, mod0 x:\", x); }; return exports; })".to_string());
    let codes = opts.unwrap();

    if !js.load_module(codes.as_slice()) {
        //加载失败，则返回undefined
        js.new_undefined();
    }
    Some(CallResult::Ok)
}

//测试虚拟机异步加载模块
#[test]
fn test_vm_async_load_mod() {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    TIMER.run();
    TASK_POOL_TIMER.run();
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    set_max_alloced_limit(1073741824);
    set_vm_timeout(30000);

    //初始化异步调用的环境
    register_native_object();
    register_native_function(0x1, js_test_vm_async_load_mod);

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_load_mod.js".to_string(), "function onload(path, wait_load) { var loaded = wait_load({}); console.log(\"load module ok,\", path, \", \", loaded); var mod0_test0 = loaded.test0(); console.log(\"bind module function ok, function:\", mod0_test0); x = 10000000000; y = 999999999; function test_call() { console.log(\"!!!!!!local:\", mod0_test0); }; }; var index = callbacks.register(onload); NativeObject.call(0x1, [index, \"./test/mod\"]); ".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    let factory = VMFactory::new("test vm", 3, 27, 1073741824, 1073741824, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(1) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            let func = Box::new(move |js: Arc<JS>| {
                0usize
            });
            factory.call(None,
                         Atom::from("test_call"),
                         func,
                         Atom::from("test async load module task"));
        },
    }
    thread::sleep(Duration::from_millis(100000));
}

fn js_test_vm_async_load_mod(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let callback = args[0].get_u32();
    let path = args[1].get_str();
    let js_copy = js.clone();

    //用线程模拟异步加载
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(1000));

        let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
        //为了保证模块的封装函数，可以是匿名的，且不绑定到全局环境中，需要用括号将封装函数括起来
        let opts = opts.unwrap().compile("test_mod_0.js".to_string(), "(function(exports) { mod0_num = 0xffffffff; var x = 1000; exports.test0 = function() { console.log(\"!!!!!!mod0.test0 called, mod0 x:\", x); }; return exports; })".to_string());
        let codes = opts.unwrap();

        let func = Box::new(move |vm: Arc<JS>| -> usize {
            vm.new_str(path);
            if !vm.load_module(codes.as_slice()) {
                //加载失败，则返回undefined
                vm.new_undefined();
            }
            2
        });
        push_callback(js_copy.clone(), callback, func, None, Atom::from("register async load module callback task"));
    });

    js.new_undefined();
    Some(CallResult::Ok)
}

#[test]
fn test_process() {
    env_logger::builder()
        .format_timestamp_millis()
        .init();
    TIMER.run();
    TASK_POOL_TIMER.run();
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    set_max_alloced_limit(1073741824);
    set_vm_timeout(30000);

    //初始化进程的环境
    let auth = Arc::new(NativeObjsAuth::new(None, None));
    let opts = JS::new(1, Atom::from("test vm"), auth.clone(), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_process_base.js".to_string(), "start = function(b, x, y, str, bin, natobj) { onmessage = function(src, b, x, y, str, bin, natobj) { if(x > 3 && x < 5) { throw(new Error(\"test throw\")); } console.log(\"receive ok, src:\", src + \", b:\" + b + \", x:\" + x + \", y:\" + y + \", str:\" + str + \", bin:\" + bin + \", natobj:\" + natobj); }; onerror = function(e) { console.log(\"process handle error, e:\", e); }; var index0 = callbacks.register(onmessage); var r0 = NativeObject.call(0x10, [_$pid, index0]); var index1 = callbacks.register(onerror); var r1 = NativeObject.call(0x100, [_$pid, index1]); console.log(\"register onmessage and onerror ok\"); console.log(\"start process ok, b:\" + b + \", x:\" + x + \", y:\" + y + \", str:\" + str + \", bin:\" + bin + \", natobj:\" + natobj); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    let duk_facotry_name = Atom::from("duk_proc_factory");
    let duk_factory = DukProcessFactory::new(duk_facotry_name.clone(), auth.clone(), Arc::new(vec![code]));
    set_factory(duk_facotry_name.clone(), Arc::new(duk_factory));

    //初始化同步调用的环境
    register_native_object();
    register_native_function(0x1, js_test_process_spawn);
    register_native_function(0x10, js_test_process_register_receiver);
    register_native_function(0x100, js_test_process_register_catcher);
    register_native_function(0x1000, js_test_process_send);
    register_native_function(0x10000, js_test_process_close);

    let opts = JS::new(1, Atom::from("test vm"), auth, None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_process.js".to_string(), "var pid = NativeObject.call(0x1, [\"duk_proc_factory\", \"test_process\", \"handler\", \"start\", \"start\", [true, 0xffffffff, 9.9999999, \"Hello Process\", new Uint8Array([97, 97, 97])]]); console.log(\"spawn process, pid:\", pid); function test_call() { for(var i = 0; i < 10; i++) { var r = NativeObject.call(0x1000, [pid, [true, i, 9.9999999, \"Hello Process\", new Uint8Array([97, 97, 97])]]); console.log(\"send msg to process, i: \" + i + \", r:\", r); } NativeObject.call(0x10000, [pid]); console.log(\"close process, pid:\", pid); }".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    let factory = VMFactory::new("test vm", 3, 27, 1073741824, 1073741824, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(1) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            let func = Box::new(move |js: Arc<JS>| {
                0usize
            });
            factory.call(None,
                         Atom::from("test_call"),
                         func,
                         Atom::from("test sync load module task"));
        },
    }
    thread::sleep(Duration::from_millis(100000));
}

fn js_test_process_spawn(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let factory_name = args[0].get_str();
    let process_name = args[1].get_str();
    let module = args[2].get_str();
    let function = args[3].get_str();
    let init = args[4].get_str();
    let array = &args[5];
    let b = array.get_index(0).get_boolean();
    let x = array.get_index(1).get_f64();
    let y = array.get_index(2).get_f64();
    let str = array.get_index(3).get_str();
    let bin = array.get_index(4).into_vec();

    let natobj = GenType::Array(vec![GenType::USize(0xffff), GenType::USize(0xffff)]);
    let args1 = GenType::Array(vec![GenType::Bool(b), GenType::F64(x), GenType::F64(y), GenType::Str(str), GenType::Bin(bin), natobj]);

    match spawn_process(Some(process_name), Atom::from(factory_name), module, function, init, args1) {
        Err(e) => {
            Some(CallResult::Err(e.to_string()))
        },
        Ok(pid) => {
            js.new_u32(pid as u32);
            Some(CallResult::Ok)
        },
    }
}

fn js_test_process_register_receiver(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let pid = args[0].get_u32() as u64;
    let callback = args[1].get_u32();

    println!("!!!!!!pid: {}, callback: {}", pid, callback);
    if let Err(e) = set_receiver(pid, GenType::U32(callback)) {
        return Some(CallResult::Err(e.to_string()));
    }

    js.new_undefined();
    Some(CallResult::Ok)
}

fn js_test_process_register_catcher(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let pid = args[0].get_u32() as u64;
    let callback = args[1].get_u32();

    if let Err(e) = set_catcher(pid, GenType::U32(callback)) {
        return Some(CallResult::Err(e.to_string()));
    }

    js.new_undefined();
    Some(CallResult::Ok)
}

fn js_test_process_send(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let pid = args[0].get_u32() as u64;
    let array = &args[1];
    let b = array.get_index(0).get_boolean();
    let x = array.get_index(1).get_f64();
    let y = array.get_index(2).get_f64();
    let str = array.get_index(3).get_str();
    let bin = array.get_index(4).into_vec();

    let natobj = GenType::Array(vec![GenType::USize(0xffff), GenType::USize(0xffff)]);
    let args1 = GenType::Array(vec![GenType::Bool(b), GenType::F64(x), GenType::F64(y), GenType::Str(str), GenType::Bin(bin), natobj]);

    if let Err(e) = pid_send(0, pid, args1) {
        return Some(CallResult::Err(e.to_string()));
    }

    js.new_undefined();
    Some(CallResult::Ok)
}

fn js_test_process_close(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let pid = args[0].get_u32() as u64;

    if let Err(e) = close_process(pid, "normal".to_string()) {
        return Some(CallResult::Err(e.to_string()));
    }

    js.new_undefined();
    Some(CallResult::Ok)
}

#[test]
fn test_vm_collect() {
    let mut rng = SmallRng::from_entropy();

    let mut x = 0;
    let mut y = 0;
    let r = 0.5f64.powf(8.0).powf(0.9);
    for _ in 0..10000 {
        if rng.gen_bool(r) {
            x += 1;
        } else {
            y += 1;
        }
    }
    println!("!!!!!!r: {}, x: {}, y: {}", r, x, y);
}

// #[test]
fn test_stack_length() {
    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("base_test.js".to_string(), "var obj = {a: 10, c: true, d: {a: 0.9999999, c: \"ADSFkfaf中()**&^$111\", d: [new Uint8Array(), new ArrayBuffer(), function(x) { return x; }]}}; console.log(\"!!!!!!obj:\", obj);".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));

    let mut member: JSType;
    let array = js.new_array();
    assert!(array.is_array() && array.get_array_length() == 0);
    for idx in 0..10 {
        member = js.new_u8(idx as u8);
        js.set_index(&array, idx, &mut member);
    }

    unsafe {
        for idx in 0..10 {
            let n = array.get_index(idx as u32);
            assert!(n.is_number());
        }
        println!("!!!!!!top: {}", dukc_top(js.get_vm()));
    }

    //out of stack
    unsafe {
        let mut arr: [JSType; 200] = unsafe { mem::zeroed() };
        for idx in 0..200 {
            arr[idx] = array.get_index(idx as u32);
            assert!(arr[idx].is_number());
            println!("!!!!!!top: {}", dukc_top(js.get_vm()));
        }
    }
}

// #[test]
fn test_js_string() {
    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_string.js".to_string(), "console.log(\"!!!!!!string: \" + \"你好!!!!!!\".length); var view = (new TextEncoder()).encode(\"你好!!!!!!\"); console.log(\"!!!!!!view: \" + view); var r = NativeObject.call(0xffffffff, [view]); console.log(\"!!!!!!r: \" + r); console.log(\"!!!!!!string: \" + (new TextDecoder()).decode(view));".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
}

// #[test]
fn test_js_this() {
    load_lib_backtrace();
    register_native_object();

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this0.js".to_string(), "var obj = {}; function call() { console.log(\"!!!!!!obj: \" + obj); obj.a = 100; var a = 10; console.log(\"!!!!!!obj.a: \" + obj.a + \", a: \" + a); obj.func = function call0() { console.log(\"!!!!!!this.a: \" + this.a); }; obj.func();}; call();".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this1.js".to_string(), "var obj = {str: \"Hello\", func: function() { console.log(\"!!!!!!this.str: \" + this.str); this.str = 10; console.log(\"!!!!!!this.str: \" + this.str); } }; obj.func();".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this2.js".to_string(), "var obj = {x: 10, y: { func: function() { console.log(\"!!!!!!this.x: \" + this.x); this.x = \"Hello\"; console.log(\"!!!!!!this.x: \" + this.x); } } }; obj.y.func();".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this3.js".to_string(), "var obj = {name : 'linxin'}; function func(firstName, lastName) { console.log(firstName + ' ' + this.name + ' ' + lastName); } func.apply(obj, ['A', 'B']);".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
}



