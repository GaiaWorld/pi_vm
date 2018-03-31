use libc::{c_void, c_char, uint8_t, c_int, uint32_t, uint64_t, c_double, memcpy};
use std::ffi::{CStr, CString};
use std::slice::from_raw_parts;
use std::os::raw::c_uchar;
use std::mem::transmute;
use std::ops::Drop;
use std::ptr::null;

use data_view_impl::*;

#[link(name = "njsc")]
extern "C" {
    fn njsc_register_data_view_get_int8(func: extern fn(*mut c_void, uint64_t, uint64_t) -> c_double);
    fn njsc_register_data_view_get_int16(func: extern fn(*mut c_void, uint64_t, uint64_t, c_uchar) -> c_double);
    fn njsc_register_data_view_get_int32(func: extern fn(*mut c_void, uint64_t, uint64_t, c_uchar) -> c_double);
    fn njsc_register_data_view_get_uint8(func: extern fn(*mut c_void, uint64_t, uint64_t) -> c_double);
    fn njsc_register_data_view_get_uint16(func: extern fn(*mut c_void, uint64_t, uint64_t, c_uchar) -> c_double);
    fn njsc_register_data_view_get_uint32(func: extern fn(*mut c_void, uint64_t, uint64_t, c_uchar) -> c_double);
    fn njsc_register_data_view_get_float32(func: extern fn(*mut c_void, uint64_t, uint64_t, c_uchar) -> c_double);
    fn njsc_register_data_view_get_float64(func: extern fn(*mut c_void, uint64_t, uint64_t, c_uchar) -> c_double);
    fn njsc_register_data_view_set_int8(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double));
    fn njsc_register_data_view_set_int16(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double, c_uchar));
    fn njsc_register_data_view_set_int32(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double, c_uchar));
    fn njsc_register_data_view_set_uint8(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double));
    fn njsc_register_data_view_set_uint16(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double, c_uchar));
    fn njsc_register_data_view_set_uint32(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double, c_uchar));
    fn njsc_register_data_view_set_float32(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double, c_uchar));
    fn njsc_register_data_view_set_float64(func: extern fn(*mut c_void, uint64_t, uint64_t, c_double, c_uchar));
    fn test_main(argc: c_int, argv: *const c_void) -> c_int;
    fn njsc_vm_new(script: *const c_char) -> *const c_void;
    fn njsc_vm_clone(template: *const c_void) -> *const c_void;
    fn njsc_vm_destroy(vm: *const c_void);
    fn njsc_vm_template_destroy(template: *const c_void);
    fn njsc_get_value_type(value: *const c_void) -> uint8_t;
    fn njsc_get_boolean(value: *const c_void) -> uint8_t;
    fn njsc_get_number(value: *const c_void) -> c_double;
    fn njsc_get_string(value: *const c_void) -> *const c_char;
    fn njsc_get_object_field(vm: *const c_void, object: *const c_void, key: *const c_char) -> *const c_void;
    fn njsc_get_array_length(array: *const c_void) -> uint32_t;
    fn njsc_get_array_index(vm: *const c_void, array: *const c_void, index: uint32_t) -> *const c_void;
    fn njsc_get_buffer_length(value: *const c_void) -> uint32_t;
    fn njsc_get_buffer(value: *const c_void) -> *const c_void;
    fn njsc_get_native_object_instance(value: *const c_void) -> uint64_t;
    fn njsc_new_null(vm: *const c_void) -> *const c_void;
    fn njsc_new_undefined(vm: *const c_void) -> *const c_void;
    fn njsc_new_boolean(vm: *const c_void, b: uint8_t) -> *const c_void;
    fn njsc_new_number(vm: *const c_void, num: c_double) -> *const c_void;
    fn njsc_new_string(vm: *const c_void, str: *const c_char) -> *const c_void;
    fn njsc_new_object(vm: *const c_void) -> *const c_void;
    fn njsc_set_object_field(vm: *const c_void, object: *const c_void, key: *const c_char, value: *const c_void);
    fn njsc_new_array(vm: *const c_void, length: uint32_t) -> *const c_void;
    fn njsc_set_array_index(vm: *const c_void, array: *const c_void, index: uint32_t, value: *const c_void);
    fn njsc_new_array_buffer(vm: *const c_void, length: uint32_t) -> *const c_void;
    fn njsc_new_uint8_array(vm: *const c_void, length: uint32_t) -> *const c_void;
    fn njsc_new_native_object(vm: *const c_void, ptr: uint64_t) -> *const c_void;
}

//初始化注入DataView
pub fn register_data_view() {
    unsafe {
        njsc_register_data_view_get_int8(data_view_read_int8);
        njsc_register_data_view_get_int16(data_view_read_int16);
        njsc_register_data_view_get_int32(data_view_read_int32);
        njsc_register_data_view_get_uint8(data_view_read_uint8);
        njsc_register_data_view_get_uint16(data_view_read_uint16);
        njsc_register_data_view_get_uint32(data_view_read_uint32);
        njsc_register_data_view_get_float32(data_view_read_float32);
        njsc_register_data_view_get_float64(data_view_read_float64);
        njsc_register_data_view_set_int8(data_view_write_int8);
        njsc_register_data_view_set_int16(data_view_write_int16);
        njsc_register_data_view_set_int32(data_view_write_int32);
        njsc_register_data_view_set_uint8(data_view_write_uint8);
        njsc_register_data_view_set_uint16(data_view_write_uint16);
        njsc_register_data_view_set_uint32(data_view_write_uint32);
        njsc_register_data_view_set_float32(data_view_write_float32);
        njsc_register_data_view_set_float64(data_view_write_float64);
    }
}

//执行njsc测试代码
pub fn njsc_test_main() {
    unsafe { test_main(0, null()); }
}

/*
* js虚拟机模板
*/
pub struct JSTemplate(*const c_void);

impl Drop for JSTemplate {
    fn drop(&mut self) {
        unsafe {
            let inner = self.get_inner();
            if inner == 0 {
                return;
            }
            njsc_vm_template_destroy(inner as *const c_void);
        };
    }
}

impl JSTemplate {
    //构造一个指定脚本的js虚拟机模板
    pub fn new(script: String) -> Option<Self> {
        let ptr: *const c_void;
        unsafe { ptr = njsc_vm_new(CString::new(script).unwrap().as_ptr()) }
        if (ptr as usize) == 0 {
            None
        } else {
            Some(JSTemplate(ptr))
        }
    }

    //复制一个指定模板的js虚拟机
    pub fn clone(&self) -> Option<JS> {
        let ptr: *const c_void;
        unsafe { ptr = njsc_vm_clone(self.0) }
        if (ptr as usize) == 0 {
            None
        } else {
            Some(JS {vm: ptr as usize})
        }
    }

    //获取虚拟机模板的指针
    pub unsafe fn get_inner(&self) -> usize {
        self.0 as usize
    }
}

/*
* js运行环境
*/
#[derive(Clone)]
pub struct JS {
    vm: usize,
}

impl Drop for JS {
    fn drop(&mut self) {
        if self.vm == 0 {
            return;
        }
        unsafe { njsc_vm_destroy(self.vm as *const c_void) };
    }
}

impl JS {
    //构建null
    pub fn new_null(&self) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_null(self.vm as *const c_void) }
        JSType {
            type_id: JSValueType::Null as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建undefined
    pub fn new_undefined(&self) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_undefined(self.vm as *const c_void) }
        JSType {
            type_id: JSValueType::Undefined as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建boolean
    pub fn new_boolean(&self, b: bool) -> JSType {
        let ptr: *const c_void;
        unsafe {
            if b {
                ptr = njsc_new_boolean(self.vm as *const c_void, 1u8); 
            } else {
                ptr = njsc_new_boolean(self.vm as *const c_void, 0u8); 
            }
        }
        JSType {
            type_id: JSValueType::Boolean as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i8
    pub fn new_i8(&self, num: i8) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i16
    pub fn new_i16(&self, num: i16) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i32
    pub fn new_i32(&self, num: i32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建i64
    pub fn new_i64(&self, num: i64) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u8
    pub fn new_u8(&self, num: u8) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u16
    pub fn new_u16(&self, num: u16) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u32
    pub fn new_u32(&self, num: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建u64
    pub fn new_u64(&self, num: u64) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建f32
    pub fn new_f32(&self, num: f32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建f64
    pub fn new_f64(&self, num: f64) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm as *const c_void, num) }
        JSType {
            type_id: JSValueType::Number as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建字符串
    pub fn new_str(&self, str: String) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_string(self.vm as *const c_void, CString::new(str).unwrap().as_ptr()) }
        JSType {
            type_id: JSValueType::String as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建对象
    pub fn new_object(&self) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_object(self.vm as *const c_void) }
        JSType {
            type_id: JSValueType::Object as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }
    
    //设置指定对象的域
    pub fn set_field(&self, object: &JSType, key: String, value: &JSType) {
        if (self.vm != object.vm) || (self.vm != value.vm){
            //如果对象和值不是在指定虚拟机上创建的，则忽略
            return;
        }
        unsafe { 
            njsc_set_object_field(self.vm as *const c_void, object.value as *const c_void, CString::new(key).unwrap().as_ptr(), 
                value.value as *const c_void)
        }
    }

    //构建数组
    pub fn new_array(&self, length: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_array(self.vm as *const c_void, length) }
        JSType {
            type_id: JSValueType::Array as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //设置指定数组指定偏移的值
    pub fn set_index(&self, array: &JSType, index: u32, value: &JSType) {
        if (self.vm != array.vm) || (self.vm != value.vm){
            //如果数组和值不是在指定虚拟机上创建的，则忽略
            return;
        }
        unsafe { njsc_set_array_index(self.vm as *const c_void, array.value as *const c_void, index, value.value as *const c_void) }
    }

    //构建ArrayBuffer
    pub fn new_array_buffer(&self, length: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_array_buffer(self.vm as *const c_void, length) }
        JSType {
            type_id: JSValueType::ArrayBuffer as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建Uint8Array
    pub fn new_uint8_array(&self, length: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_uint8_array(self.vm as *const c_void, length) }
        JSType {
            type_id: JSValueType::Uint8Array as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //构建NativeObject
    pub fn new_native_object(&self, instance: usize) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_native_object(self.vm as *const c_void, instance as uint64_t) }
        JSType {
            type_id: JSValueType::NativeObject as u8,
            vm: self.vm,
            value: ptr as usize,
        }
    }
}

/*
* 值类型
*/
pub enum JSValueType {
    Null = 0x0,
    Undefined,
    Boolean,
    Number,
    String,
    ArrayBuffer = 0x8,
    Uint8Array = 0x9,
    NativeObject = 0xb,
    Object = 0x10,
    Array,
}

/*
* js类型
*/
#[derive(Clone)]
pub struct JSType {
    type_id:    u8,
    vm:         usize,
    value:      usize,
}

impl JSType {
    //获取指定类型的类型id
    fn get_type_id(value: *const c_void) -> u8 {
        unsafe { njsc_get_value_type(value) as u8 }
    }

    //判断是否是null
    pub fn is_null(&self) -> bool {
        if self.type_id == JSValueType::Null as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是undefined
	pub fn is_undefined(&self) -> bool {
        if self.type_id == JSValueType::Undefined as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是boolean
	pub fn is_boolean(&self) -> bool {
        if self.type_id == JSValueType::Boolean as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是数字
	pub fn is_number(&self) -> bool {
        if self.type_id == JSValueType::Number as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是字符串
	pub fn is_string(&self) -> bool {
        if self.type_id == JSValueType::String as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是对象
	pub fn is_object(&self) -> bool {
        if self.type_id == JSValueType::Object as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是数组
	pub fn is_array(&self) -> bool {
        if self.type_id == JSValueType::Array as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是ArrayBuffer
	pub fn is_array_buffer(&self) -> bool {
        if self.type_id == JSValueType::ArrayBuffer as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是Uint8Array
	pub fn is_uint8_array(&self) -> bool {
        if self.type_id == JSValueType::Uint8Array as u8 {
            true
        } else {
            false
        }
    }

    //判断是否是NativeObject
	pub fn is_native_object(&self) -> bool {
        if self.type_id == JSValueType::NativeObject as u8 {
            true
        } else {
            false
        }
    }

    //获取boolean
    pub fn get_boolean(&self) -> bool {
        let num: u8;
        unsafe { num = njsc_get_boolean(self.value as *const c_void) }
        if num == 0 {
            false
        } else {
            true
        }
    }

    //获取i8
    pub fn get_i8(&self) -> i8 {
        unsafe { njsc_get_number(self.value as *const c_void) as i8 }
    }

    //获取i16
	pub fn get_i16(&self) -> i16 {
        unsafe { njsc_get_number(self.value as *const c_void) as i16 }
    }

    //获取i32
	pub fn get_i32(&self) -> i32 {
        unsafe { njsc_get_number(self.value as *const c_void) as i32 }
    }

    //获取i64
	pub fn get_i64(&self) -> i64 {
        unsafe { njsc_get_number(self.value as *const c_void) as i64 }
    }

    //获取u8
	pub fn get_u8(&self) -> u8 {
        unsafe { njsc_get_number(self.value as *const c_void) as u8 }
    }

    //获取u16
	pub fn get_u16(&self) -> u16 {
        unsafe { njsc_get_number(self.value as *const c_void) as u16 }
    }

    //获取u32
	pub fn get_u32(&self) -> u32 {
        unsafe { njsc_get_number(self.value as *const c_void) as u32 }
    }

    //获取u64
	pub fn get_u64(&self) -> u64 {
        unsafe { njsc_get_number(self.value as *const c_void) as u64 }
    }

    //获取f32
	pub fn get_f32(&self) -> f32 {
        unsafe { njsc_get_number(self.value as *const c_void) as f32 }
    }

    //获取f64
	pub fn get_f64(&self) -> f64 {
        unsafe { njsc_get_number(self.value as *const c_void) as f64 }
    }

    //获取字符串
	pub fn get_str(&self) -> String {
        unsafe { CStr::from_ptr(njsc_get_string(self.value as *const c_void)).to_string_lossy().into_owned() }
    }

    //获取对象指定域的值
	pub fn get_field(&self, key: String) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_get_object_field(self.vm as *const c_void, self.value as *const c_void, CString::new(key).unwrap().as_ptr()) }
        JSType {
            type_id: Self::get_type_id(ptr),
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取数组长度
    pub fn get_array_length(&self) -> usize {
        unsafe { njsc_get_array_length(self.value as *const c_void) as usize }
    }

    //获取数组指定偏移的值
	pub fn get_index(&self, index: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_get_array_index(self.vm as *const c_void, self.value as *const c_void, index) }
        JSType {
            type_id: Self::get_type_id(ptr),
            vm: self.vm,
            value: ptr as usize,
        }
    }

    //获取指定Buffer的引用
    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            let length = njsc_get_buffer_length(self.value as *const c_void) as usize;
            let buffer = njsc_get_buffer(self.value as *const c_void);
            from_raw_parts(buffer as *const u8, length)
        }
    }

    //获取指定Buffer的复制
	pub fn into_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    //重置指定的Buffer
	pub fn from_vec(&self, vec: Vec<u8>) {
        unsafe {
            let length = njsc_get_buffer_length(self.value as *const c_void) as usize;
            let buffer = njsc_get_buffer(self.value as *const c_void);
            memcpy(buffer as *mut c_void, vec.as_ptr() as *const c_void, length);
        }
    }

    //获取指定的Buffer
    pub fn into_buffer(&self) -> JSBuffer {
        unsafe {
            let length = njsc_get_buffer_length(self.value as *const c_void) as usize;
            let buffer = njsc_get_buffer(self.value as *const c_void);
            JSBuffer {
                buffer: buffer as *mut c_void,
                len: length,
            }
        }
    }

    //获取NativeObject
	pub fn get_native_object(&self) -> usize {
        unsafe { njsc_get_native_object_instance(self.value as *const c_void) as usize }
    }
}

/*
* Js Buffer
*/
pub struct JSBuffer {
    buffer: *mut c_void,
    len: usize,
}

impl JSBuffer {
    //构建JSBuffer
    pub fn new(ptr: *mut c_void, len: usize) -> Self {
        JSBuffer {
            buffer: ptr,
            len: len,
        }
    }

    //获取buffer长度
    pub fn len(&self) -> usize {
        self.len
    }

    //在指定位置读小端i8
    pub fn read_i8(&self, offset: usize) -> i8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i8::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i8)) }
    }

    //在指定位置读小端i16
    pub fn read_i16(&self, offset: usize) -> i16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i16::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i16)) }
    }

    //在指定位置读小端i32
    pub fn read_i32(&self, offset: usize) -> i32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i32::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i32)) }
    }

    //在指定位置读小端i64
    pub fn read_i64(&self, offset: usize) -> i64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i64::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut i64)) }
    }

    //在指定位置读小端u8
    pub fn read_u8(&self, offset: usize) -> u8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u8::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u8)) }
    }

    //在指定位置读小端u16
    pub fn read_u16(&self, offset: usize) -> u16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u16::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u16)) }
    }

    //在指定位置读小端u32
    pub fn read_u32(&self, offset: usize) -> u32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u32::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u32)) }
    }

    //在指定位置读小端u64
    pub fn read_u64(&self, offset: usize) -> u64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u64::from_le(*(self.buffer.wrapping_offset(offset as isize) as *mut u64)) }
    }

    //在指定位置读小端f32
    pub fn read_f32(&self, offset: usize) -> f32 {
        unsafe { transmute::<u32, f32>(self.read_u32(offset)) }
    }

    //在指定位置读小端f64
    pub fn read_f64(&self, offset: usize) -> f64 {
        unsafe { transmute::<u64, f64>(self.read_u64(offset)) }
    }

    //在指定位置读大端i8
    pub fn read_i8_be(&self, offset: usize) -> i8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i8::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i8)) }
    }

    //在指定位置读大端i16
    pub fn read_i16_be(&self, offset: usize) -> i16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i16::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i16)) }
    }

    //在指定位置读大端i32
    pub fn read_i32_be(&self, offset: usize) -> i32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i32::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i32)) }
    }

    //在指定位置读大端i64
    pub fn read_i64_be(&self, offset: usize) -> i64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { i64::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut i64)) }
    }

    //在指定位置读大端u8
    pub fn read_u8_be(&self, offset: usize) -> u8 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u8::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u8)) }
    }

    //在指定位置读大端u16
    pub fn read_u16_be(&self, offset: usize) -> u16 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u16::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u16)) }
    }

    //在指定位置读大端u32
    pub fn read_u32_be(&self, offset: usize) -> u32 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u32::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u32)) }
    }

    //在指定位置读大端u64
    pub fn read_u64_be(&self, offset: usize) -> u64 {
        if offset >= self.len {
            panic!("access out of range");
        }
        unsafe { u64::from_be(*(self.buffer.wrapping_offset(offset as isize) as *mut u64)) }
    }

    //在指定位置读大端f32
    pub fn read_f32_be(&self, offset: usize) -> f32 {
        unsafe { transmute::<u32, f32>(self.read_u32_be(offset)) }
    }

    //在指定位置读小端f64
    pub fn read_f64_be(&self, offset: usize) -> f64 {
        unsafe { transmute::<u64, f64>(self.read_u64_be(offset)) }
    }

    //在指定位置读字节数组
    pub fn read(&self, offset: usize, len: usize) -> &[u8] {
        if offset + len > self.len {
            panic!("access out of range");
        }
        unsafe {
            from_raw_parts(self.buffer.wrapping_offset(offset as isize) as *const u8, len)
        }
    }

    //在指定位置写小端i8
    pub fn write_i8(&mut self, offset: usize, v: i8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i8) = v.to_le() }
        last as isize
    }

    //在指定位置写小端i16
    pub fn write_i16(&mut self, offset: usize, v: i16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i16) = v.to_le() }
        last as isize
    }

    //在指定位置写小端i32
    pub fn write_i32(&mut self, offset: usize, v: i32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i32) = v.to_le() }
        last as isize
    }

    //在指定位置写小端i64
    pub fn write_i64(&mut self, offset: usize, v: i64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i64) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u8
    pub fn write_u8(&mut self, offset: usize, v: u8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u8) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u16
    pub fn write_u16(&mut self, offset: usize, v: u16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u16) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u32
    pub fn write_u32(&mut self, offset: usize, v: u32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u32) = v.to_le() }
        last as isize
    }

    //在指定位置写小端u64
    pub fn write_u64(&mut self, offset: usize, v: u64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u64) = v.to_le() }
        last as isize
    }

    //在指定位置写小端f32
    pub fn write_f32(&mut self, offset: usize, v: f32) -> isize {
        self.write_u32(offset, unsafe { transmute::<f32, u32>(v) })
    }

    //在指定位置写小端f64
    pub fn write_f64(&mut self, offset: usize, v: f64) -> isize {
        self.write_u64(offset, unsafe { transmute::<f64, u64>(v) })
    }

    //在指定位置写大端i8
    pub fn write_i8_be(&mut self, offset: usize, v: i8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i8) = v.to_be() }
        last as isize
    }

    //在指定位置写大端i16
    pub fn write_i16_be(&mut self, offset: usize, v: i16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i16) = v.to_be() }
        last as isize
    }

    //在指定位置写大端i32
    pub fn write_i32_be(&mut self, offset: usize, v: i32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i32) = v.to_be() }
        last as isize
    }

    //在指定位置写大端i64
    pub fn write_i64_be(&mut self, offset: usize, v: i64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut i64) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u8
    pub fn write_u8_be(&mut self, offset: usize, v: u8) -> isize {
        let last = offset + 1;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u8) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u16
    pub fn write_u16_be(&mut self, offset: usize, v: u16) -> isize {
        let last = offset + 2;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u16) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u32
    pub fn write_u32_be(&mut self, offset: usize, v: u32) -> isize {
        let last = offset + 4;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u32) = v.to_be() }
        last as isize
    }

    //在指定位置写大端u64
    pub fn write_u64_be(&mut self, offset: usize, v: u64) -> isize {
        let last = offset + 8;
        if last > self.len {
            return -1;
        }
        unsafe { *(self.buffer.wrapping_offset(offset as isize) as *mut u64) = v.to_be() }
        last as isize
    }

    //在指定位置写大端f32
    pub fn write_f32_be(&mut self, offset: usize, v: f32) -> isize {
        self.write_u32_be(offset, unsafe { transmute::<f32, u32>(v) })
    }

    //在指定位置写大端f64
    pub fn write_f64_be(&mut self, offset: usize, v: f64) -> isize {
        self.write_u64_be(offset, unsafe { transmute::<f64, u64>(v) })
    }

    //从指定位置写字节数组
    pub fn write(&mut self, offset: usize, v: &[u8]) -> isize {
        let len = v.len();
        let last = offset + len;
        if  last > self.len {
            return -1;
        }
        unsafe { memcpy(self.buffer.wrapping_offset(offset as isize), v.as_ptr() as *const c_void, len); }
        last as isize
    }
}

