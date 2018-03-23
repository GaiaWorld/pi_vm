#[cfg(test)]
extern crate pi_vm;

use pi_vm::adapter::{JSTemplate, JS, JSType};

#[test]
fn base_test() {
    let js = JSTemplate::new("var obj = {}; console.log(\"!!!!!!obj: \" + obj);".to_string());
    assert!(js.0.is_some());
    let copy: JS = js.clone().unwrap();
    let val = copy.new_null();
    assert!(val.is_null());
    let val = copy.new_undefined();
    assert!(val.is_undefined());
    let val = copy.new_boolean(true);
    assert!(val.is_boolean() && val.get_boolean());
    let val = copy.new_boolean(false);
    assert!(val.is_boolean() && !val.get_boolean());
    let val = copy.new_i8(255i8);
    assert!(val.is_number() && val.get_i8() == 255i8);
    let val = copy.new_i16(65535i16);
    assert!(val.is_number() && val.get_i16() == 65535i16);
    let val = copy.new_i32(0xffffffffi32);
    assert!(val.is_number() && val.get_i32() == 0xffffffffi32);
    let val = copy.new_u8(255u8);
    assert!(val.is_number() && val.get_u8() == 255u8);
    let val = copy.new_u16(65535u16);
    assert!(val.is_number() && val.get_u16() == 65535u16);
    let val = copy.new_u32(0xffffffffu32);
    assert!(val.is_number() && val.get_u32() == 0xffffffffu32);
    let val = copy.new_f32(0.0173136f32);
    assert!(val.is_number() && val.get_f32() == 0.0173136f32);
    let val = copy.new_f64(921.1356737853f64);
    assert!(val.is_number() && val.get_f64() == 921.1356737853f64);
    let val = copy.new_str("Hello World".to_string());
    assert!(val.is_string() && val.get_str() == "Hello World".to_string());
    let val = copy.new_str("Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    assert!(val.is_string() && val.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    let object = copy.new_object();
    assert!(object.is_object());
    copy.set_field(&object, "x".to_string(), &val);
    let tmp = object.get_field("x".to_string());
    assert!(object.is_object() && tmp.is_string() && tmp.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    let tmp = object.get_field("c".to_string());
    assert!(object.is_object() && tmp.is_null()); //key不存在
    let array = copy.new_array(10);
    assert!(array.is_array());
    copy.set_index(&array, 3, &object);
    copy.set_index(&array, 30, &val); //数组自动扩容
    let tmp = array.get_index(3);
    assert!(array.is_array() && tmp.is_object() && tmp.get_field("x".to_string()).get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    let tmp = array.get_index(30);
    assert!(array.is_array() && tmp.is_string() && tmp.get_str() == "Hello Hello Hello Hello Hello Hello你好^)(*&^%%$#^\r\n".to_string());
    let tmp = array.get_index(0);
    assert!(array.is_array() && tmp.is_null()); //index不存在
    let val = copy.new_array_buffer(32);
    let mut tmp = val.into_vec();
    assert!(val.is_array_buffer() && tmp.capacity() == 32 && tmp.len() == 32);
    println!("buffer: {:?}", tmp);
    for i in 0..tmp.len() {
        tmp[i] = 255;
    }
    val.from_vec(tmp);
    let tmp = val.into_vec();
    assert!(val.is_array_buffer() && tmp.capacity() == 32 && tmp.len() == 32);
    println!("buffer: {:?}", tmp);
    let val = copy.new_uint8_array(10);
    let mut tmp = val.into_vec();
    assert!(val.is_uint8_array() && tmp.capacity() == 10 && tmp.len() == 10);
    println!("buffer: {:?}", tmp);
    for i in 0..tmp.len() {
        tmp[i] = 255;
    }
    val.from_vec(tmp);
    let mut tmp = val.into_vec();
    assert!(val.is_uint8_array() && tmp.capacity() == 10 && tmp.len() == 10);
    println!("buffer: {:?}", tmp);
    let val = copy.new_native_object(0xffffffffusize);
    assert!(val.is_native_object() && val.get_native_object() == 0xffffffffusize);
}