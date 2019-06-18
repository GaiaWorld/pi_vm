#![feature(duration_extras)]

#[cfg(test)]
extern crate worker;
extern crate util;
extern crate atom;
extern crate task_pool;
extern crate handler;
extern crate pi_vm;

use std::mem;
use std::thread;
use std::sync::atomic::AtomicUsize;
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex, Condvar};

#[macro_use]
extern crate lazy_static;

use handler::{Env, GenType, Handler, Args};
use worker::task::TaskType;
use worker::worker::WorkerType;
use atom::Atom;
use task_pool::TaskPool;
use worker::worker_pool::WorkerPool;
use worker::impls::{JS_WORKER_WALKER, JS_TASK_POOL, create_js_task_queue, lock_js_task_queue, unlock_js_task_queue, cast_js_task};
use pi_vm::pi_vm_impl::{VMFactory, block_reply, block_throw, push_callback, register_async_request};
use pi_vm::adapter::{load_lib_backtrace, register_native_object, dukc_remove_value, dukc_top, JS, JSType};
use pi_vm::channel_map::VMChannel;
use pi_vm::bonmgr::{CallResult, NativeObjsAuth, FnMeta, BON_MGR};

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

    let val = js.new_str("Hello World".to_string());
    assert!(val.is_string() && val.get_str() == "Hello World".to_string());
    let val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    assert!(val.is_string() && val.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());

    let object = js.new_object();
    assert!(object.is_object());
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
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
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
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
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    js.set_field(&object, "x".to_string(), &mut val);
    js.set_index(&array, 3, &mut object);
    let mut val = js.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
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
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "var buf = undefined; var tmp = 0; function call(x, y) { buf = new ArrayBuffer(256 * 1024 * 1024); console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", y length: \" + y.length + \", tmp: \" + tmp); tmp += 1; throw(\"test call throw\"); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    //要测试虚拟机复用，需要将factory capacity设置为大于0，且produce生成的虚拟机数量应该大于0
    //如果需要测试虚拟机不复用，需要将factory capacity和produce都设置为0
    let factory = VMFactory::new("test vm", 3, 2, 536870912, 536870912, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(3) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            println!("!!!!!!factory vm len: {:?}", len);
            let now = Instant::now();
            for _ in 0..30 {
                let func = Box::new(move |js: Arc<JS>| {
                    js.new_str("Hello World".to_string());
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
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    //初始化同步调用的环境
    register_native_object();
    register_native_function(0x1, js_test_vm_factory_sync_call);

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "var buf = undefined; function call(x, y) { buf = new ArrayBuffer(256 * 1024 * 1024); var r = NativeObject.call(0x1, [true, 10, \"Hello World!\"]); console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", r: \" + r); throw(\"test sync throw\"); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    //要测试虚拟机复用，需要将factory capacity设置为大于0，且produce生成的虚拟机数量应该大于0
    //如果需要测试虚拟机不复用，需要将factory capacity和produce都设置为0
    let factory = VMFactory::new("test vm", 3, 2, 536870912, 536870912, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(3) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            println!("!!!!!!factory vm len: {:?}", len);
            let now = Instant::now();
            for _ in 0..30 {
                let func = Box::new(move |js: Arc<JS>| {
                    js.new_str("Hello World".to_string());
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
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 8, 1024 * 1024, 30000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    //初始化阻塞调用的环境
    register_native_object();
    register_native_function(0x1, js_test_vm_factory_block_call);
    register_native_function(0x10, js_test_vm_factory_block_throw);

    let opts = JS::new(1, Atom::from("test vm"), Arc::new(NativeObjsAuth::new(None, None)), None);
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "var buf = undefined; function call(x, y) { buf = new ArrayBuffer(256 * 1024 * 1024); NativeObject.call(0x1, [true, 10, \"Hello World!\"]); var r = __thread_yield(); console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", r: \" + r); NativeObject.call(0x10, [10]); r = __thread_yield(); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    //要测试虚拟机复用，需要将factory capacity设置为大于0，且produce生成的虚拟机数量应该大于0
    //如果需要测试虚拟机不复用，需要将factory capacity和produce都设置为0
    let factory = VMFactory::new("test vm", 3, 2, 536870912, 536870912, Arc::new(NativeObjsAuth::new(None, None)));
    let factory = factory.append(Arc::new(code));
    match factory.produce(3) {
        Err(e) => println!("factory produce failed, e: {:?}", e),
        Ok(len) => {
            println!("!!!!!!factory vm len: {:?}", len);
            let now = Instant::now();
            for _ in 0..30 {
                let func = Box::new(move |js: Arc<JS>| {
                    js.new_str("Hello World".to_string());
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



