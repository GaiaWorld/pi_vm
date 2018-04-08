#![crate_type = "rlib"]
#![feature(libc)]
#![feature(fnbox)]
#![feature(rustc_private)]
#![feature(type_ascription)]

extern crate time;
extern crate libc;
extern crate rand;
extern crate threadpool;

#[macro_use]
extern crate lazy_static;

pub mod data_view_impl;
pub mod adapter;
pub mod util;
pub mod worker;
pub mod worker_pool;
pub mod task;
pub mod task_pool;
pub mod bonmgr;
