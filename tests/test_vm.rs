#![feature(duration_extras)]

#[cfg(test)]
extern crate threadpool;
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

use handler::{Env, GenType, Handler, Args};
use worker::task::TaskType;
use worker::worker::WorkerType;
use atom::Atom;
use task_pool::TaskPool;
use util::now_nanosecond;
use worker::worker_pool::WorkerPool;
use worker::impls::{JS_WORKER_WALKER, JS_TASK_POOL, create_js_task_queue, lock_js_task_queue, unlock_js_task_queue, cast_js_task};
use pi_vm::pi_vm_impl::{VMFactory, block_reply, block_throw, push_callback, register_async_request};
use pi_vm::adapter::{load_lib_backtrace, register_native_object, dukc_remove_value, dukc_top, JS, JSType};
use pi_vm::channel_map::VMChannel;
use pi_vm::bonmgr::NativeObjsAuth;

// // #[test]
// fn njsc_test() {
//     load_lib_backtrace();
//     register_native_object();
//     njsc_test_main();
// }

// #[test]
fn test_vm_performance() {
    register_native_object();

    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_clone_performance.js".to_string(), "function call(x, y, z) { var r = [0, 0, 0]; r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); };".to_string());
    assert!(opts.is_some());
    let codes = opts.unwrap();
    let time = Instant::now();
    for _ in 0..10000 {
        JS::new(Arc::new(NativeObjsAuth::new(None, None))).unwrap().load(codes.as_slice());
    }
    let finish_time = time.elapsed();
    println!("!!!!!!load time: {}", finish_time.as_secs() * 1000000 + (finish_time.subsec_micros() as u64));

    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
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
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
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

// #[test]
fn test_stack_length() {
    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
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
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
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

    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this0.js".to_string(), "var obj = {}; function call() { console.log(\"!!!!!!obj: \" + obj); obj.a = 100; var a = 10; console.log(\"!!!!!!obj.a: \" + obj.a + \", a: \" + a); obj.func = function call0() { console.log(\"!!!!!!this.a: \" + this.a); }; obj.func();}; call();".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));

    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this1.js".to_string(), "var obj = {str: \"Hello\", func: function() { console.log(\"!!!!!!this.str: \" + this.str); this.str = 10; console.log(\"!!!!!!this.str: \" + this.str); } }; obj.func();".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));

    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this2.js".to_string(), "var obj = {x: 10, y: { func: function() { console.log(\"!!!!!!this.x: \" + this.x); this.x = \"Hello\"; console.log(\"!!!!!!this.x: \" + this.x); } } }; obj.y.func();".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));

    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_js_this3.js".to_string(), "var obj = {name : 'linxin'}; function func(firstName, lastName) { console.log(firstName + ' ' + this.name + ' ' + lastName); } func.apply(obj, ['A', 'B']);".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
}

#[test]
fn native_object_call_test() {
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 3, 1024 * 1024, 1000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("native_object_call_test.js".to_string(), "var obj = {};\n
     console.log(\"!!!!!!obj: \" + obj);\n
     function callback(index) {\n
        console.log(\"!!!!!!callback ok, index: \" + index);\n
        var r = NativeObject.call(0xffffffff, [index]);\n
        console.log(\"!!!!!!callback ok, r: \" + r);\n
     };\n
     function async_call(func) {\n
        var index = callbacks.register(func);\n
        NativeObject.call(index, []);\n
        console.log(\"!!!!!!register callback ok, index: \" + index);\n
     };\n
     function call(x, y, z) {\n
        async_call(callback);\n
        var r = [0, 0, 0];\n
        r = NativeObject.call(0xffffffff, [x, y, z]);\n
        console.log(\"!!!!!!call ok, r: \" + r);\n
        async_call(callback);\n
    };".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }

    js.get_js_function("call".to_string());
    js.new_boolean(false);
    js.new_u64(0xfffffffffff);
    js.new_str("Hello World!!!!!!".to_string());
    js.call(3);
    thread::sleep(Duration::from_millis(1000));
    let args = Box::new(move |tmp: Arc<JS>| {
        tmp.new_u32(0);
        1
    });
    push_callback(js.clone(), 0, args, Atom::from("callback by sync call"));
    thread::sleep(Duration::from_millis(1000));
    let args = Box::new(move |tmp: Arc<JS>| {
        tmp.new_u32(1);
        1
    });
    push_callback(js.clone(), 1, args, Atom::from("callback by sync call"));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }
    
    js.get_js_function("call".to_string());
    js.new_boolean(false);
    js.new_u64(0xfffffffffff);
    js.new_str("Hello World!!!!!!".to_string());
    js.call(3);
    thread::sleep(Duration::from_millis(1000));
    let args = Box::new(move |tmp: Arc<JS>| {
        tmp.new_u32(2);
        1
    });
    push_callback(js.clone(), 0, args, Atom::from("callback by sync call"));
    thread::sleep(Duration::from_millis(1000));
    let args = Box::new(move |tmp: Arc<JS>| {
        tmp.new_u32(3);
        1
    });
    push_callback(js.clone(), 1, args, Atom::from("callback by sync call"));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }
    
    js.get_js_function("call".to_string());
    js.new_boolean(false);
    js.new_u64(0xfffffffffff);
    js.new_str("你好 World!!!!!!".to_string());
    js.call(3);
    thread::sleep(Duration::from_millis(1000));
    let args = Box::new(move |tmp: Arc<JS>| {
        tmp.new_u32(4);
        1
    });
    push_callback(js.clone(), 0, args, Atom::from("callback by sync call"));
    thread::sleep(Duration::from_millis(1000));
    let args = Box::new(move |tmp: Arc<JS>| {
        tmp.new_u32(5);
        1
    });
    push_callback(js.clone(), 1, args, Atom::from("callback by sync call"));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn native_object_call_block_reply_test() {
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 3, 1024 * 1024, 1000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("native_object_call_block_reply_test_0.js".to_string(), "var obj = {}; console.log(\"!!!!!!obj: \" + obj); __thread_call(function() { var r = NativeObject.call(0xffffffff, [true, 0.999, \"你好\"]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); }); function callback(index) { console.log(\"!!!!!!callback ok, index: \" + index); var r = NativeObject.call(0xffffffff, [index]); r = __thread_yield(); console.log(\"!!!!!!callback ok, r: \" + r); }; function async_call(func) { var index = callbacks.register(func); NativeObject.call(index, []); __thread_yield(); console.log(\"!!!!!!register callback ok, index: \" + index); };\n".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
    //运行时阻塞返回
    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World0".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task0"));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }

    let opts = js.compile("native_object_call_block_reply_test_1.js".to_string(), "function call(x, y, z) { async_call(callback); var r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); try{ __thread_yield() } catch(e) { console.log(\"!!!!!!e: \" + e) } async_call(callback); async_call(callback); };".to_string());
    assert!(opts.is_some());
    let codes1 = opts.unwrap();
    assert!(js.load(codes1.as_slice()));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }
    
    //调用时阻塞返回
    js.set_tasks(create_js_task_queue(100, false)); //为虚拟机设置同步任务队列
    let copy = js.clone();
    let task_type = TaskType::Sync(true);
    let func = Box::new(move|lock: Option<isize>| {
        copy.set_tasks(lock.unwrap());
        copy.get_js_function("call".to_string());
        copy.new_boolean(true);
        copy.new_f64(0.999);
        copy.new_str("你好 World!!!!!!".to_string());
        copy.call(3);
    });
    cast_js_task(task_type, 0, Some(js.get_tasks()), func, Atom::from("call block task"));
    thread::sleep(Duration::from_millis(500)); //保证同步任务先执行
    
    let result = |vm: Arc<JS>| {
        vm.new_u32(0);
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply register callback task0"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World1".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task1"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World2".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task2"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World3".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task3"));
    thread::sleep(Duration::from_millis(500));

    block_throw(js.clone(), "Throw Error".to_string(), Atom::from("block throw task"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_u32(1);
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply register callback task1"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_u32(2);
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply register callback task2"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_u32(0);
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply callback task0"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_u32(1);
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply callback task1"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_u32(2);
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply callback task2"));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }
}

// #[test]
fn native_object_call_block_reply_test_by_clone() {
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 3, 1024 * 1024, 1000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("native_object_call_block_reply_test_by_clone.js".to_string(), "var obj = {}; console.log(\"!!!!!!obj: \" + obj); __thread_call(function() { var r = NativeObject.call(0xffffffff, [true, 0.999, \"你好\"]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); }); function call(x, y, z) { var r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); try{ __thread_yield() } catch(e) { console.log(\"!!!!!!e: \" + e) } };".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
    //运行时阻塞返回
    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World0".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task0"));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }

    //调用时阻塞返回
    let copy = js.clone();
    let task_type = TaskType::Sync(true);
    let priority = 10;
    let func = Box::new(move|lock: Option<isize>| {
        copy.set_tasks(lock.unwrap());
        copy.get_js_function("call".to_string());
        copy.new_boolean(true);
        copy.new_f64(0.999);
        copy.new_str("你好 World!!!!!!".to_string());
        copy.call(3);
    });
    cast_js_task(task_type, 0, Some(js.get_tasks()), func, Atom::from("call block task"));
    thread::sleep(Duration::from_millis(500)); //保证同步任务先执行
    
    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World1".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task1"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World2".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task2"));
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World3".to_string());
    };
    block_reply(js.clone(), Box::new(result), Atom::from("block reply task3"));

    block_throw(js.clone(), "Throw Error".to_string(), Atom::from("block throw task"));
    thread::sleep(Duration::from_millis(1000));
}

// #[test]
// fn test_async_request_and_repsonse() {
//     let worker_pool = Box::new(WorkerPool::new(3, 1024 * 1024, 1000));
//     worker_pool.run(JS_TASK_POOL.clone());

//     struct AsyncRequestHandler;

//     unsafe impl Send for AsyncRequestHandler {}
//     unsafe impl Sync for AsyncRequestHandler {}

//     impl Handler for AsyncRequestHandler {
//         type A = Arc<Vec<u8>>;
//         type B = Vec<JSType>;
//         type C = Option<u32>;
//         type D = ();
//         type E = ();
//         type F = ();
//         type G = ();
//         type H = ();
//         type HandleResult = ();

//         fn handle(&self, env: Arc<dyn Env>, name: Atom, args: Args<Self::A, Self::B, Self::C, Self::D, Self::E, Self::F, Self::G, Self::H>) -> Self::HandleResult {
//             match env.get_attr(Atom::from("_$gray")) {
//                 Some(val) => {
//                     match val {
//                         GenType::USize(gray) => {
//                             println!("!!!!!!gray: {}", gray);
//                         },
//                         _ => assert!(false),
//                     }
//                 },
//                 _ => assert!(false),
//             }

//             assert!(name == Atom::from("test_async_call"));

//             match args {
//                 Args::ThreeArgs(bin, native_objs, callback) => {
//                     assert!(callback.is_some());
//                     let index = callback.unwrap();
//                     assert!(index == 0);
//                     assert!(native_objs[0].get_native_object() == 0);
//                     assert!(native_objs[1].get_native_object() == 1);
//                     assert!(native_objs[2].get_native_object() == 0xffffffff);
//                     println!("!!!!!!bin: {:?}", bin);

//                     let mut objs = Vec::new();
//                     for idx in 0..native_objs.len() {
//                         objs.push(native_objs[idx].get_native_object());
//                     }
//                     let channel = unsafe { Arc::from_raw(Arc::into_raw(env.clone()) as *const VMChannel) };
//                     assert!(channel.response(Some(index), Arc::new("Async Call OK".to_string().into_bytes()), objs))
//                 },
//                 _ => assert!(false)
//             }
//         }
//     }

//     register_async_request(Atom::from("test_async_call"), Arc::new(AsyncRequestHandler));

//     load_lib_backtrace();
//     register_native_object();
//     let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
//     assert!(opts.is_some());
//     let js = opts.unwrap();
//     let opts = js.compile("native_async_call.js".to_string(), "var index = callbacks.register(function(result, objs) { console.log(\"!!!!!!async call ok, result:\", result); for(i = 0; i < objs.length; i++) { console.log(\"!!!!!!async call ok, objs[\" + i + \"]:\", objs[i].toString(), is_native_object(objs[i])); } }); var r = NativeObject.call(0x7fffffff, []); console.log(\"!!!!!!async call start, callback:\", index, \", r:\", r);".to_string());
//     assert!(opts.is_some());
//     let codes0 = opts.unwrap();
//     assert!(js.load(codes0.as_slice()));
//     while !js.is_ran() {
//         thread::sleep(Duration::from_millis(1));
//     }
// }

// #[test]
// fn test_async_block_request_and_repsonse() {
//     let worker_pool = Box::new(WorkerPool::new(3, 1024 * 1024, 1000));
//     worker_pool.run(JS_TASK_POOL.clone());

//     struct AsyncBlockRequestHandler;

//     unsafe impl Send for AsyncBlockRequestHandler {}
//     unsafe impl Sync for AsyncBlockRequestHandler {}

//     impl Handler for AsyncBlockRequestHandler {
//         type A = Arc<Vec<u8>>;
//         type B = Vec<JSType>;
//         type C = Option<u32>;
//         type D = ();
//         type E = ();
//         type F = ();
//         type G = ();
//         type H = ();
//         type HandleResult = ();

//         fn handle(&self, env: Arc<dyn Env>, name: Atom, args: Args<Self::A, Self::B, Self::C, Self::D, Self::E, Self::F, Self::G, Self::H>) -> Self::HandleResult {
//             match env.get_attr(Atom::from("_$gray")) {
//                 Some(val) => {
//                     match val {
//                         GenType::USize(gray) => {
//                             println!("!!!!!!gray: {}", gray);
//                         },
//                         _ => assert!(false),
//                     }
//                 },
//                 _ => assert!(false),
//             }

//             assert!(name == Atom::from("test_async_block_call"));

//             match args {
//                 Args::ThreeArgs(bin, native_objs, callback) => {
//                     assert!(callback.is_none());
//                     assert!(native_objs[0].get_native_object() == 0);
//                     assert!(native_objs[1].get_native_object() == 1);
//                     assert!(native_objs[2].get_native_object() == 0xffffffff);
//                     println!("!!!!!!bin: {:?}", bin);

//                     let mut objs = Vec::new();
//                     for idx in 0..native_objs.len() {
//                         objs.push(native_objs[idx].get_native_object());
//                     }
//                     let channel = unsafe { Arc::from_raw(Arc::into_raw(env.clone()) as *const VMChannel) };
//                     assert!(channel.response(callback, Arc::new("Async Block Call OK".to_string().into_bytes()), objs))
//                 },
//                 _ => assert!(false)
//             }
//         }
//     }

//     register_async_request(Atom::from("test_async_block_call"), Arc::new(AsyncBlockRequestHandler));

//     load_lib_backtrace();
//     register_native_object();
//     let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
//     assert!(opts.is_some());
//     let js = opts.unwrap();
//     let opts = js.compile("native_async_block_call.js".to_string(), "function async_block_call() { var r = NativeObject.call(0x7fffffff, []); console.log(\"!!!!!!async block call start, r: \" + r); r = __thread_yield(); console.log(\"!!!!!!async block call ok, result:\", r); var objs = r[1]; for(i = 0; i < objs.length; i++) { console.log(\"!!!!!!objs[\" + i + \"]:\", objs[i].toString(), is_native_object(objs[i])); } }".to_string());
//     assert!(opts.is_some());
//     let codes0 = opts.unwrap();
//     assert!(js.load(codes0.as_slice()));
//     while !js.is_ran() {
//         thread::sleep(Duration::from_millis(1));
//     }

//     let copy = js.clone();
//     let task_type = TaskType::Sync(true);
//     let priority = 10;
//     let func = Box::new(move|lock: Option<isize>| {
//         copy.set_tasks(lock.unwrap());
//         copy.get_js_function("async_block_call".to_string());
//         copy.call(0);
//     });
//     cast_js_task(task_type, 0, Some(&js.get_tasks()), func, Atom::from("async block call task"));
//     thread::sleep(Duration::from_millis(5000)); //保证异步阻塞调用执行
// }

// #[test]
fn task_test() {
    let mut worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 3, 1024 * 1024, 1000, JS_WORKER_WALKER. clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    
    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("task_test.js".to_string(), "var obj = {}; console.log(\"!!!!!!obj: \" + obj); function echo(x, y, z) { console.log(\"!!!!!!x: \" + x + \" y: \" + y + \" z: \" + z); };".to_string());
    assert!(opts.is_some());
    let codes0 = opts.unwrap();
    assert!(js.load(codes0.as_slice()));
    while !js.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }
    let copy_js = js.clone();
    let copy_js0 = js.clone();

    let task_type = TaskType::Sync(true);
    let priority = 0;
    let func = Box::new(move|lock: Option<isize>| {
        js.set_tasks(lock.unwrap());
        js.get_js_function("echo".to_string());
        js.new_boolean(false);
        js.new_u64(0xfffffffffff);
        js.new_str("Hello World!!!!!!".to_string());
        js.call(3);

        let task_type = TaskType::Sync(true);
        let priority = 10;
        let func = Box::new(move|lock: Option<isize>| {
            js.set_tasks(lock.unwrap());
            js.get_js_function("echo".to_string());
            js.new_boolean(true);
            js.new_f64(0.999);
            js.new_str("你好 World!!!!!!".to_string());
            js.call(3);
            thread::sleep(Duration::from_millis(1000)); //延迟结束任务
        });
        cast_js_task(task_type, 0, Some(copy_js.get_tasks()), func, Atom::from("first task"));
        thread::sleep(Duration::from_millis(1000)); //延迟结束任务
    });
    cast_js_task(task_type, 0, Some(copy_js0.get_tasks()), func, Atom::from("second task"));
    println!("worker_pool: {}", worker_pool);
    //测试运行任务的同时增加工作者
    for index in 0..10 {
        let mut copy: Arc<JS> = copy_js0.clone();
        cast_js_task(task_type, 0, Some(copy_js0.get_tasks()), Box::new(move |lock: Option<isize>| {
                copy.set_tasks(lock.unwrap());
                copy.get_js_function("echo".to_string());
                copy.new_boolean(true);
                copy.new_u64(index);
                copy.new_str("Hello World!!!!!!".to_string());
                copy.call(3);
                thread::sleep(Duration::from_millis(1000)); //延迟结束任务
            }), Atom::from("other task"));
    }
    worker_pool.increase(JS_TASK_POOL.clone(), 7);
    thread::sleep(Duration::from_millis(10000));
    println!("worker_pool: {}", worker_pool);
    //测试运行任务的同时减少工作者
    for index in 0..10 {
        let mut copy: Arc<JS> = copy_js0.clone();
        cast_js_task(task_type, 0, Some(copy_js0.get_tasks()), Box::new(move |lock: Option<isize>| {
                copy.set_tasks(lock.unwrap());
                copy.get_js_function("echo".to_string());
                copy.new_boolean(false);
                copy.new_u64(index);
                copy.new_str("Hello World!!!!!!".to_string());
                copy.call(3);
                thread::sleep(Duration::from_millis(1000)); //延迟结束任务
            }), Atom::from("other task"));
    }
    worker_pool.decrease(JS_TASK_POOL.clone(), 7);
    thread::sleep(Duration::from_millis(10000));
    println!("worker_pool: {}", worker_pool);
}

// #[test]
fn test_vm_factory() {
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 3, 1024 * 1024, 1000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    load_lib_backtrace();
    register_native_object();
    let opts = JS::new(Arc::new(NativeObjsAuth::new(None, None)));
    assert!(opts.is_some());
    let js = opts.unwrap();
    let opts = js.compile("test_vm_factory.js".to_string(), "function call(x, y) { console.log(\"!!!!!!x: \" + x + \", y: \" + y + \", y length: \" + y.length); var r = NativeObject.call(0xffffffff, [x, y]); console.log(\"!!!!!!r: \" + r); };".to_string());
    assert!(opts.is_some());
    let code = opts.unwrap();

    let factory = VMFactory::new(0, Arc::new(NativeObjsAuth::new(None, None)));
    assert!(factory.size() == 0);
    // factory.append(Arc::new(code))
    //         .call(1, Atom::from("Hello World"), Arc::new(vec![100, 100, 100, 100, 100, 100]), "factory call");
    thread::sleep(Duration::from_millis(1000));
}

#[test]
fn test_new_task_pool() {
    let worker_pool = Box::new(WorkerPool::new("js test".to_string(), WorkerType::Js, 3, 1024 * 1024, 1000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());
    
    let queue = create_js_task_queue(10, false);
    for n in 0..10 {
        let func = Box::new(move |lock: Option<isize>| {
            println!("!!!!!!n: {}", n);
            if n % 2 == 0 {
                lock_js_task_queue(lock.unwrap());
                println!("!!!!!!lock queue ok");
            } else {
                unlock_js_task_queue(lock.unwrap());
                println!("!!!!!!unlock queue ok");
            }
        });
        cast_js_task(TaskType::Sync(true), 0, Some(queue.clone()), func, Atom::from("test new task pool"));
    }
    thread::sleep(Duration::from_millis(1000000000));
}
