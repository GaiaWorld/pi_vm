use std::collections::HashMap;
use std::cell::RefCell;
use adapter::{JSType, JS};
use std::sync::{Arc, Mutex};

lazy_static! {
	pub static ref BON_MGR: Arc<Mutex<BonMgr>> = Arc::new(Mutex::new(BonMgr::new()));
}

pub fn bon_call(js: Arc<JS>, fun_hash: u32, args: Option<Vec<JSType>>) -> Option<JSType>{
	(&mut *BON_MGR.lock().unwrap()).call(js, fun_hash, args)
}

pub trait StructMember {}
 
pub struct TypeDesc(bool, bool, NType);//(是否为引用, 是否可变, NType)
impl StructMember for TypeDesc{}

pub struct Property(String, TypeDesc);// Vec<(属性名, TypeDesc)>
impl StructMember for Property{}

pub struct StructMeta {
	pub name: String,
	//pub tp:String,//struct, tuple, empty
	//pub members:Vec<Box<StructMember>>
}

// pub struct EnumMeta {
// 	pub name: String,
// 	pub members: Vec<StructMeta>
// }

pub enum FnMeta {
	CallArg(fn(&BonMgr, Arc<JS>, Vec<JSType>) -> Option<JSType>),
	Call(fn(&BonMgr, Arc<JS>) -> Option<JSType>)
}
// pub struct FnMeta{
// 	pub call: fn(Vec<JSType>) -> Result<JSType, &'static str>,
// 	pub param: Vec<TypeDesc>, //（是否为引用，是否可变, 参数类型）如（"&str", String）
// 	pub result: TypeDesc,
// }

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
	fun_metas:HashMap<u32, FnMeta>,
	struct_metas:HashMap<u32, StructMeta>,
	//enum_metas:HashMap<u32, EnumMeta>,
	objs:RefCell<HashMap<usize, NObject>>
}

impl BonMgr{
	pub fn new () -> BonMgr{
		BonMgr{
			fun_metas: HashMap::new(),
			struct_metas: HashMap::new(),
			//enum_metas: HashMap::new(),
			objs: RefCell::new(HashMap::new())
		}
	}

	//有参数的调用
	pub fn call(&mut self, js: Arc<JS>, fun_hash: u32, args: Option<Vec<JSType>>) -> Option<JSType> {
		let func = match self.fun_metas.get(&fun_hash){
			Some(v) => v,
			None => {
				panic!("FnMeta is not finded");
			}
		};

		match func{
			&FnMeta::CallArg(ref f) => {
				f(self, js, args.unwrap())
			},
			&FnMeta::Call(ref f) => {
				f(self, js)
			}
		}
	}

	pub fn regist_fun_meta(&mut self, meta: FnMeta, hash: u32){
		self.fun_metas.insert(hash, meta);
	}

	pub fn regist_struct_meta(&mut self, meta: StructMeta, hash: u32){
		self.struct_metas.insert(hash, meta);
	}

	// pub fn regist_enum_meta(&mut self, meta: EnumMeta, hash: u32){
	// 	self.enum_metas.insert(hash, meta);
	// }

	pub fn regist_obj(&self, obj:NObject, ptr: usize){
		self.objs.borrow_mut().insert(ptr, obj);
	}

	// pub fn get_func_meta(&self, hash: u32) -> Result<&FnMeta, &'static str>{
	// 	let func = self.fun_metas.get(&hash);
	// 	match func{
	// 		Some(v) => Ok(v),
	// 		None => {Err("FnMeta is not finded")}
	// 	}
	// }

	// pub fn get_struct_meta(&self, hash: u32) -> Result<&StructMeta, &'static str>{
	// 	let func = self.struct_metas.get(&hash);
	// 	match func{
	// 		Some(v) => Ok(v),
	// 		None => {Err("StructMeta is not finded")}
	// 	}
	// }

	// fn get_enum_meta(&self, hash: u32) -> Result<&EnumMeta, &'static str>{
	// 	let func = self.enum_metas.get(&hash);
	// 	match func{
	// 		Some(v) => Ok(v),
	// 		None => {Err("EnumMeta is not finded")}
	// 	}
	// }
	// pub fn get_obj(&self, ptr: usize) -> Result<*const NObject, &'static str>{
	// 	let func = self.objs.borrow().get(&ptr);
	// 	//let func = self.objs.get(&ptr);
	// 	match func{
	// 		Some(v) => Ok(Box::into_raw(v) as *const NObject),
	// 		None => {Err("NObject is not finded")}
	// 	}
		
	// }
}

//特为构建代码提供，主要用于函数参数native_object转换为ptr， 如果类型不匹配将返回一个错误
pub fn jstype_ptr<'a>(jstype: &JSType, mgr: &BonMgr, obj_type: u32 ,error_str: &'a str) -> Result<usize, &'a str>{
	if !jstype.is_native_object(){
		return Err(error_str);
	}
	let ptr = jstype.get_native_object();
	let obj = mgr.objs.borrow();
	let obj = match obj.get(&ptr){
		Some(v) => v,
		None => {return Ok(ptr)//return Err("NObject is not finded");  
		}
	};
	if(obj.meta_hash == obj_type){
		Ok(ptr)
	}else{
		Err("type is diff")
	}
	
	// let meta = match mgr.struct_metas.get(&obj.meta_hash){
	// 	Some(v) => v,
	// 	None => {return Err("StructMeta is not finded");}
	// };

	// if meta.name == obj_type {
	// 	Ok(ptr)
	// }else{
	// 	Err("type is diff")
	// }
}

//特为构建代码提供，主要用于函数返回时ptr转换为native_object， 同时将根据返回类型构建NObject并注册
pub fn ptr_jstype(mgr: &BonMgr,js: Arc<JS>, ptr: usize, meta_hash: u32) -> JSType{
	let nobj = NObject{meta_hash: meta_hash};
	mgr.regist_obj(nobj, ptr);
	js.new_native_object(ptr)
}


// #[cfg(test)]
// mod tests {
// 	pub struct XX{
// 		name: i32,
// 		hash: u32,
// 	}

// 	impl XX{
// 		pub fn new(name: i32, hash: u32) -> XX{
// 			XX{
// 				name:name,
// 				hash:hash
// 			}
// 		}
// 	}

// 	pub fn xx_new_call (js: JS, mgr: BonMgr, v:Vec<JSType>) -> Result<JSType>{
// 		if v.len() != 2{
// 			return Err("param count error on xx_new");
// 		}

// 		let jst0 = v[0];
// 		if !jst0.is_number(){
// 		 	return Err("Some error message");
// 		}
// 		let jst0 = jst0.get_i32();

// 		let jst1 = v[1];
// 		if !jst1.is_number(){
// 		 	return Err("Some error message");
// 		}
// 		let jst1 = jst1.get_i32();

// 		let ptr = Box::into_raw(Box::new(XX::new(jst0, jst1))) as *const c_void as usize;
// 		let nobj = NObject{meta_hash: 11111111, ptr};
// 		mgr.regist_obj(ptr, nobj);
// 		Ok(js::new_native_object(ptr))
// 	}

//     #[test]
// 	fn test_obj() {
// 		let mgr = BonMgr::new();

// 		//注册XX结构体
// 		let members: Vec<Box<StructMember>> = Vec::new();
// 		members.push(Box::new(TypeDesc(false, false, "i32")));
// 		members.push(Box::new(TypeDesc(false, false, "u32")));
// 		mgr.regist_struct_meta(StructMeta{name: "bon_mgr::tests::XX", members: members}, 11111111);

// 		//注册XX的new方法
// 		let params = Vec::new();
// 		params.push(TypeDesc(false, false, "i32"));
// 		params.push(TypeDesc(false, false, "u32"));
// 		mgr.regist_fun_metas(xx_new_call, 2222222);


// 		mgr.call(2222222, Vec::new());
// 	}
// }

