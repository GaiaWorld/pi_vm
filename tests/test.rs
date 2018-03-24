#[cfg(test)]
extern crate pi_vm;

use pi_vm::adapter::{JSTemplate, JS};

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
    let val = copy.new_i8(0x7fi8);
    assert!(val.is_number() && val.get_i8() == 0x7fi8);
    let val = copy.new_i16(0x7fffi16);
    assert!(val.is_number() && val.get_i16() == 0x7fffi16);
    let val = copy.new_i32(0x7fffffffi32);
    assert!(val.is_number() && val.get_i32() == 0x7fffffffi32);
    let val = copy.new_i64(0x7199254740992i64);
    assert!(val.is_number() && val.get_i64() == 0x7199254740992i64);
    let val = copy.new_u8(255u8);
    assert!(val.is_number() && val.get_u8() == 255u8);
    let val = copy.new_u16(65535u16);
    assert!(val.is_number() && val.get_u16() == 65535u16);
    let val = copy.new_u32(0xffffffffu32);
    assert!(val.is_number() && val.get_u32() == 0xffffffffu32);
    let val = copy.new_u64(9007199254740992u64);
    assert!(val.is_number() && val.get_u64() == 9007199254740992u64);
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
    assert!(array.is_array() && array.get_array_length() == 0);
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
        tmp[i] = 10;
    }
    val.from_vec(tmp);
    let tmp = val.to_bytes();
    assert!(val.is_array_buffer() && tmp.len() == 32);
    println!("buffer: {:?}", tmp);
    let mut tmp = val.into_buffer();
    assert!(val.is_array_buffer() && tmp.len() == 32);
    tmp.write_i8(0, 0x7f);
    assert!(tmp.read_i8(0) == 0x7f);
    tmp.write_i16(1, 0x7fff);
    assert!(tmp.read_i16(1) == 0x7fff);
    tmp.write_i32(3, 0x7fffffff);
    assert!(tmp.read_i32(3) == 0x7fffffff);
    tmp.write_i64(7, 0x7fffffffffffffff);
    assert!(tmp.read_i64(7) == 0x7fffffffffffffff);
    tmp.write_u8(15, 0xff);
    assert!(tmp.read_u8(15) == 0xff);
    tmp.write_u16(16, 0xffff);
    assert!(tmp.read_u16(16) == 0xffff);
    tmp.write_u32(18, 0xffffffff);
    assert!(tmp.read_u32(18) == 0xffffffff);
    tmp.write_u64(22, 0xffffffffffffffff);
    assert!(tmp.read_u64(22) == 0xffffffffffffffff);
    tmp.write_f32(18, 0.7891312);
    assert!(tmp.read_f32(18) == 0.7891312);
    tmp.write_f64(22, 0.999999999999);
    assert!(tmp.read_f64(22) == 0.999999999999);
    println!("buffer: {:?}", tmp.read(0, 32));
    tmp.write_i8_be(0, 0x7f);
    assert!(tmp.read_i8_be(0) == 0x7f);
    tmp.write_i16_be(1, 0x7fff);
    assert!(tmp.read_i16_be(1) == 0x7fff);
    tmp.write_i32_be(3, 0x7fffffff);
    assert!(tmp.read_i32_be(3) == 0x7fffffff);
    tmp.write_i64_be(7, 0x7fffffffffffffff);
    assert!(tmp.read_i64_be(7) == 0x7fffffffffffffff);
    tmp.write_u8_be(15, 0xff);
    assert!(tmp.read_u8_be(15) == 0xff);
    tmp.write_u16_be(16, 0xffff);
    assert!(tmp.read_u16_be(16) == 0xffff);
    tmp.write_u32_be(18, 0xffffffff);
    assert!(tmp.read_u32_be(18) == 0xffffffff);
    tmp.write_u64_be(22, 0xffffffffffffffff);
    assert!(tmp.read_u64_be(22) == 0xffffffffffffffff);
    tmp.write_f32_be(18, 0.7891312);
    assert!(tmp.read_f32_be(18) == 0.7891312);
    tmp.write_f64_be(22, 0.999999999999);
    assert!(tmp.read_f64_be(22) == 0.999999999999);
    println!("buffer: {:?}", tmp.read(0, 32));
    tmp.write(0, &[100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100]);
    println!("buffer: {:?}", tmp.read(0, 32));
    let val = copy.new_uint8_array(10);
    let mut tmp = val.into_vec();
    assert!(val.is_uint8_array() && tmp.capacity() == 10 && tmp.len() == 10);
    println!("buffer: {:?}", tmp);
    for i in 0..tmp.len() {
        tmp[i] = 255;
    }
    val.from_vec(tmp);
    let tmp = val.to_bytes();
    assert!(val.is_uint8_array() && tmp.len() == 10);
    println!("buffer: {:?}", tmp);
    let val = copy.new_native_object(0xffffffffusize);
    assert!(val.is_native_object() && val.get_native_object() == 0xffffffffusize);
}
