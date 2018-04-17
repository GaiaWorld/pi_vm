use std::vec::Vec;
use std::mem::forget;
use std::os::raw::c_uchar;
use libc::{c_void, uint32_t, uint64_t, c_double};

use adapter::JSBuffer;

//从指定的缓冲区读出一个有符号1字节整数
#[no_mangle]
pub extern "C" fn data_view_read_int8 (buf: *mut c_void, length: uint64_t, offset: uint64_t) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    buffer.read_i8(offset as usize) as f64
}

//从指定的缓冲区读出一个有符号2字节整数
#[no_mangle]
pub extern "C" fn data_view_read_int16 (buf: *mut c_void, length: uint64_t, offset: uint64_t, is_le: c_uchar) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.read_i16(offset as usize) as f64
    } else {
        buffer.read_i16_be(offset as usize) as f64
    }
}

//从指定的缓冲区读出一个有符号4字节整数
#[no_mangle]
pub extern "C" fn data_view_read_int32 (buf: *mut c_void, length: uint64_t, offset: uint64_t, is_le: c_uchar) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.read_i32(offset as usize) as f64
    } else {
        buffer.read_i32_be(offset as usize) as f64
    }
}

//从指定的缓冲区读出一个无符号1字节整数
#[no_mangle]
pub extern "C" fn data_view_read_uint8 (buf: *mut c_void, length: uint64_t, offset: uint64_t) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    buffer.read_u8(offset as usize) as f64
}

//从指定的缓冲区读出一个无符号2字节整数
#[no_mangle]
pub extern "C" fn data_view_read_uint16 (buf: *mut c_void, length: uint64_t, offset: uint64_t, is_le: c_uchar) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.read_u16(offset as usize) as f64
    } else {
        buffer.read_u16_be(offset as usize) as f64
    }
}

//从指定的缓冲区读出一个无符号4字节整数
#[no_mangle]
pub extern "C" fn data_view_read_uint32 (buf: *mut c_void, length: uint64_t, offset: uint64_t, is_le: c_uchar) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.read_u32(offset as usize) as f64
    } else {
        buffer.read_u32_be(offset as usize) as f64
    }
}

//从指定的缓冲区读出一个4字节浮点数
#[no_mangle]
pub extern "C" fn data_view_read_float32 (buf: *mut c_void, length: uint64_t, offset: uint64_t, is_le: c_uchar) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.read_f32(offset as usize) as f64
    } else {
        buffer.read_f32_be(offset as usize) as f64
    }
}

//从指定的缓冲区读出一个8字节浮点数
#[no_mangle]
pub extern "C" fn data_view_read_float64 (buf: *mut c_void, length: uint64_t, offset: uint64_t, is_le: c_uchar) -> c_double {
    let buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.read_f64(offset as usize) as f64
    } else {
        buffer.read_f64_be(offset as usize) as f64
    }
}

//向指定的缓冲区写入一个有符号1字节整数
#[no_mangle]
pub extern "C" fn data_view_write_int8 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    buffer.write_i8(offset as usize, value as i8);
    
}

//向指定的缓冲区写入一个有符号2字节整数
#[no_mangle]
pub extern "C" fn data_view_write_int16 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double, is_le: c_uchar) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.write_i16(offset as usize, value as i16);
    } else {
        buffer.write_i16_be(offset as usize, value as i16);
    }
    
}

//向指定的缓冲区写入一个有符号4字节整数
#[no_mangle]
pub extern "C" fn data_view_write_int32 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double, is_le: c_uchar) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.write_i32(offset as usize, value as i32);
    } else {
        buffer.write_i32_be(offset as usize, value as i32);
    }
    
}

//向指定的缓冲区写入一个无符号1字节整数
#[no_mangle]
pub extern "C" fn data_view_write_uint8 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    buffer.write_u8(offset as usize, value as u8);
    
}

//向指定的缓冲区写入一个无符号2字节整数
#[no_mangle]
pub extern "C" fn data_view_write_uint16 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double, is_le: c_uchar) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.write_u16(offset as usize, value as u16);
    } else {
        buffer.write_u16_be(offset as usize, value as u16);
    }
    
}

//向指定的缓冲区写入一个无符号4字节整数
#[no_mangle]
pub extern "C" fn data_view_write_uint32 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double, is_le: c_uchar) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.write_u32(offset as usize, value as u32);
    } else {
        buffer.write_u32_be(offset as usize, value as u32);
    }
    
}

//向指定的缓冲区写入一个4字节浮点数
#[no_mangle]
pub extern "C" fn data_view_write_float32 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double, is_le: c_uchar) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.write_f32(offset as usize, value as f32);
    } else {
        buffer.write_f32_be(offset as usize, value as f32);
    }
    
}

//向指定的缓冲区写入一个8字节浮点数
#[no_mangle]
pub extern "C" fn data_view_write_float64 (buf: *mut c_void, length: uint64_t, offset: uint64_t, value: c_double, is_le: c_uchar) {
    let mut buffer: JSBuffer = JSBuffer::new(buf, length as usize, length as usize);
    let endian = is_le as u8;
    if endian > 0 {
        buffer.write_f64(offset as usize, value as f64);
    } else {
        buffer.write_f64_be(offset as usize, value as f64);
    }
    
}

#[repr(C)] 
#[derive(Debug, Copy, Clone)] 
pub struct NativeObjectCallback { 
	pub callback: Option<unsafe extern "C" fn(arg1: uint64_t , arg2: uint64_t , arg3: uint64_t)>, 
	pub arg: *mut c_void,
}

//创建一个指定id的NativeObject
#[no_mangle]
pub extern "C" fn rs_native_object_create (id: uint32_t, 
    buf: *mut c_void, buf_length: uint64_t, 
    objs: *mut c_void, objs_length: uint64_t) -> usize {

    let buffer: JSBuffer = JSBuffer::new(buf, buf_length as usize, buf_length as usize);
    let array = to_usize_array(objs as *mut usize, objs_length as usize);

    let instance: usize = 0;
    //TODO...
    (id, buffer);
    
    into_usize_ptr(array);
    instance
}

//同步创建一个指定id的NativeObject
#[no_mangle]
pub extern "C" fn rs_native_object_sync_create (id: uint32_t, 
    buf: *mut c_void, buf_length: uint64_t, 
    objs: *mut c_void, objs_length: uint64_t, 
    callback: *mut c_void) {

    let buffer: JSBuffer = JSBuffer::new(buf, buf_length as usize, buf_length as usize);
    let array = to_usize_array(objs as *mut usize, objs_length as usize);
    let cb: Box<NativeObjectCallback> = void2box::<NativeObjectCallback>(callback);
    //TODO...
    (id, buffer, cb);
    
    into_usize_ptr(array);
}

#[inline]
pub fn to_usize_array(ptr: *mut usize, length: usize) -> Vec<usize> {
    unsafe { Vec::from_raw_parts(ptr, length, length) }
}

#[inline]
pub fn into_usize_ptr(mut array: Vec<usize>) -> *mut usize {
    let ptr = array.as_mut_ptr();
    forget(array);
    ptr
}

#[inline]
pub fn box2void<T>(ptr_box: Box<T>) -> *const c_void {
    Box::into_raw(ptr_box) as *const c_void
}

#[inline]
pub fn void2box<T>(ptr_void: *mut c_void) -> Box<T> {
    unsafe { Box::from_raw(ptr_void as *mut T) }
}