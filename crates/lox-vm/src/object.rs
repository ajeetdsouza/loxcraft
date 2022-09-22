use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use crate::chunk::Chunk;
use crate::value::Value;

#[derive(Clone, Copy)]
#[repr(C)]
pub union Object {
    pub common: *mut ObjectCommon,
    pub class: *mut ObjectClass,
    pub closure: *mut ObjectClosure,
    pub function: *mut ObjectFunction,
    pub string: *mut ObjectString,
    pub upvalue: *mut ObjectUpvalue,
}

impl Debug for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self)
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            match (*self.common).type_ {
                ObjectType::Class => write!(f, "<class {}>", (*(*self.class).name).value),
                ObjectType::Closure => {
                    write!(f, "<function {}>", (*(*(*self.closure).function).name).value)
                }
                ObjectType::Function => {
                    write!(f, "<function {}>", (*(*self.function).name).value)
                }
                ObjectType::String => write!(f, "{}", (*self.string).value),
                ObjectType::Upvalue => write!(f, "<upvalue>"),
            }
        }
    }
}

impl Eq for Object {}

impl From<*mut ObjectClosure> for Object {
    fn from(closure: *mut ObjectClosure) -> Self {
        Self { closure }
    }
}

impl From<*mut ObjectFunction> for Object {
    fn from(function: *mut ObjectFunction) -> Self {
        Self { function }
    }
}

impl From<*mut ObjectString> for Object {
    fn from(string: *mut ObjectString) -> Self {
        Self { string }
    }
}

impl From<*mut ObjectUpvalue> for Object {
    fn from(upvalue: *mut ObjectUpvalue) -> Self {
        Self { upvalue }
    }
}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.common }.hash(state)
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.common == other.common }
    }
}

#[repr(C)]
pub struct ObjectCommon {
    pub type_: ObjectType,
    pub is_marked: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum ObjectType {
    Class,
    Closure,
    Function,
    String,
    Upvalue,
}

#[repr(C)]
pub struct ObjectClass {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub name: *mut ObjectString,
}

#[repr(C)]
pub struct ObjectClosure {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub function: *mut ObjectFunction,
    pub upvalues: Vec<*mut ObjectUpvalue>,
}

impl ObjectClosure {
    pub fn new(function: *mut ObjectFunction, upvalues: Vec<*mut ObjectUpvalue>) -> *mut Self {
        let closure = ObjectClosure { type_: ObjectType::Closure, is_marked: false, function, upvalues };
        Box::into_raw(Box::new(closure))
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ObjectFunction {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub name: *mut ObjectString,
    pub arity: u8,
    pub upvalues: u8,
    pub chunk: Chunk,
}

impl ObjectFunction {
    pub fn new(name: *mut ObjectString, arity: u8) -> *mut Self {
        let function = ObjectFunction {
            type_: ObjectType::Function,
            is_marked: false,
            name,
            arity,
            upvalues: 0,
            chunk: Default::default(),
        };
        Box::into_raw(Box::new(function))
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
    pub fn new(value: &'static str) -> *mut Self {
        let string = ObjectString { type_: ObjectType::String, is_marked: false, value };
        Box::into_raw(Box::new(string))
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
    pub fn new(location: *mut Value) -> *mut Self {
        let upvalue =
            ObjectUpvalue { type_: ObjectType::Upvalue, is_marked: false, location, closed: Default::default() };
        Box::into_raw(Box::new(upvalue))
    }
}
