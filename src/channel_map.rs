use std::sync::Arc;
use std::clone::Clone;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::cell::RefCell;

use pi_lib::atom::Atom;
use pi_lib::handler::{Env, GenType, Handler, Args};
use pi_lib::gray::GrayVersion;
use pi_base::task::TaskType;

use adapter::{JS, JSType};
use pi_vm_impl::{block_reply, push_callback};

/*
* 通道对端
*/
pub enum VMChannelPeer {
    Any,            //任意虚拟机
    VM(Arc<JS>),    //指定虚拟机
}

/*
* 虚拟机通道
*/
pub struct VMChannel {
    src: VMChannelPeer,             //源
    dst: VMChannelPeer,             //目标
    attrs: RefCell<HashMap<Atom, GenType>>,  //属性表
    gray: Option<usize>,
}

impl GrayVersion for VMChannel {
    fn get_gray(&self) -> &Option<usize> {
        &self.gray
    }

    fn set_gray(&mut self, gray: Option<usize>) {
        self.gray = gray
    }
}

impl Env for VMChannel {
    fn get_attr(&self, key: Atom) -> Option<GenType> {
        match self.attrs.borrow().get(&key) {
            None => None,
            Some(value) => Some(value.clone()),
        }
    }

    fn set_attr(&self, key: Atom, value: GenType) -> Option<GenType> {
        match self.attrs.borrow_mut().entry(key) {
            Entry::Occupied(ref mut e) => {
                Some(e.insert(value))
            },
            Entry::Vacant(e) => {
                e.insert(value);
                None
            },
        }
    }

    fn remove_attr(&self, key: Atom) -> Option<GenType> {
        self.attrs.borrow_mut().remove(&key)
    }
}

impl VMChannel {
    //构建一个虚拟机通道
    pub fn new(src: VMChannelPeer, dst: VMChannelPeer) -> Self {
        VMChannel {
            src: src,
            dst: dst,
            gray: None,
            attrs: RefCell::new(HashMap::new()),
        }
    }

    //发送消息
    pub fn send(&self, _name: Atom, _msg: Arc<Vec<u8>>) {
        //TODO
        &self.dst;
    }

    //回应请求
    pub fn response(&self, callback: Option<u32>, result: Arc<Vec<u8>>, native_objs: Vec<usize>) -> bool {
        match self.src {
            VMChannelPeer::VM(ref js) => {
                match callback {
                    None => {
                        //同步阻塞返回
                        let result = Box::new(move |vm: Arc<JS>| {
                            let array = vm.new_array();
                            let mut buffer = vm.new_uint8_array(result.len() as u32);
                            buffer.from_bytes(result.as_slice());
                            vm.set_index(&array, 0, &mut buffer);
                            let mut value: JSType;
                            let mut sub_array = vm.new_array();
                            for i in 0..native_objs.len() {
                                value = vm.new_native_object(native_objs[i]);
                                vm.set_index(&sub_array, i as u32, &mut value);
                            }
                            vm.set_index(&array, 1, &mut sub_array);
                        });
                        block_reply(js.clone(), result, TaskType::Sync, 1000000000, Atom::from("vm async block call response task"));
                    },
                    Some(index) => {
                        //异步回调
                        let args = Box::new(move |vm: Arc<JS>| -> usize {
                            let buffer = vm.new_uint8_array(result.len() as u32);
                            buffer.from_bytes(result.as_slice());
                            let mut value: JSType;
                            let array = vm.new_array();
                            for i in 0..native_objs.len() {
                                value = vm.new_native_object(native_objs[i]);
                                vm.set_index(&array, i as u32, &mut value);
                            }
                            2
                        });
                        push_callback(js.clone(), index, args, Atom::from("vm async call response task"));
                    }
                }
                true
            },
            _ => false
        }
    }
}

/*
* 虚拟机通道表
*/
pub struct VMChannelMap {
    gray: usize,                                                                                                                                        //灰度值
    map: HashMap<Atom, Arc<Handler<A = Arc<Vec<u8>>, B = Vec<JSType>, C = Option<u32>, D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>>,    //通道表
}

impl VMChannelMap {
    //构建一个虚拟机通道表
    pub fn new(gray: usize) -> Self {
        VMChannelMap {
            gray: gray,
            map: HashMap::new(),
        }
    }

    //获取灰度值
    pub fn get_gray(&self) -> usize {
        self.gray
    }

    //设置灰度值
    pub fn set_gray(&mut self, gray: usize) -> usize {
        let old = self.gray;
        self.gray = gray;
        old
    }

    //获取处理器数量
    pub fn size(&self) -> usize {
        self.map.len()
    }

    //设置指定名称的处理器，返回同名的上一个处理器
    pub fn set(&mut self, name: Atom, handler: Arc<Handler<A = Arc<Vec<u8>>, B = Vec<JSType>, C = Option<u32>, D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>) -> Option<Arc<Handler<A = Arc<Vec<u8>>, B = Vec<JSType>, C = Option<u32>, D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>> {
        match self.map.entry(name) {
            Entry::Occupied(ref mut e) => {
                Some(e.insert(handler))
            },
            Entry::Vacant(e) => {
                e.insert(handler);
                None
            },
        }
    }

    //移除指定名称的处理器，返回处理器
    pub fn remove(&mut self, name: Atom) -> Option<Arc<Handler<A = Arc<Vec<u8>>, B = Vec<JSType>, C = Option<u32>, D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>> {
        self.map.remove(&name)
    }

    //请求
    pub fn request(&self, js: Arc<JS>, name: Atom, msg: Arc<Vec<u8>>, native_objs: Vec<usize>, callback: Option<u32>) -> bool {
        let handler = match self.map.get(&name) {
            None => {
                return false;
            },
            Some(h) => {
                h
            },
        };

        let mut objs = Vec::new();
        for index in 0..native_objs.len() {
            objs.push(js.new_native_object(native_objs[index]));
        }

        let channel = VMChannel::new(VMChannelPeer::VM(js), VMChannelPeer::Any);
        handler.handle(Arc::new(channel), name, Args::ThreeArgs(msg, objs, callback));
        true
    }
}