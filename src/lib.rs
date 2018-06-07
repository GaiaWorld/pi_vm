#![crate_type = "rlib"]
#![feature(asm)]
#![feature(libc)]
#![feature(fnbox)]
#![feature(drain_filter)]
#![feature(rustc_private)]
#![feature(type_ascription)]
#![feature(duration_extras)]
#![feature(slice_internals)]
#![feature(duration_from_micros)]

extern crate core;
extern crate time;
extern crate libc;
extern crate rand;
extern crate magnetic;

#[macro_use]
extern crate lazy_static;

#[cfg(not(unix))]
extern crate kernel32;

extern crate pi_lib;
extern crate pi_base;

pub mod adapter;
pub mod native_object_impl;
pub mod pi_vm_impl;
pub mod bonmgr;