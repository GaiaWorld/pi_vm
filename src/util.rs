use time;
use libc::c_void;
use std::sync::Arc;

/*
* 获取当前本地时间的秒数
*/
pub fn now_second() -> i64 {
	time::get_time().sec
}

/*
* 获取当前本地时间的毫秒数
*/
pub fn now_millisecond() -> i64 {
	time::get_time().sec * 1000 + (time::get_time().nsec / 1000000) as i64
}

/*
* 获取当前本地时间的微秒数
*/
pub fn now_microsecond() -> i64 {
	time::get_time().sec * 1000000 + (time::get_time().nsec / 1000) as i64
}

/*
* 获取当前本地时间的纳秒数
*/
pub fn now_nanosecond() -> i128 {
    (time::get_time().sec * 1000000000) as i128 + time::get_time().nsec as i128
}

/*
* 将box转换为*const c_void
*/
#[inline]
pub fn box2void<T>(ptr_box: Box<T>) -> *const c_void {
    Box::into_raw(ptr_box) as *const c_void
}

/*
* 将*mut c_void转换为box
*/
#[inline]
pub fn void2box<T>(ptr_void: *mut c_void) -> Box<T> {
    unsafe { Box::from_raw(ptr_void as *mut T) }
}

/*
* 将Arc转换为*const c_void
*/
#[inline]
pub fn arc2void<T>(ptr_box: Arc<T>) -> *const c_void {
    Arc::into_raw(ptr_box) as *const c_void
}

/*
* 将*mut c_void转换为Arc
*/
#[inline]
pub fn void2arc<T>(ptr_void: *mut c_void) -> Arc<T> {
    unsafe { Arc::from_raw(ptr_void as *mut T) }
}

/*
* 将*const c_void转换为usize
*/
#[inline]
pub fn void2usize(ptr_void: *const c_void) -> usize {
    ptr_void as usize
}

/*
* 将usize转换为*const c_void
*/
#[inline]
pub fn usize2void(ptr: usize) -> *const c_void {
    ptr as *const c_void
}