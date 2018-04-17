#![crate_type = "rlib"]
#![feature(libc)]
#![feature(fnbox)]
#![feature(drain_filter)]
#![feature(rustc_private)]
#![feature(type_ascription)]
#![feature(duration_extras)]
#![feature(slice_internals)]

extern crate core;
extern crate time;
extern crate libc;
extern crate rand;
extern crate threadpool;

#[macro_use]
extern crate lazy_static;

pub mod adapter;
pub mod util;
pub mod worker;
pub mod worker_pool;
pub mod task;
pub mod task_pool;
pub mod data_view_impl;
pub mod native_object_impl;
pub mod pi_vm_impl;
pub mod bonmgr;
