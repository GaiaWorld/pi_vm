#![crate_type = "rlib"]
#![feature(libc)]
#![feature(fnbox)]
#![feature(drain_filter)]
#![feature(rustc_private)]
#![feature(type_ascription)]
#![feature(duration_extras)]
#![feature(slice_internals)]
#![feature(duration_from_micros)]

extern crate fnv;
extern crate core;
extern crate time;
extern crate libc;
extern crate rand;
extern crate threadpool;

#[macro_use]
extern crate lazy_static;

#[cfg(not(unix))]
extern crate kernel32;

pub mod adapter;
pub mod util;
pub mod worker;
pub mod worker_pool;
pub mod task;
pub mod task_pool;
pub mod native_object_impl;
pub mod pi_vm_impl;
pub mod bonmgr;