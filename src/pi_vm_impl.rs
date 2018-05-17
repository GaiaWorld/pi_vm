use std::boxed::FnBox;
use std::sync::{Arc, Mutex, Condvar};

use task::TaskType;
use task_pool::TaskPool;
use adapter::{JSStatus, JS, try_js_destroy, dukc_vm_status_check, dukc_vm_status_switch, dukc_vm_status_sub, dukc_wakeup, dukc_continue, js_reply_callback};

lazy_static! {
	pub static ref JS_TASK_POOL: Arc<(Mutex<TaskPool>, Condvar)> = Arc::new((Mutex::new(TaskPool::new(10)), Condvar::new()));
}

/*
* 线程安全的向任务池投递任务
*/
pub fn cast_task(task_type: TaskType, priority: u32, func: Box<FnBox()>, info: &'static str) {
        let &(ref lock, ref cvar) = &**JS_TASK_POOL;
        let mut task_pool = lock.lock().unwrap();
        (*task_pool).push(task_type, priority, func, info);
        cvar.notify_one();
}

/*
* 线程安全的回应阻塞调用
*/
pub fn block_reply(js: Arc<JS>, result: Box<FnBox(Arc<JS>)>, task_type: TaskType, priority: u32, info: &'static str) {
    let copy_js = js.clone();
    let func = Box::new(move || {
        unsafe {
            if dukc_vm_status_check(copy_js.get_vm(), JSStatus::WaitBlock as i8) > 0 || 
                dukc_vm_status_check(copy_js.get_vm(), JSStatus::SingleTask as i8) > 0 {
                //同步任务还未阻塞虚拟机，重新投递当前异步任务，并等待同步任务阻塞虚拟机
                block_reply(copy_js, result, task_type, priority, info);
            } else {
                let status = dukc_vm_status_switch(copy_js.get_vm(), JSStatus::MultiTask as i8, JSStatus::SingleTask as i8);
                if status == JSStatus::MultiTask as i8 {
                    //同步任务已阻塞虚拟机，则返回指定的值，并唤醒虚拟机继续同步执行
                    dukc_wakeup(copy_js.get_vm());
                    result(copy_js.clone());
                    dukc_continue(copy_js.get_vm(), js_reply_callback);
                    //当前异步任务如果没有投递其它异步任务，则当前异步任务成为同步任务，并在当前异步任务完成后回收虚拟机
                    //否则还有其它异步任务，则回收权利交由其它异步任务
                    dukc_vm_status_sub(copy_js.get_vm(), 1);
                } else {
                    try_js_destroy(&copy_js);
                    panic!("cast block reply task failed");
                }
            }
        }
    });
    
    let &(ref lock, ref cvar) = &**JS_TASK_POOL;
    let mut task_pool = lock.lock().unwrap();
    (*task_pool).push(task_type, priority, func, info);
    cvar.notify_one();
}