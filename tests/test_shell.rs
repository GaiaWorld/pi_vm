#![feature(fnbox)]

extern crate atom;
extern crate worker;
extern crate pi_vm;

use std::io;
use std::thread;
use std::sync::Arc;
use std::boxed::FnBox;
use std::sync::mpsc::channel;
use std::io::{Read, Write, Result};
use std::sync::atomic::{AtomicBool, Ordering};

use worker::worker_pool::WorkerPool;
use worker::impls::{JS_TASK_POOL, JS_WORKER_WALKER};

use atom::Atom;

use pi_vm::pi_vm_impl::block_reply;
use pi_vm::adapter::{JSType, JS, register_native_object};
use pi_vm::shell::SHELL_MANAGER;
use pi_vm::bonmgr::{BON_MGR, NativeObjsAuth, FnMeta, CallResult};

const TEST_SHELL_CODE: &'static str =
    r#"var self = this;

        var test_call = function(x) {
            return x;
        };

        var test_sync_call = function(x) {
            return NativeObject.call(0x1, [x]);
        };

        var test_block_call = function(x) {
            __thread_call(
                function() {
                    NativeObject.call(0x10, [x]);
                    console.log("!!!!!!block call start");
                    var r = __thread_yield();
                    console.log("!!!!!!block call finish, r:", r);
                    return r;
                }
            );
        };"#;

#[test]
fn test_shell() {
    let worker_pool = Box::new(WorkerPool::new(10, 1024 * 1024, 10000, JS_WORKER_WALKER.clone()));
    worker_pool.run(JS_TASK_POOL.clone());

    register_native_object();
    register_native_function(0x1, shell_sync_call);
    register_native_function(0x10, shell_block_call);

    let tmp = JS::new(Arc::new(NativeObjsAuth::new(None, None))).unwrap();
    let test_code = Arc::new(tmp.compile("test.js".to_string(), TEST_SHELL_CODE.to_string()).unwrap());
    SHELL_MANAGER.write().unwrap().init(Some(vec![test_code]));
    SHELL_MANAGER.write().unwrap().add_string_env("_$root", "test_shell");

    let (req_sender, req_receiver) = channel();
    let (resp_sender, resp_receiver) = channel();

    let req_sender_copy = req_sender.clone();
    let resp = Arc::new(move |result: Result<Arc<Vec<u8>>>, req: Option<Box<FnBox(Arc<Vec<u8>>)>>| {
        resp_sender.send(result);
        req_sender.send(req);
    });


    let s = SHELL_MANAGER.write().unwrap().open();
    if let Some(shell) = s {
        let req = SHELL_MANAGER.write().unwrap().connect(shell, resp.clone());
        if req.is_none() {
            eprintln!("Connect Error");
        }
        req_sender_copy.send(req);

        println!("Shell v0.1");

        let mut req: Option<Box<FnBox(Arc<Vec<u8>>)>> = None;
        loop {
            print!(">");
            io::stdout().flush();

            let mut buffer = String::new();
            while let Err(e) = io::stdin().read_line(&mut buffer) {
                eprintln!("Input Error, {:?}", e);
                print!(">");
                io::stdout().flush();
            }

            if buffer.trim().as_bytes() == b"exit" {
                println!("Shell closed");
                return;
            }

            if let None = req {
                //当前没有请求回调，则接收请求回调
                match req_receiver.recv() {
                    Err(e) => {

                    },
                    Ok(new) => {
                        if new.is_none() {
                            println!("Shell closed");
                            return;
                        }
                        req = new; //更新请求回调
                    }
                }
            }

            if let Some(r) = req.take() {
                r(Arc::new(buffer.into_bytes()));
            }

            //接收请求响应
            match resp_receiver.recv() {
                Err(e) => eprintln!("Output Error, {:?}", e),
                Ok(result) => {
                    match result {
                        Err(e) => eprintln!("{:?}", e),
                        Ok(r) => println!("{output}", output = String::from_utf8_lossy(&r[..]).as_ref()),
                    }
                }
            }
        }
    }
}

//注册本地函数
fn register_native_function(id: u32, fun: fn(Arc<JS>, Vec<JSType>) -> Option<CallResult>) {
    BON_MGR.regist_fun_meta(FnMeta::CallArg(fun), id);
}

//shell同步调用
fn shell_sync_call(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    js.new_str("Hello World!".to_string());
    Some(CallResult::Ok)
}

//shell同步阻塞调用
fn shell_block_call(js: Arc<JS>, _args: Vec<JSType>) -> Option<CallResult> {
    let result = Box::new(|vm: Arc<JS>| {
        vm.new_str("Hello World!".to_string());
    });
    block_reply(js, result, Atom::from("block reply task"));
    None
}