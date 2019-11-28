#![crate_type = "rlib"]
#![feature(asm)]
#![feature(libc)]
#![feature(drain_filter)]
#![feature(rustc_private)]
#![feature(type_ascription)]
#![feature(slice_internals)]
#![feature(proc_macro_hygiene)]

extern crate core;
extern crate libc;
extern crate rand;

#[macro_use]
extern crate lazy_static;

#[cfg(not(unix))]
extern crate kernel32;

extern crate crossbeam_channel;

#[macro_use]
extern crate log;

extern crate flame;
#[macro_use]
extern crate flamer;

extern crate atom;
extern crate apm;
extern crate worker;
extern crate timer;
extern crate handler;
extern crate gray;
extern crate hash;
extern crate lfstack;
extern crate parking_lot;

pub mod adapter;
pub mod native_object_impl;
pub mod pi_vm_impl;
pub mod bonmgr;
pub mod channel_map;
pub mod shell;
pub mod proc;
pub mod proc_pool;
pub mod duk_proc;