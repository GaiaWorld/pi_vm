use std::any::Any;
use std::sync::Arc;
use std::boxed::FnBox;
use std::clone::Clone;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use pi_lib::atom::Atom;
use pi_lib::handler::{Env, GenType, Handler};

use adapter::JS;

/*
* 通道对端
*/
pub enum VMChannelPeer {
    Any,    //任意
    VM(JS), //虚拟机
}

/*
* 虚拟机通道
*/
pub struct VMChannel {
    src: VMChannelPeer,             //源
    dst: VMChannelPeer,             //目标
    attrs: HashMap<Atom, GenType>,  //属性表
}

impl Env for VMChannel {
    fn get_attr(&self, key: Atom) -> Option<GenType> {
        match self.attrs.get(&key) {
            None => None,
            Some(value) => Some(value.clone()),
        }
    }

    fn set_attr(&mut self, key: Atom, value: GenType) -> Option<GenType> {
        match self.attrs.entry(key) {
            Entry::Occupied(mut e) => {
                Some(e.insert(value))
            },
            Entry::Vacant(mut e) => {
                e.insert(value);
                None
            },
        }
    }

    fn remove_attr(&mut self, key: Atom) -> Option<GenType> {
        self.attrs.remove(&key)
    }
}

impl VMChannel {
    //构建一个虚拟机通道
    pub fn new(src: VMChannelPeer, dst: VMChannelPeer) -> Self {
        VMChannel {
            src: src,
            dst: dst,
            attrs: HashMap::new(),
        }
    }

    //发送消息
    pub fn send(&self, _name: Atom, _msg: Arc<Vec<u8>>) {

    }

    //回应请求
    pub fn response(&self, callback: u32, args: Box<FnBox(Arc<JS>) -> usize>) {

    }
}

/*
* 虚拟机通道表
*/
pub struct VMChannelMap {
    gray: usize,                        //灰度值
    map: HashMap<Atom, Arc<Handler<A = Arc<Vec<u8>>, B = (), C = (), D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>>,   //通道表
}

impl VMChannelMap {
    //构建一个虚拟机通道表
    pub fn new(gray: usize) -> Self {
        VMChannelMap {
            gray: gray,
            map: HashMap::new(),
        }
    }

    //设置指定名称的通道，返回同名的上一个通道
    pub fn set(&mut self, name: Atom, handler: Arc<Handler<A = Arc<Vec<u8>>, B = (), C = (), D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>) -> Option<Arc<Handler<A = Arc<Vec<u8>>, B = (), C = (), D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>> {
        match self.map.entry(name) {
            Entry::Occupied(mut e) => {
                Some(e.insert(handler))
            },
            Entry::Vacant(mut e) => {
                e.insert(handler);
                None
            },
        }
    }

    //移除指定名称的通道，返回通道
    pub fn remove(&mut self, name: Atom) -> Option<Arc<Handler<A = Arc<Vec<u8>>, B = (), C = (), D = (), E = (), F = (), G = (), H = (), HandleResult = ()>>> {
        self.map.remove(&name)
    }

    //请求
    pub fn request(&self, name: Atom, msg: Arc<Vec<u8>>, callback: u32) {

    }
}