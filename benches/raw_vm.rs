#![feature(test)]
#![deny(warnings)]

extern crate test;

extern crate pi_vm;

use std::sync::Arc;
use std::fs::File;
use std::io::prelude::*;

use test::Bencher;

use pi_vm::bonmgr::NativeObjsAuth;
use pi_vm::adapter::{register_native_object, JS};

//虚拟机创建
#[bench]
fn create_vm(b: &mut Bencher) {
    register_native_object();

    b.iter(|| {
        if let None = JS::new(Arc::new(NativeObjsAuth::new(None, None))) {
            panic!("!!!> Create Vm Error");
        }
    });
}

//虚拟机编译小代码
#[bench]
fn vm_compile_small(b: &mut Bencher) {
    register_native_object();

    let file_name = &String::from("first.js");
    if let Ok(mut file) = File::open("benches/first.js") {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(ref js) = JS::new(Arc::new(NativeObjsAuth::new(None, None))) {
                b.iter(|| {
                    if let None = js.compile(file_name.clone(), (&contents).clone()) {
                        panic!("!!!> Vm Compile Error");
                    }
                });
            }
        }
    }
}

//虚拟机编译大代码
#[bench]
fn vm_compile_big(b: &mut Bencher) {
    register_native_object();

    let file_name = &String::from("core.js");
    if let Ok(mut file) = File::open("benches/core.js") {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(ref js) = JS::new(Arc::new(NativeObjsAuth::new(None, None))) {
                b.iter(|| {
                    if let None = js.compile(file_name.clone(), (&contents).clone()) {
                        panic!("!!!> Vm Compile Error");
                    }
                });
            }
        }
    }
}

//虚拟机加载并运行小字节码
#[bench]
fn vm_load_small(b: &mut Bencher) {
    register_native_object();

    let file_name = &String::from("first.js");
    if let Ok(mut file) = File::open("benches/first.js") {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(ref js) = JS::new(Arc::new(NativeObjsAuth::new(None, None))) {
                if let Some(ref code) = js.compile(file_name.clone(), (&contents).clone()) {
                    b.iter(|| {
                       js.load(code);
                    });
                }
            }
        }
    }
}

//虚拟机加载并运行大字节码
#[bench]
fn vm_load_big(b: &mut Bencher) {
    register_native_object();

    let file_name = &String::from("core.js");
    if let Ok(mut file) = File::open("benches/core.js") {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(ref js) = JS::new(Arc::new(NativeObjsAuth::new(None, None))) {
                if let Some(ref code) = js.compile(file_name.clone(), (&contents).clone()) {
                    b.iter(|| {
                        js.load(code);
                    });
                }
            }
        }
    }
}

