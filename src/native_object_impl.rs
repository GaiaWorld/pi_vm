use std::sync::Arc;
use std::ffi::CString;

use libc::{c_void as c_void_ptr, c_char, c_int};

use atom::Atom;
use apm::counter::{GLOBAL_PREF_COLLECT, PrefCounter, PrefTimer};
use worker::task::TaskType;

use bonmgr::{CallResult, bon_call};
use adapter::{JSStatus, JS, JSType, dukc_vm_status_switch, dukc_throw, dukc_switch_context};

lazy_static! {
    //虚拟机同步调用数量
    static ref VM_SYNC_CALL_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_sync_call_count"), 0).unwrap();
    //虚拟机同步阻塞调用数量
    static ref VM_BLOCK_CALL_COUNT: PrefCounter = GLOBAL_PREF_COLLECT.new_static_counter(Atom::from("vm_block_call_count"), 0).unwrap();
}

//调用NativeObject函数
#[no_mangle]
pub extern "C" fn native_object_function_call(
    handler: *const c_void_ptr,
    hash: u32, 
    args_size: u32,
    args_type: *const c_void_ptr,
    args: *const c_void_ptr) -> c_int {
        let js = unsafe { JS::from_raw(handler) };
        let vm = unsafe { js.get_vm() };
        unsafe { dukc_switch_context(vm); }
        let vec = args_to_vec(vm, args_size, args_type as *const u8, args as *const u32);
        match bon_call(js.clone(), hash, vec) {
            Some(CallResult::Ok) => {
                VM_SYNC_CALL_COUNT.sum(1);

                unsafe { dukc_switch_context(vm); }
                Arc::into_raw(js);
                return 1
            },
            Some(CallResult::Err(reason)) => {
                VM_SYNC_CALL_COUNT.sum(1);

                unsafe {
                    let reason_ptr = CString::into_raw(CString::new(reason).unwrap());
                    dukc_switch_context(vm); //必须先切换上下文，再抛出异常
                    dukc_throw(vm, reason_ptr as *const c_char);
                    CString::from_raw(reason_ptr);
                }
                Arc::into_raw(js);
                return 0;
            }
            None => {
                //没有立即返回，则表示会阻塞，并异步返回
                VM_BLOCK_CALL_COUNT.sum(1);

                unsafe {
                    dukc_switch_context(vm);
                    Arc::into_raw(js);
                    if dukc_vm_status_switch(vm, JSStatus::SingleTask as i8, JSStatus::WaitBlock as i8) == JSStatus::SingleTask as i8 {
                        //改变状态成功，防止虚拟机在当前同步任务完成后被立即回收，回收权利交由异步任务
                        return 0;
                    } else {
                        return -1;
                    }
                }
            },
        }
}

//转换参数
fn args_to_vec(vm: *const c_void_ptr, args_size: u32, args_type: *const u8, args: *const u32) -> Option<Vec<JSType>> {
    if args_size == 0 {
        return None;
    }
    
    let mut type_id: u8;
    let mut arg: u32;
    let mut vec = Vec::new();
    for offset in 0..args_size {
        unsafe {
            type_id = args_type.wrapping_offset(offset as isize).read();
            arg = args.wrapping_offset(offset as isize).read();
            vec.insert(offset as usize, JSType::new(type_id, false, vm, arg as *const c_void_ptr));
        }
    }
    Some(vec)
}

//释放指定虚拟机对应的NativeObject实例
#[no_mangle]
pub extern "C" fn native_object_function_free(ptr: *const c_void_ptr, size: u32) {
    let mut vec = Vec::with_capacity(size as usize);
    let instances = ptr as *const u64;
    for offset in 0..size {
        vec.insert(offset as usize, unsafe { instances.wrapping_offset(offset as isize).read() });
    }
    //TODO 调用实际的free函数...
}
