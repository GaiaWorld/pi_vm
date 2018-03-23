#![allow(dead_code, non_snake_case)]

use libc::{c_void, c_char, uint8_t, uint32_t, uint64_t, c_double, memcpy};
use std::ffi::{CStr, CString};
use std::slice::from_raw_parts;

#[link(name = "njsc")]
extern "C" {
    fn njsc_vm_new(script: *const c_char) -> *const c_void;
    fn njsc_vm_clone(template: *const c_void) -> *const c_void;
    fn njsc_get_value_type(value: *const c_void) -> uint8_t;
    fn njsc_get_boolean(value: *const c_void) -> uint8_t;
    fn njsc_get_number(value: *const c_void) -> c_double;
    fn njsc_get_string(value: *const c_void) -> *const c_char;
    fn njsc_get_object_field(vm: *const c_void, object: *const c_void, key: *const c_char) -> *const c_void;
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

/*
* js虚拟机模板
*/
pub struct JSTemplate(pub Option<*const c_void>);

impl JSTemplate {
    //构造一个指定脚本的js虚拟机模板
    pub fn new(script: String) -> Self {
        let ptr: *const c_void;
        unsafe { ptr = njsc_vm_new(CString::new(script).unwrap().as_ptr()) }
        if (ptr as usize) == 0 {
            JSTemplate(None)
        } else {
            JSTemplate(Some(ptr))
        }
    }

    //复制一个指定模板的js虚拟机
    pub fn clone(&self) -> Option<JS> {
        match self.0 {
            Some(ptr) => {
                let p: *const c_void;
                unsafe { p = njsc_vm_clone(ptr) }
                if (p as usize) == 0 {
                    None
                } else {
                    Some(JS {vm: p})
                }
            },
            _ => None,
        }
    }
}

/*
* js运行环境
*/
pub struct JS {
    vm: *const c_void,
}

impl JS {
    //构建null
    pub fn new_null(&self) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_null(self.vm) }
        JSType {
            type_id: JSValueType::Null as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建undefined
    pub fn new_undefined(&self) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_undefined(self.vm) }
        JSType {
            type_id: JSValueType::Undefined as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建boolean
    pub fn new_boolean(&self, b: bool) -> JSType {
        let ptr: *const c_void;
        unsafe {
            if b {
                ptr = njsc_new_boolean(self.vm, 1u8); 
            } else {
                ptr = njsc_new_boolean(self.vm, 0u8); 
            }
        }
        JSType {
            type_id: JSValueType::Boolean as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建i8
    pub fn new_i8(&self, num: i8) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建i16
    pub fn new_i16(&self, num: i16) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建i32
    pub fn new_i32(&self, num: i32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建i64
    pub fn new_i64(&self, num: i64) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建u8
    pub fn new_u8(&self, num: u8) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建u16
    pub fn new_u16(&self, num: u16) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建u32
    pub fn new_u32(&self, num: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建u64
    pub fn new_u64(&self, num: u64) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建f32
    pub fn new_f32(&self, num: f32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num as c_double) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建f64
    pub fn new_f64(&self, num: f64) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_number(self.vm, num) }
        JSType {
            type_id: JSValueType::Number as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建字符串
    pub fn new_str(&self, str: String) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_string(self.vm, CString::new(str).unwrap().as_ptr()) }
        JSType {
            type_id: JSValueType::String as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建对象
    pub fn new_object(&self) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_object(self.vm) }
        JSType {
            type_id: JSValueType::Object as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }
    
    //设置指定对象的域
    pub fn set_field(&self, object: &JSType, key: String, value: &JSType) {
        if (self.vm != object.js.vm) || (self.vm != value.js.vm){
            //如果对象和值不是在指定虚拟机上创建的，则忽略
            return;
        }
        unsafe { njsc_set_object_field(self.vm, object.value, CString::new(key).unwrap().as_ptr(), value.value) }
    }

    //构建数组
    pub fn new_array(&self, length: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_array(self.vm, length) }
        JSType {
            type_id: JSValueType::Array as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //设置指定数组指定偏移的值
    pub fn set_index(&self, array: &JSType, index: u32, value: &JSType) {
        if (self.vm != array.js.vm) || (self.vm != value.js.vm){
            //如果数组和值不是在指定虚拟机上创建的，则忽略
            return;
        }
        unsafe { njsc_set_array_index(self.vm, array.value, index, value.value) }
    }

    //构建ArrayBuffer
    pub fn new_array_buffer(&self, length: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_array_buffer(self.vm, length) }
        JSType {
            type_id: JSValueType::ArrayBuffer as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建Uint8Array
    pub fn new_uint8_array(&self, length: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_uint8_array(self.vm, length) }
        JSType {
            type_id: JSValueType::Uint8Array as u8,
            js: JS {vm: self.vm},
            value: ptr,
        }
    }

    //构建NativeObject
    pub fn new_native_object(&self, instance: usize) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_new_native_object(self.vm, instance as uint64_t) }
        JSType {
            type_id: JSValueType::NativeObject as u8,
            js: JS {vm: self.vm},
            value: ptr,
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
pub struct JSType {
    type_id:    u8,
    js:         JS,
    value:      *const c_void,
}

impl JSType {
    //获取指定类型的类型id
    pub fn get_type_id(value: *const c_void) -> u8 {
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
        unsafe { num = njsc_get_boolean(self.value) }
        if num == 0 {
            false
        } else {
            true
        }
    }

    //获取i8
    pub fn get_i8(&self) -> i8 {
        unsafe { njsc_get_number(self.value) as i8 }
    }

    //获取i16
	pub fn get_i16(&self) -> i16 {
        unsafe { njsc_get_number(self.value) as i16 }
    }

    //获取i32
	pub fn get_i32(&self) -> i32 {
        unsafe { njsc_get_number(self.value) as i32 }
    }

    //获取i64
	pub fn get_i64(&self) -> i64 {
        unsafe { njsc_get_number(self.value) as i64 }
    }

    //获取u8
	pub fn get_u8(&self) -> u8 {
        unsafe { njsc_get_number(self.value) as u8 }
    }

    //获取u16
	pub fn get_u16(&self) -> u16 {
        unsafe { njsc_get_number(self.value) as u16 }
    }

    //获取u32
	pub fn get_u32(&self) -> u32 {
        unsafe { njsc_get_number(self.value) as u32 }
    }

    //获取u64
	pub fn get_u64(&self) -> u64 {
        unsafe { njsc_get_number(self.value) as u64 }
    }

    //获取f32
	pub fn get_f32(&self) -> f32 {
        unsafe { njsc_get_number(self.value) as f32 }
    }

    //获取f64
	pub fn get_f64(&self) -> f64 {
        unsafe { njsc_get_number(self.value) as f64 }
    }

    //获取字符串
	pub fn get_str(&self) -> String {
        unsafe { CStr::from_ptr(njsc_get_string(self.value)).to_string_lossy().into_owned() }
    }

    //获取对象指定域的值
	pub fn get_field(&self, key: String) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_get_object_field(self.js.vm, self.value, CString::new(key).unwrap().as_ptr()) }
        JSType {
            type_id: Self::get_type_id(ptr),
            js: JS {vm: self.js.vm},
            value: ptr,
        }
    }

    //获取数组指定偏移的值
	pub fn get_index(&self, index: u32) -> JSType {
        let ptr: *const c_void;
        unsafe { ptr = njsc_get_array_index(self.js.vm, self.value, index) }
        JSType {
            type_id: Self::get_type_id(ptr),
            js: JS {vm: self.js.vm},
            value: ptr,
        }
    }

    //获取指定Buffer
	pub fn into_vec(&self) -> Vec<u8> {
        unsafe {
            let length = njsc_get_buffer_length(self.value) as usize;
            let buffer = njsc_get_buffer(self.value);
            from_raw_parts(buffer as *const u8, length).to_vec()
        }
    }

    //回收指定的Buffer
	pub fn from_vec(&self, vec: Vec<u8>) {
        unsafe {
            let length = njsc_get_buffer_length(self.value) as usize;
            let buffer = njsc_get_buffer(self.value);
            memcpy(buffer as *mut c_void, vec.as_ptr() as *const c_void, length);
        }
    }

    //获取NativeObject
	pub fn get_native_object(&self) -> usize {
        unsafe { njsc_get_native_object_instance(self.value) as usize }
    }	
}