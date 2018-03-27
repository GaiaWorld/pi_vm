#![crate_type = "rlib"]
#![feature(libc)]
#![feature(rustc_private)]
#![feature(type_ascription)]

extern crate libc;

pub mod data_view_impl;
pub mod adapter;
