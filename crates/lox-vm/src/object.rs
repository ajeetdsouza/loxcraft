use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{BuildHasherDefault, Hash, Hasher};

use arrayvec::ArrayVec;
use hashbrown::HashMap;
use rustc_hash::FxHasher;

use crate::chunk::Chunk;
use crate::value::Value;

#[derive(Clone, Copy)]
#[repr(C)]
pub union Object {
    pub common: *mut ObjectCommon,

    pub bound_method: *mut ObjectBoundMethod,
    pub class: *mut ObjectClass,
    pub closure: *mut ObjectClosure,
    pub function: *mut ObjectFunction,
    pub instance: *mut ObjectInstance,
    pub string: *mut ObjectString,
    pub upvalue: *mut ObjectUpvalue,
}

impl Object {
    pub fn type_(&self) -> &'static str {
        match unsafe { (*self.common).type_ } {
            ObjectType::BoundMethod => "bound method",
            ObjectType::Class => "class",
            ObjectType::Closure => "function",
            ObjectType::Function => "raw function",
            ObjectType::Instance => unsafe { (*(*(*self.instance).class).name).value },
            ObjectType::String => "string",
            ObjectType::Upvalue => unsafe { *(*self.upvalue).location }.type_(),
        }
    }

    pub fn free(self) {
        match unsafe { (*self.common).type_ } {
            ObjectType::BoundMethod => {
                unsafe { Box::from_raw(self.bound_method) };
            }
            ObjectType::Class => {
                unsafe { Box::from_raw(self.class) };
            }
            ObjectType::Closure => {
                unsafe { Box::from_raw(self.closure) };
            }
            ObjectType::Function => {
                unsafe { Box::from_raw(self.function) };
            }
            ObjectType::Instance => {
                unsafe { Box::from_raw(self.instance) };
            }
            ObjectType::String => {
                unsafe { Box::from_raw(self.string) };
            }
            ObjectType::Upvalue => {
                unsafe { Box::from_raw(self.upvalue) };
            }
        }
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match unsafe { (*self.common).type_ } {
            ObjectType::BoundMethod => {
                write!(f, "<bound method {}>", unsafe {
                    (*(*(*(*self.bound_method).closure).function).name).value
                })
            }
            ObjectType::Class => {
                write!(f, "<class {}>", unsafe { (*(*self.class).name).value })
            }
            ObjectType::Closure => {
                write!(f, "{}", unsafe { Object::from((*self.closure).function) })
            }
            ObjectType::Function => {
                let name = unsafe { (*(*self.function).name).value };
                if name.is_empty() {
                    write!(f, "<script>")
                } else {
                    write!(f, "<function {}>", name)
                }
            }
            ObjectType::Instance => {
                write!(f, "<object {}>", unsafe { (*(*(*self.instance).class).name).value })
            }
            ObjectType::String => write!(f, "{}", unsafe { (*self.string).value }),
            ObjectType::Upvalue => write!(f, "<upvalue>"),
        }
    }
}

macro_rules! impl_from_object {
    ($name:tt, $type_:ty) => {
        impl From<*mut $type_> for Object {
            fn from($name: *mut $type_) -> Self {
                Self { $name }
            }
        }
    };
}

impl_from_object!(bound_method, ObjectBoundMethod);
impl_from_object!(class, ObjectClass);
impl_from_object!(closure, ObjectClosure);
impl_from_object!(function, ObjectFunction);
impl_from_object!(instance, ObjectInstance);
impl_from_object!(string, ObjectString);
impl_from_object!(upvalue, ObjectUpvalue);

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.common }.hash(state)
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectCommon {
    pub type_: ObjectType,
    pub is_marked: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ObjectType {
    BoundMethod,
    Class,
    Closure,
    Function,
    Instance,
    String,
    Upvalue,
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectBoundMethod {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub this: *mut ObjectInstance,
    pub closure: *mut ObjectClosure,
}

impl ObjectBoundMethod {
    pub fn new(this: *mut ObjectInstance, method: *mut ObjectClosure) -> Self {
        Self { type_: ObjectType::BoundMethod, is_marked: false, this, closure: method }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectClass {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub name: *mut ObjectString,
    pub methods: HashMap<*mut ObjectString, *mut ObjectClosure, BuildHasherDefault<FxHasher>>,
}

impl ObjectClass {
    pub fn new(name: *mut ObjectString) -> Self {
        Self { type_: ObjectType::Class, is_marked: false, name, methods: HashMap::default() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectClosure {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub function: *mut ObjectFunction,
    pub upvalues: ArrayVec<*mut ObjectUpvalue, 256>,
}

impl ObjectClosure {
    pub fn new(function: *mut ObjectFunction, upvalues: ArrayVec<*mut ObjectUpvalue, 256>) -> Self {
        Self { type_: ObjectType::Closure, is_marked: false, function, upvalues }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectFunction {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub name: *mut ObjectString,
    pub arity: u8,
    pub upvalue_count: u16,
    pub chunk: Chunk,
}

impl ObjectFunction {
    pub fn new(name: *mut ObjectString, arity: u8) -> Self {
        Self {
            type_: ObjectType::Function,
            is_marked: false,
            name,
            arity,
            upvalue_count: 0,
            chunk: Chunk::default(),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectInstance {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub class: *mut ObjectClass,
    pub fields: HashMap<*mut ObjectString, Value, BuildHasherDefault<FxHasher>>,
}

impl ObjectInstance {
    pub fn new(class: *mut ObjectClass) -> Self {
        Self { type_: ObjectType::Instance, is_marked: false, class, fields: HashMap::default() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectString {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub value: &'static str,
}

impl ObjectString {
    pub fn new(value: &'static str) -> Self {
        Self { type_: ObjectType::String, is_marked: false, value }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectUpvalue {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub location: *mut Value,
    pub closed: Value,
}

impl ObjectUpvalue {
    pub fn new(location: *mut Value) -> Self {
        Self { type_: ObjectType::Upvalue, is_marked: false, location, closed: Value::default() }
    }
}
