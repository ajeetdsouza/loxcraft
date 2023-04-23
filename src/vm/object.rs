use std::fmt::{self, Debug, Display, Formatter};
use std::hash::BuildHasherDefault;
use std::mem;

use hashbrown::HashMap;
use rustc_hash::FxHasher;

use crate::vm::chunk::Chunk;
use crate::vm::value::Value;

const _: () = assert!(mem::size_of::<Object>() == 4 || mem::size_of::<Object>() == 8);

#[derive(Clone, Copy, Eq)]
#[repr(C)]
pub union Object {
    pub common: *mut ObjectCommon,
    pub bound_method: *mut ObjectBoundMethod,
    pub class: *mut ObjectClass,
    pub closure: *mut ObjectClosure,
    pub function: *mut ObjectFunction,
    pub instance: *mut ObjectInstance,
    pub native: *mut ObjectNative,
    pub string: *mut ObjectString,
    pub upvalue: *mut ObjectUpvalue,
}

impl Object {
    /// Returns the type of the [`Object`] as a string. Useful for error
    /// messages.
    pub fn type_(&self) -> ObjectType {
        unsafe { (*self.common).type_ }
    }

    /// Frees the value being pointed to by the [`Object`], based on its type.
    pub fn free(self) {
        match self.type_() {
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
            ObjectType::Native => {
                unsafe { Box::from_raw(self.native) };
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
        write!(f, "{self}")
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.type_() {
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
                if name.is_empty() { write!(f, "<script>") } else { write!(f, "<function {name}>") }
            }
            ObjectType::Instance => {
                write!(f, "<object {}>", unsafe { (*(*(*self.instance).class).name).value })
            }
            ObjectType::Native => write!(f, "<native {}>", unsafe { (*self.native).native }),
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

impl_from_object!(common, ObjectCommon);
impl_from_object!(bound_method, ObjectBoundMethod);
impl_from_object!(class, ObjectClass);
impl_from_object!(closure, ObjectClosure);
impl_from_object!(function, ObjectFunction);
impl_from_object!(instance, ObjectInstance);
impl_from_object!(native, ObjectNative);
impl_from_object!(string, ObjectString);
impl_from_object!(upvalue, ObjectUpvalue);

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.common == other.common }
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
    Native,
    Instance,
    String,
    Upvalue,
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::BoundMethod => write!(f, "bound method"),
            ObjectType::Class => write!(f, "class"),
            ObjectType::Closure => write!(f, "function"),
            ObjectType::Function => write!(f, "function"),
            ObjectType::Instance => write!(f, "instance"),
            ObjectType::Native => write!(f, "native"),
            ObjectType::String => write!(f, "string"),
            ObjectType::Upvalue => write!(f, "upvalue"),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectBoundMethod {
    pub common: ObjectCommon,
    pub this: *mut ObjectInstance,
    pub closure: *mut ObjectClosure,
}

impl ObjectBoundMethod {
    pub fn new(this: *mut ObjectInstance, method: *mut ObjectClosure) -> Self {
        let common = ObjectCommon { type_: ObjectType::BoundMethod, is_marked: false };
        Self { common, this, closure: method }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectClass {
    pub common: ObjectCommon,
    pub name: *mut ObjectString,
    pub methods: HashMap<*mut ObjectString, *mut ObjectClosure, BuildHasherDefault<FxHasher>>,
}

impl ObjectClass {
    pub fn new(name: *mut ObjectString) -> Self {
        let common = ObjectCommon { type_: ObjectType::Class, is_marked: false };
        Self { common, name, methods: HashMap::default() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectClosure {
    pub common: ObjectCommon,
    pub function: *mut ObjectFunction,
    pub upvalues: Vec<*mut ObjectUpvalue>,
}

impl ObjectClosure {
    pub fn new(function: *mut ObjectFunction, upvalues: Vec<*mut ObjectUpvalue>) -> Self {
        let common = ObjectCommon { type_: ObjectType::Closure, is_marked: false };
        Self { common, function, upvalues }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectFunction {
    pub common: ObjectCommon,
    pub name: *mut ObjectString,
    pub arity: u8,
    pub upvalue_count: u16,
    pub chunk: Chunk,
}

impl ObjectFunction {
    pub fn new(name: *mut ObjectString, arity: u8) -> Self {
        let common = ObjectCommon { type_: ObjectType::Function, is_marked: false };
        Self { common, name, arity, upvalue_count: 0, chunk: Chunk::default() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectInstance {
    pub common: ObjectCommon,
    pub class: *mut ObjectClass,
    pub fields: HashMap<*mut ObjectString, Value, BuildHasherDefault<FxHasher>>,
}

impl ObjectInstance {
    pub fn new(class: *mut ObjectClass) -> Self {
        let common = ObjectCommon { type_: ObjectType::Instance, is_marked: false };
        Self { common, class, fields: HashMap::default() }
    }
}

pub struct ObjectNative {
    pub common: ObjectCommon,
    pub native: Native,
}

impl ObjectNative {
    pub fn new(native: Native) -> Self {
        let common = ObjectCommon { type_: ObjectType::Native, is_marked: false };
        Self { common, native }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Native {
    Clock,
}

impl Display for Native {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Native::Clock => write!(f, "clock"),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectString {
    pub common: ObjectCommon,
    pub value: &'static str,
}

impl ObjectString {
    pub fn new(value: &'static str) -> Self {
        let common = ObjectCommon { type_: ObjectType::String, is_marked: false };
        Self { common, value }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectUpvalue {
    pub common: ObjectCommon,
    pub location: *mut Value,
    pub closed: Value,
}

impl ObjectUpvalue {
    pub fn new(location: *mut Value) -> Self {
        let common = ObjectCommon { type_: ObjectType::Upvalue, is_marked: false };
        Self { common, location, closed: Value::default() }
    }
}
