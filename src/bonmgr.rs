use std::sync::{Arc, Mutex};
use std::cell::RefCell;

use std::collections::HashMap;
use adapter::{JSType, JS};
use atom::Atom;

lazy_static! {
	pub static ref BON_MGR: Arc<BonMgr> = Arc::new(BonMgr::new());
}

//权限表
#[derive(Clone)]
pub struct NativeObjsAuth(Option<Arc<HashMap<Atom, ()>>>, Option<Arc<HashMap<Atom, ()>>>);

impl NativeObjsAuth{
    pub fn new(white: Option<Arc<HashMap<Atom, ()>>>, black: Option<Arc<HashMap<Atom, ()>>>) -> NativeObjsAuth{
        NativeObjsAuth(white, black)
    }

    pub fn with_none() -> Arc<NativeObjsAuth>{
        Arc::new(NativeObjsAuth(None, None))
    }
}

#[derive(Clone)]
pub struct NativeObjs(pub Arc<RefCell<HashMap<usize, NObject>>>);

impl NativeObjs{
    pub fn new() -> Self{
        NativeObjs(Arc::new(RefCell::new(HashMap::new())))
    }
    pub fn insert(&self, ptr: usize, obj: NObject){
        self.0.borrow_mut().insert(ptr, obj);
    }
}

impl Drop for NativeObjs{
    fn drop(&mut self){
//        println!("drop nativeobj!");
        let map = self.0.borrow();
        let struct_metas = BON_MGR.struct_metas.lock().unwrap();
        for (ptr, nobj) in map.iter(){
            if let Some(meta) = struct_metas.get(&nobj.meta_hash) {
				(meta.drop_fn)(*ptr);
			}
        }
    }
}

pub fn bon_call(js: Arc<JS>, fun_hash: u32, args: Option<Vec<JSType>>) -> Option<CallResult>{
	BON_MGR.call(js, fun_hash, args)
}

pub enum CallResult{
    Ok,
    Err(String),
}

pub trait StructMember {}
 
pub struct TypeDesc(bool, bool, NType);//(是否为引用, 是否可变, NType)
impl StructMember for TypeDesc{}

pub struct Property(String, TypeDesc);// Vec<(属性名, TypeDesc)>
impl StructMember for Property{}

pub struct StructMeta {
	pub name: String,
    pub drop_fn: fn(ptr: usize),
	//pub tp:String,//struct, tuple, empty
	//pub members:Vec<Box<StructMember\0// pub struct EnumMeta {
// 	pub name: String,
// 	pub members: Vec<StructMeta>
}

#[derive(Clone)]
pub enum FnMeta {
	CallArg(fn(Arc<JS>, Vec<JSType>) -> Option<CallResult>),
    Call(fn(Arc<JS>) -> Option<CallResult>),
}

pub enum NType{
	I8,
	I16,
	I32,
	I64,
	U8,
	U16,
	U32,
	U64,
	F32,
	F64,
	Str,
	Bool,
	NativeObj(String)
}

impl NType {
	pub fn from_str(s: &str) -> NType{
		match s {
			"i8" => NType::I8,
			"i16" => NType::I16,
			"i32" => NType::I64,
			"i64" => NType::I8,
			"u8" => NType::U8,
			"u16" => NType::U16,
			"u32" => NType::U32,
			"u64" => NType::U64,
			"f32" => NType::F32,
			"f64" => NType::F64,
			"str" => NType::Str,
			"bool"=> NType::Bool,
			_ => NType::NativeObj(String::from(s)),
		}
	}
}

pub struct NObject {
	pub meta_hash: u32,
}

pub struct BonMgr{
	fun_metas: Arc<Mutex<HashMap<u32, FnMeta>>>,
	pub struct_metas:Arc<Mutex<HashMap<u32, StructMeta>>>,
}

impl BonMgr{
	pub fn new () -> BonMgr{
		BonMgr{
			fun_metas: Arc::new(Mutex::new(HashMap::new())),
			struct_metas: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	//有参数的调用
	pub fn call(&self, js: Arc<JS>, fun_hash: u32, args: Option<Vec<JSType>>) -> Option<CallResult> {
        let func = {
            let fun_ref = self.fun_metas.lock().unwrap();
            match fun_ref.get(&fun_hash){
                Some(v) => v.clone(),
                None => {
                    panic!("FnMeta is not finded, hash:{}", fun_hash);
                }
            }
        };

		match func{
            FnMeta::CallArg(f) => {
				f(js, args.unwrap())
			},
            FnMeta::Call(f) => {
				f(js)
			},
		}
	}

	pub fn regist_fun_meta(&self, meta: FnMeta, hash: u32){
        let mut fun_ref = self.fun_metas.lock().unwrap();
		fun_ref.insert(hash, meta);
	}

	pub fn regist_struct_meta(&self, meta: StructMeta, hash: u32){
		self.struct_metas.lock().unwrap().insert(hash, meta);
	}
}

//特为构建代码提供，主要用于函数参数native_object转换为ptr， 如果类型不匹配将返回一个错误
pub fn jstype_ptr<'a>(jstype: &JSType, js: Arc<JS>, obj_type: u32 , is_ownership:bool, error_str: &'a str) -> Result<usize, &'a str>{
	if !jstype.is_native_object(){
		return Err(error_str);
	}
	let ptr = jstype.get_native_object();
    let objs = js.get_objs();
    let mut objs = objs.borrow_mut();
	let r = {
        let objs_ref = js.get_objs_ref();
        let objs_ref = objs_ref.borrow();
        let obj = match objs.get(&ptr){//先从拥有所有权的obj列表中获取NObject
            Some(v) => v,
            None => {
                if is_ownership {//如果需要所有权， 直接抛出错误
                    println!("NObject is not found in objs, ptr:{}, type:{}", ptr, obj_type);
                    return Err("NObject is not found in objs");
                }else{//如果不需要所有权， 从引用类obj列表中获取NObject
                    match objs_ref.get(&ptr){
                        Some(v) => v,
                        None => {
                            println!("NObject is not found in objs_ref, ptr:{}, type: {}", ptr, obj_type);
                            return Err("NObject is not found in objs_ref");
                        }
                    }
                }
            }
        };
        if obj.meta_hash == obj_type{
            Ok(ptr)
        }else{
            println!("expect {}, found {}", obj_type, obj.meta_hash);
            Err("type is diff")
        }
    };

    if is_ownership{//如果参数要求所有权， 需要从所有权obj列表中删
        //println!("-----------------------------------------------------------------------------------------{}", &ptr);
        objs.remove(&ptr);
    }
    r
}

//特为构建代码提供，主要用于函数返回时ptr转换为native_object， 同时将根据返回类型构建NObject并注册
pub fn ptr_jstype(objs: Arc<RefCell<HashMap<usize, NObject>>>,js: Arc<JS>, ptr: usize, meta_hash: u32) -> JSType{
    let mut objs = objs.borrow_mut();
	let nobj = NObject{meta_hash: meta_hash};
    objs.insert(ptr, nobj);
	js.new_native_object(ptr)
}