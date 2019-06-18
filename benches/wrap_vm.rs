#![feature(test)]
#![feature(proc_macro_hygiene)]

extern crate test;

#[macro_use]
extern crate lazy_static;

extern crate flame;
#[macro_use]
extern crate flamer;

extern crate atom;
extern crate pi_vm;
extern crate handler;
extern crate gray;
extern crate task_pool;
extern crate worker;

use std::sync::{Arc, RwLock};
use std::fs::File;
use std::io::prelude::*;

use test::Bencher;

use atom::Atom;
use handler::{GenType, Handler, Args};
use gray::{GrayVersion, Gray, GrayTab};

use pi_vm::pi_vm_impl::{VMFactory, BlockError, block_set_global_var, block_reply, block_throw, push_callback, register_async_request, async_request};
use pi_vm::adapter::{register_native_object, JS, JSType};
use pi_vm::channel_map::VMChannel;
use pi_vm::bonmgr::{BON_MGR, NativeObjsAuth, FnMeta, CallResult, ptr_jstype, jstype_ptr};

use worker::task::TaskType;
use worker::worker_pool::WorkerPool;
use worker::impls::{JS_WORKER_WALKER, JS_TASK_POOL};
use task_pool::TaskPool;
use worker::worker::WorkerType;

lazy_static! {
	pub static ref BINARY: Vec<u8> = vec![0xff; 4096];
}

//空调用
#[bench]
fn empty_call(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/js_sync_call_small_bigtest-empty-call.js");

    b.iter(|| {
        for _ in 0..10000 {
            js.get_js_function("test".to_string());
            js.call(0);
        }
        js.get_js_function("__gc".to_string());
        js.call(0);
    });
}

//有参数和返回值的空调用
#[bench]
fn base_call(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-base-call.js");

    b.iter(|| {
        for _ in 0..10000 {
            js.get_js_function("test".to_string());
            js.new_undefined();
            js.new_boolean(false);
            js.new_u32(0xffffffff);
            js.new_u64(0xfffffffffff);
            js.new_f64(0.999999999);
            js.new_str("Hello World!!!!!!".to_string());
            js.call(6);
        }
        js.get_js_function("__gc".to_string());
        js.call(0);
    });
}

//小参数小返回同步调用
#[bench]
fn js_sync_call_small_small(b: &mut Bencher) {
    register_native_object();
    register_native_function(0x1, js_sync_call_return_small);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_call_args_small.js");

    start(b, js);
}

//小参数大返回同步调用
#[bench]
fn js_sync_call_small_big(b: &mut Bencher) {
    register_native_object();
    register_native_function(0x1, js_sync_call_return_big);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_call_args_small.js");

    start(b, js);
}

//大参数小返回同步调用
#[bench]
fn js_sync_call_big_small(b: &mut Bencher) {
    register_native_object();
    register_native_function(0x1, js_sync_call_return_small);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_call_args_big.js");

    start(b, js);
}

//大参数大返回同步调用
#[bench]
fn js_sync_call_big_big(b: &mut Bencher) {
    register_native_object();
    register_native_function(0x1, js_sync_call_return_big);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_call_args_big.js");

    start(b, js);
}

//错误返回同步调用
#[bench]
fn js_sync_call_error(b: &mut Bencher) {
    register_native_object();
    register_native_function(0x1, js_sync_call_return_error);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_call.js");

    start(b, js);
}

fn js_sync_call_return_small(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    js.new_u32(0xffffffff);
    Some(CallResult::Ok)
}

fn js_sync_call_return_big(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    let buffer = js.new_array_buffer(4096);
    buffer.from_bytes(BINARY.as_slice());
    Some(CallResult::Ok)
}

fn js_sync_call_return_error(_js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    Some(CallResult::Err("What is Error?".to_string()))
}

//同步阻塞调用，设置全局变量后返回
#[bench]
fn js_sync_block_call_set_global_var(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js, 1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, js_sync_block_call_set_global_var_return);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_block_call_args_small_get_global_var.js");

    start(b, js);
}

//小参数小返回同步阻塞调用
#[bench]
fn js_sync_block_call_small_small(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js, 1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, js_sync_block_call_return_small);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_block_call_args_small.js");

    start(b, js);
}

//小参数大返回同步阻塞调用
#[bench]
fn js_sync_block_call_small_big(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 1000000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, js_sync_block_call_return_big);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_block_call_args_small.js");

    start(b, js);
}

//大参数小返回同步阻塞调用
#[bench]
fn js_sync_block_call_big_small(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, js_sync_block_call_return_small);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_block_call_args_big.js");

    start(b, js);
}

//大参数大返回同步阻塞调用
#[bench]
fn js_sync_block_call_big_big(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 1000000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, js_sync_block_call_return_big);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_block_call_args_big.js");

    start(b, js);
}

//错误返回同步阻塞调用
#[bench]
fn js_sync_block_call_error(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 1000000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, js_sync_block_call_return_error);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_sync_block_call.js");

    start(b, js);
}

fn js_sync_block_call_set_global_var_return(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    let var = Box::new(move |js: Arc<JS>| -> Result<JSType, String> {
        let array = js.new_array();
        let mut key = js.new_str("Hello".to_string());
        js.set_index(&array, 0, &mut key);
        let mut value = js.new_str("World!".to_string());
        js.set_index(&array, 1, &mut value);
        Ok(array)
    });

    let next = Box::new(move |r: Result<Arc<JS>, BlockError>| {
        match r {
            Ok(js) => {
                let result = Box::new(|vm: Arc<JS>| {
                    vm.new_u32(0xffffffff);
                });
                block_reply(js, result, Atom::from("block reply task"));
            },
            Err(BlockError::SetGlobalVar(name)) => {
                panic!("!!!!!!set global var failed, name: {:?}", name);
            },
            _ => {
                panic!("!!!!!!other error");
            }
        }
    });

    block_set_global_var(js, "_$tmp_var".to_string(), var, next, Atom::from("js_sync_block_call_set_global_var_return task"));
    None
}

fn js_sync_block_call_return_small(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    let result = Box::new(|vm: Arc<JS>| {
        vm.new_u32(0xffffffff);
    });
    block_reply(js, result, Atom::from("block reply task"));
    None
}

fn js_sync_block_call_return_big(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    let result = Box::new(|vm: Arc<JS>| {
        let buffer = vm.new_array_buffer(4096);
        buffer.from_bytes(BINARY.as_slice());
    });
    block_reply(js, result, Atom::from("block reply task"));
    None
}

fn js_sync_block_call_return_error(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    block_throw(js, "What is Error?".to_string(), Atom::from("block throw task"));
    None
}

//注册异步回调
#[bench]
fn js_async_callback_register(b: &mut Bencher) {
    register_native_object();
    register_native_function(0x1, js_async_callback_register_no_push);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_async_callback_register.js");

    //因为不执行异步回调，所以虚拟机状态为有任务未完成，无法使用start正常结束测试
    b.iter(|| {
        js.get_js_function("test".to_string());
        js.call(0);
    });
}

//异步回调
#[bench]
fn js_async_callback(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, js_async_callback_register_push);

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_async_callback_register.js");

    start(b, js);
}

fn js_async_callback_register_no_push(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    js.new_boolean(true);
    Some(CallResult::Ok)
}

fn js_async_callback_register_push(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let callback = args[0].get_u32();
    let func = Box::new(move |vm: Arc<JS>| -> usize {
        vm.new_u32(callback);
        vm.new_u32(callback);
        vm.new_u32(callback);
        3
    });
    push_callback(js.clone(), args[0].get_u32(), func, Atom::from("register callback task"));
    js.new_boolean(true);
    Some(CallResult::Ok)
}

//小参数小返回异步调用
#[bench]
fn js_async_call_small_small(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    init_async_request_env("test_async_call", "benches/pref/test_js_async_remote_small.js");
    register_native_function(0x1, js_async_call_request); //注册异步调用的本地请求函数
    register_native_function(0x10, js_async_call_response); //注册异步调用的本地回应函数

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_async_call_small.js");

    start(b, js);
}

//大参数大返回异步调用
#[bench]
fn js_async_call_big_big(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    init_async_request_env("test_async_call", "benches/pref/test_js_async_remote_big.js");
    register_native_function(0x1, js_async_call_request); //注册异步调用的本地请求函数
    register_native_function(0x10, js_async_call_response); //注册异步调用的本地回应函数

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_async_call_big.js");

    start(b, js);
}

fn js_async_call_request(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let call = args[0].get_str();
    let seq = args[1].into_vec();
    let callback = args[2].get_u32();
    if !async_request(js.clone(), Atom::from(call), Arc::new(seq), vec![0xffff, 0x1ffff, 0x1fffff], Some(callback)) {
        panic!("!!!> Async Request Error");
    }
    js.new_boolean(true);
    Some(CallResult::Ok)
}

fn js_async_call_response(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let tmp = &args[0];
    if let Ok(ptr) = jstype_ptr(tmp, js.clone(), 3366364668, true, "response parse channel failed") {
        let channel = unsafe { *(Box::from_raw(ptr as *mut Arc<VMChannel>)) };
        let result = args[1].into_vec();
        let len = args[2].get_array_length();
        let mut objs = Vec::with_capacity(len);
        for index in 0..len {
            objs.push(args[2].get_index(index as u32).get_native_object());
        }
        let callback = args[3].get_u32();
        channel.response(Some(callback), Arc::new(result), objs);
        js.new_boolean(true);
    } else {
        js.new_boolean(false);
    }
    Some(CallResult::Ok)
}

//小参数小返回异步阻塞调用
#[bench]
fn js_async_block_call_small_small(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    init_async_request_env("test_async_block_call", "benches/pref/test_js_async_remote_small.js");
    register_native_function(0x1, js_async_block_call_request); //注册异步阻塞调用的本地请求函数
    register_native_function(0x10, js_async_block_call_response); //注册异步阻塞调用的本地回应函数

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_async_block_call_small.js");

    start(b, js);
}

//大参数大返回异步阻塞调用
#[bench]
fn js_async_block_call_big_big(b: &mut Bencher) {
    let worker_pool = Box::new(WorkerPool::new("Test Wrap VM".to_string(), WorkerType::Js,1, 1024 * 1024, 100000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    init_async_request_env("test_async_block_call", "benches/pref/test_js_async_remote_big.js");
    register_native_function(0x1, js_async_block_call_request); //注册异步阻塞调用的本地请求函数
    register_native_function(0x10, js_async_block_call_response); //注册异步阻塞调用的本地回应函数

    let js = create_js();
    load_js(js.clone(), "benches/pref/test_js_async_block_call_big.js");

    start(b, js);
}

fn js_async_block_call_request(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let call = args[0].get_str();
    let seq = args[1].into_vec();
    if !async_request(js.clone(), Atom::from(call), Arc::new(seq), vec![0xffff, 0x1ffff, 0x1fffff], None) {
        panic!("!!!> Async Request Error");
    }
    None
}

fn js_async_block_call_response(js: Arc<JS>, args: Vec<JSType>) -> Option<CallResult> {
    let tmp = &args[0];
    if let Ok(ptr) = jstype_ptr(tmp, js.clone(), 3366364668, true, "response parse channel failed") {
        let channel = unsafe { *(Box::from_raw(ptr as *mut Arc<VMChannel>)) };
        let result = args[1].into_vec();
        let len = args[2].get_array_length();
        let mut objs = Vec::with_capacity(len);
        for index in 0..len {
            objs.push(args[2].get_index(index as u32).get_native_object());
        }
        channel.response(None, Arc::new(result), objs);
        js.new_boolean(true);
    } else {
        js.new_boolean(false);
    }
    Some(CallResult::Ok)
}

fn init_async_request_env(port: &str, file: &str) {
    //模拟的表库及事务管理器
    #[derive(Clone)]
    pub struct Mgr;

    //模拟的NativeObject, 灰度系统需要使用
    #[derive(Clone, Debug)]
    pub struct Nobj;

    //模拟的本地对象
    #[derive(Clone)]
    pub struct Nobjs;

    //模拟的灰度
    #[derive(Clone)]
    pub struct JSGray {
        pub mgr: Mgr, //数据库管理器
        pub factory: Arc<VMFactory>, //虚拟机工厂
        pub nobjs: Nobjs, //本地对象
        pub name: Atom,//为灰度取一个名称， 所有灰度不能重复重复
    }

    impl Gray for JSGray {}

    impl JSGray {
        pub fn new(mgr: &Mgr, factory: VMFactory, name: &str, nobjs: &Nobjs) -> Self{
            JSGray{
                mgr: mgr.clone(),
                factory: Arc::new(factory),
                nobjs: nobjs.clone(),
                name: Atom::from(name),
            }
        }
    }

    //异步请求处理器
    struct AsyncRequestHandler {
        gray_tab: 	Arc<RwLock<GrayTab<JSGray>>>,	//灰度表
    }

    unsafe impl Send for AsyncRequestHandler {}
    unsafe impl Sync for AsyncRequestHandler {}

    impl Handler for AsyncRequestHandler {
        type A = Arc<Vec<u8>>;
        type B = Vec<JSType>;
        type C = Option<u32>;
        type D = ();
        type E = ();
        type F = ();
        type G = ();
        type H = ();
        type HandleResult = ();

        fn handle(&self, env: Arc<dyn GrayVersion>, name: Atom, args: Args<Self::A, Self::B, Self::C, Self::D, Self::E, Self::F, Self::G, Self::H>) -> Self::HandleResult {
            let gray_tab = self.gray_tab.read().unwrap();
            let gray = match env.get_gray() {
                Some(v) => match gray_tab.get(v) {
                    Some(g) => g,
                    None => panic!("gray is not exist, version:{}", v),
                },
                None => gray_tab.get_last(),
            };
            let mgr = gray.mgr.clone();
            let copy_name = name.clone();
            let real_args = Box::new(move |vm: Arc<JS>| -> usize {
                vm.new_str((*copy_name).to_string());
                match args {
                    Args::ThreeArgs(bin, objs, None) => {
                        //处理异步阻塞调用
                        let buffer = vm.new_uint8_array(bin.len() as u32);
                        buffer.from_bytes(bin.as_slice());
                        let mut value: JSType;
                        let array = vm.new_array();
                        for i in 0..objs.len() {
                            value = vm.new_native_object(objs[i].get_native_object());
                            vm.set_index(&array, i as u32, &mut value);
                        }
                        vm.new_null();
                    },
                    Args::ThreeArgs(bin, objs, Some(index)) => {
                        //处理异步调用
                        let buffer = vm.new_uint8_array(bin.len() as u32);
                        buffer.from_bytes(bin.as_slice());
                        let mut value: JSType;
                        let array = vm.new_array();
                        for i in 0..objs.len() {
                            value = vm.new_native_object(objs[i].get_native_object());
                            vm.set_index(&array, i as u32, &mut value);
                        }
                        vm.new_u32(index);
                    },
                    _ => panic!("invalid async call handler args"),
                }
                let ptr = Box::into_raw(Box::new(mgr.clone())) as usize;
                ptr_jstype(vm.get_objs(), vm.clone(), ptr, 2976191628);
                let ptr = Box::into_raw(Box::new(env.clone())) as usize;
                ptr_jstype(vm.get_objs(), vm.clone(), ptr, 3366364668);
                6
            });
            gray.factory.call(None, Atom::from("_$async"), real_args, Atom::from((*name).to_string() + " rpc task"));
        }
    }

    impl AsyncRequestHandler {
        //构建一个处理器
        pub fn new(gray: JSGray) -> Self {
            AsyncRequestHandler {
                gray_tab: Arc::new(RwLock::new(GrayTab::new(gray))),
            }
        }
    }

    let mut factory = VMFactory::new("wrap vm benches", 1000, 1000, 8388608, 67108864, Arc::new(NativeObjsAuth::new(None, None)));
    let js = JS::new(0, Atom::from("wrap vm benches"), Arc::new(NativeObjsAuth::new(None, None)), None).unwrap();
    let file_name = &String::from("benches/core.js");
    if let Ok(mut file) = File::open("benches/core.js") {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(code) = js.compile(file_name.clone(), (&contents).clone()) {
                factory = factory.append(Arc::new(code));
            }
        }
    }
    let file_name = &String::from(file);
    if let Ok(mut file) = File::open(file) {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(code) = js.compile(file_name.clone(), (&contents).clone()) {
                factory = factory.append(Arc::new(code));
            }
        }
    }
    let gray = JSGray::new(&Mgr, factory, "test_gray", &Nobjs);
    let handler = AsyncRequestHandler::new(gray);

    register_async_request(Atom::from(port), Arc::new(handler));
}

//注册本地函数
fn register_native_function(id: u32, fun: fn(Arc<JS>, Vec<JSType>) -> Option<CallResult>) {
    BON_MGR.regist_fun_meta(FnMeta::CallArg(fun), id);
}

//创建虚拟机
fn create_js() -> Arc<JS> {
    if let Some(js) = JS::new(0, Atom::from("wrap vm benches"), Arc::new(NativeObjsAuth::new(None, None)), None) {
        load_js(js.clone(), "benches/core.js");
        return js;
    }
    panic!("!!!> Create Vm Error");
}

//读取指定js文件，并在指定虚拟机上编译、加载并运行
fn load_js(js: Arc<JS>, file: &str) {
    let file_name = &String::from(file);
    if let Ok(mut file) = File::open(file) {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(ref code) = js.compile(file_name.clone(), (&contents).clone()) {
                return assert!(js.load(code));
            }
        }
    }
    panic!("!!!> Load Script Error");
}

//开始测试
fn start(b: &mut Bencher, js: Arc<JS>) {
    b.iter(|| {
        js.get_js_function("test".to_string());
        js.call(0);
        while !js.is_ran() {}
    });
}