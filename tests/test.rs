#![feature(duration_extras)]

#[cfg(test)]
extern crate pi_vm;
extern crate threadpool;

use std::thread;
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex, Condvar};

use pi_vm::task::TaskType;
use pi_vm::task_pool::TaskPool;
use pi_vm::util::now_nanosecond;
use pi_vm::worker_pool::WorkerPool;
use pi_vm::pi_vm_impl::{JS_TASK_POOL, cast_task, block_reply};
use pi_vm::adapter::{load_lib_backtrace, register_native_object, dukc_remove_value, JSTemplate, JS};

// // #[test]
// fn njsc_test() {
//     load_lib_backtrace();
//     register_native_object();
//     njsc_test_main();
// }

// #[test]
fn test_vm_performance() {
    register_native_object();

    let js = JSTemplate::new("test_vm_clone_performance.js".to_string(), "function call(x, y, z) { var r = [0, 0, 0]; r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); };".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let time = Instant::now();
    for _ in 0..10000 {
        js.clone().unwrap().run();
    }
    let finish_time = time.elapsed();
    println!("!!!!!!clone time: {}", finish_time.as_secs() * 1000000 + (finish_time.subsec_micros() as u64));

    let js = JSTemplate::new("test_vm_run_performance.js".to_string(), "var x = 0; for(var n = 0; n < 100000000; n++) { x++; }".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy = js.clone().unwrap();
    let time = Instant::now();
    copy.run();
    let finish_time = time.elapsed();
    println!("!!!!!!run time: {}", finish_time.as_secs() * 1000000 + (finish_time.subsec_micros() as u64));
}

// #[test]
fn base_test() {
    load_lib_backtrace();
    register_native_object();
    let js = JSTemplate::new("base_test.js".to_string(), "var obj = {}; console.log(\"!!!!!!obj: \" + obj);".to_string());
    assert!(js.is_some());
    let copy: JS = js.unwrap().clone().unwrap();
    copy.run();
    let val = copy.new_null();
    assert!(val.is_null());
    let val = copy.new_undefined();
    assert!(val.is_undefined());
    let val = copy.new_boolean(true);
    assert!(val.is_boolean() && val.get_boolean());
    let val = copy.new_boolean(false);
    assert!(val.is_boolean() && !val.get_boolean());
    let val = copy.new_i8(0x7fi8);
    assert!(val.is_number() && val.get_i8() == 0x7fi8);
    let val = copy.new_i16(0x7fffi16);
    assert!(val.is_number() && val.get_i16() == 0x7fffi16);
    let val = copy.new_i32(0x7fffffffi32);
    assert!(val.is_number() && val.get_i32() == 0x7fffffffi32);
    let val = copy.new_i64(0x7199254740992i64);
    assert!(val.is_number() && val.get_i64() == 0x7199254740992i64);
    let val = copy.new_u8(255u8);
    assert!(val.is_number() && val.get_u8() == 255u8);
    let val = copy.new_u16(65535u16);
    assert!(val.is_number() && val.get_u16() == 65535u16);
    let val = copy.new_u32(0xffffffffu32);
    assert!(val.is_number() && val.get_u32() == 0xffffffffu32);
    let val = copy.new_u64(9007199254740992u64);
    assert!(val.is_number() && val.get_u64() == 9007199254740992u64);
    let val = copy.new_f32(0.0173136f32);
    assert!(val.is_number() && val.get_f32() == 0.0173136f32);
    let val = copy.new_f64(921.1356737853f64);
    assert!(val.is_number() && val.get_f64() == 921.1356737853f64);

    let val = copy.new_str("Hello World".to_string());
    assert!(val.is_string() && val.get_str() == "Hello World".to_string());
    let val = copy.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    assert!(val.is_string() && val.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());

    let object = copy.new_object();
    assert!(object.is_object());
    let val = copy.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    copy.set_field(&object, "x".to_string(), &val);
    let tmp = object.get_field("x".to_string());
    assert!(object.is_object() && tmp.is_string() && tmp.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    unsafe { dukc_remove_value(copy.get_vm(), tmp.get_value() as u32); }
    let tmp = object.get_field("c".to_string());
    assert!(object.is_object() &&tmp.is_none()); //key不存在

    let array = copy.new_array();
    assert!(array.is_array() && array.get_array_length() == 0);
    let object = copy.new_object();
    let val = copy.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    copy.set_field(&object, "x".to_string(), &val);
    copy.set_index(&array, 3, &object);
    let val = copy.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    copy.set_index(&array, 30, &val); //数组自动扩容
    let tmp = array.get_index(3);
    assert!(array.is_array() && tmp.is_object() && tmp.get_field("x".to_string()).get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    unsafe { dukc_remove_value(copy.get_vm(), tmp.get_value() as u32); }
    let tmp = array.get_index(30);
    assert!(array.is_array() && tmp.is_string() && tmp.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    unsafe { dukc_remove_value(copy.get_vm(), tmp.get_value() as u32); }
    let tmp = array.get_index(0);
    assert!(array.is_array() && tmp.is_none()); //index不存在

    let val = copy.new_array_buffer(32);
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

    let mut val = copy.new_uint8_array(10);
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

    let val = copy.new_native_object(0xffffffffusize);
    assert!(val.is_native_object() && val.get_native_object() == 0xffffffffusize);
}

// #[test]
fn test_js_string() {
    load_lib_backtrace();
    register_native_object();
    let js = JSTemplate::new("test_js_string.js".to_string(), "console.log(\"!!!!!!string: \" + \"你好!!!!!!\".length); var view = (new TextEncoder()).encode(\"你好!!!!!!\"); console.log(\"!!!!!!view: \" + view); var r = NativeObject.call(0xffffffff, [view]); console.log(\"!!!!!!r: \" + r); console.log(\"!!!!!!string: \" + (new TextDecoder()).decode(view));".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy: JS = js.clone().unwrap();
    copy.run();
}

// #[test]
fn test_js_this() {
    load_lib_backtrace();
    register_native_object();

    let js = JSTemplate::new("test_js_this0.js".to_string(), "var obj = {}; function call() { console.log(\"!!!!!!obj: \" + obj); obj.a = 100; var a = 10; console.log(\"!!!!!!obj.a: \" + obj.a + \", a: \" + a); obj.func = function call0() { console.log(\"!!!!!!this.a: \" + this.a); }; obj.func();}; call();".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy: JS = js.clone().unwrap();
    copy.run();

    let js = JSTemplate::new("test_js_this1.js".to_string(), "var obj = {str: \"Hello\", func: function() { console.log(\"!!!!!!this.str: \" + this.str); this.str = 10; console.log(\"!!!!!!this.str: \" + this.str); } }; obj.func();".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy: JS = js.clone().unwrap();
    copy.run();

    let js = JSTemplate::new("test_js_this2.js".to_string(), "var obj = {x: 10, y: { func: function() { console.log(\"!!!!!!this.x: \" + this.x); this.x = \"Hello\"; console.log(\"!!!!!!this.x: \" + this.x); } } }; obj.y.func();".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy: JS = js.clone().unwrap();
    copy.run();

    let js = JSTemplate::new("test_js_this3.js".to_string(), "var obj = {name : 'linxin'}; function func(firstName, lastName) { console.log(firstName + ' ' + this.name + ' ' + lastName); } func.apply(obj, ['A', 'B']);".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy: JS = js.clone().unwrap();
    copy.run();

    // use std::path::{Path, PathBuf};
    // use std::fs::{File, OpenOptions, Metadata, remove_file};
    // use std::io::{Seek, Read, Write, Result, SeekFrom, Error, ErrorKind};

    // match OpenOptions::new()
    //                 .read(true)
    //                 .write(false)
    //                 .append(false)
    //                 .create(false)
    //                 .open("core.min.js") {
    //     Err(e) => panic!("open file failed"),
    //     Ok(mut file) => {
    //         let mut buffer = Vec::new();
    //         buffer.resize(87628, 0);
    //         file.read(&mut buffer[..]);
            
    //         let js = JSTemplate::new("core.min.js".to_string(), String::from_utf8(buffer).ok().unwrap());
    //         assert!(js.is_some());
    //         let js = js.unwrap();
    //         let copy: JS = js.clone().unwrap();
    //         copy.run();
    //     },
    // }
}

// #[test]
fn native_object_call_test() {
    let mut worker_pool = Box::new(WorkerPool::new(3, 1024 * 1024, 1000));
    worker_pool.run(JS_TASK_POOL.clone());

    load_lib_backtrace();
    register_native_object();
    let js = JSTemplate::new("native_object_call_test.js".to_string(), "var obj = {}; console.log(\"!!!!!!obj: \" + obj); function call(x, y, z) { var r = [0, 0, 0]; r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); };".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy: JS = js.clone().unwrap();
    copy.run();

    copy.get_js_function("call".to_string());
    copy.new_boolean(false);
    copy.new_u64(0xfffffffffff);
    copy.new_str("Hello World!!!!!!".to_string());
    copy.call(3);
    
    copy.get_js_function("call".to_string());
    copy.new_boolean(false);
    copy.new_u64(0xfffffffffff);
    copy.new_str("Hello World!!!!!!".to_string());
    copy.call(3);
    
    copy.get_js_function("call".to_string());
    copy.new_boolean(false);
    copy.new_u64(0xfffffffffff);
    copy.new_str("你好 World!!!!!!".to_string());
    copy.call(3);
}

#[test]
fn native_object_call_block_reply_test() {
    let mut worker_pool = Box::new(WorkerPool::new(3, 1024 * 1024, 1000));
    worker_pool.run(JS_TASK_POOL.clone());

    load_lib_backtrace();
    register_native_object();
    let js = JSTemplate::new("native_object_call_block_reply_test.js".to_string(), "var obj = {}; console.log(\"!!!!!!obj: \" + obj); __thread_call(function() { var r = NativeObject.call(0xffffffff, [true, 0.999, \"你好\"]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); }); function call(x, y, z) { var r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); r = NativeObject.call(0xffffffff, [x, y, z]); console.log(\"!!!!!!r: \" + r); r = __thread_yield(); console.log(\"!!!!!!r: \" + r); };".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy = Arc::new(js.clone().unwrap());
    copy.run();
    //运行时阻塞返回
    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World0".to_string());
    };
    block_reply(copy.clone(), Box::new(result), TaskType::Sync, 10, "block reply task0");
    while !copy.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }

    //调用时阻塞返回
    let copy1 = copy.clone();
    let task_type = TaskType::Async;
    let priority = 10;
    let func = Box::new(move|| {
        copy1.get_js_function("call".to_string());
        copy1.new_boolean(true);
        copy1.new_f64(0.999);
        copy1.new_str("你好 World!!!!!!".to_string());
        copy1.call(3);
    });
    cast_task(task_type, priority, func, "call block task");
    thread::sleep(Duration::from_millis(500)); //保证同步任务先执行
    
    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World1".to_string());
    };
    block_reply(copy.clone(), Box::new(result), TaskType::Sync, 10, "block reply task1");
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World2".to_string());
    };
    block_reply(copy.clone(), Box::new(result), TaskType::Sync, 10, "block reply task2");
    thread::sleep(Duration::from_millis(500));

    let result = |vm: Arc<JS>| {
        vm.new_str("Hello World3".to_string());
    };
    block_reply(copy.clone(), Box::new(result), TaskType::Sync, 10, "block reply task3");
    thread::sleep(Duration::from_millis(1000));
}

// #[test]
fn task_test() {
    let mut worker_pool = Box::new(WorkerPool::new(3, 1024 * 1024, 1000));
    worker_pool.run(JS_TASK_POOL.clone());
    
    load_lib_backtrace();
    register_native_object();
    let js = JSTemplate::new("task_test.js".to_string(), "var obj = {}; console.log(\"!!!!!!obj: \" + obj); function echo(x, y, z) { console.log(\"!!!!!!x: \" + x + \" y: \" + y + \" z: \" + z); };".to_string());
    assert!(js.is_some());
    let js = js.unwrap();
    let copy: JS = js.clone().unwrap();
    copy.run();
    while !copy.is_ran() {
        thread::sleep(Duration::from_millis(1));
    }

    let task_type = TaskType::Async;
    let priority = 0;
    let func = Box::new(move|| {
        copy.get_js_function("echo".to_string());
        copy.new_boolean(false);
        copy.new_u64(0xfffffffffff);
        copy.new_str("Hello World!!!!!!".to_string());
        copy.call(3);

        let task_type = TaskType::Async;
        let priority = 10;
        let func = Box::new(move|| {
            copy.get_js_function("echo".to_string());
            copy.new_boolean(true);
            copy.new_f64(0.999);
            copy.new_str("你好 World!!!!!!".to_string());
            copy.call(3);
            thread::sleep(Duration::from_millis(1000)); //延迟结束任务
        });
        cast_task(task_type, priority, func, "first task");
        thread::sleep(Duration::from_millis(1000)); //延迟结束任务
    });
    cast_task(task_type, priority, func, "second task");
    println!("worker_pool: {}", worker_pool);
    //测试运行任务的同时增加工作者
    for index in 0..10 {
        let mut copy: JS = (&js).clone().unwrap();
        copy.run();
        cast_task(TaskType::Sync, 10, Box::new(move || {
                copy.get_js_function("echo".to_string());
                copy.new_boolean(true);
                copy.new_u64(index);
                copy.new_str("Hello World!!!!!!".to_string());
                copy.call(3);
                thread::sleep(Duration::from_millis(1000)); //延迟结束任务
            }), "other task");
    }
    worker_pool.increase(JS_TASK_POOL.clone(), 7, 1000);
    thread::sleep(Duration::from_millis(10000));
    println!("worker_pool: {}", worker_pool);
    //测试运行任务的同时减少工作者
    for index in 0..10 {
        let mut copy: JS = (&js).clone().unwrap();
        copy.run();
        cast_task(TaskType::Sync, 10, Box::new(move || {
                copy.get_js_function("echo".to_string());
                copy.new_boolean(false);
                copy.new_u64(index);
                copy.new_str("Hello World!!!!!!".to_string());
                copy.call(3);
                thread::sleep(Duration::from_millis(1000)); //延迟结束任务
            }), "other task");
    }
    worker_pool.decrease(7);
    thread::sleep(Duration::from_millis(10000));
    println!("worker_pool: {}", worker_pool);
}