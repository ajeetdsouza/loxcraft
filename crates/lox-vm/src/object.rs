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

impl Object {
    pub fn mark(&self) {
        if unsafe { (*(*self).common).is_marked } {
            return;
        }
        unsafe { (*(*self).common).is_marked = true };
    }

    pub fn free(&self) {
        match unsafe { (*self.common).type_ } {
            ObjectType::Class => {
                unsafe { Box::from_raw(self.class) };
            }
            ObjectType::Closure => {
                unsafe { Box::from_raw(self.closure) };
            }
            ObjectType::Function => {
                unsafe { Box::from_raw(self.function) };
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
        writeln!(f, "{}", self)
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            match (*self.common).type_ {
                ObjectType::Class => {
                    write!(f, "<class {}>", (*(*self.class).name).value)
                }
                ObjectType::Closure => {
                    write!(
                        f,
                        "<function {}>",
                        (*(*(*self.closure).function).name).value
                    )
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

macro_rules! impl_from_object {
    ($name:tt, $type_:ty) => {
        impl From<*mut $type_> for Object {
            fn from($name: *mut $type_) -> Self {
                Self { $name }
            }
        }
    };
}

impl_from_object!(class, ObjectClass);
impl_from_object!(closure, ObjectClosure);
impl_from_object!(function, ObjectFunction);
impl_from_object!(string, ObjectString);
impl_from_object!(upvalue, ObjectUpvalue);

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
#[repr(u8)]
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

impl ObjectClass {
    pub fn new(name: *mut ObjectString) -> Self {
        Self { type_: ObjectType::Closure, is_marked: false, name }
    }
}

#[repr(C)]
pub struct ObjectClosure {
    pub type_: ObjectType,
    pub is_marked: bool,
    pub function: *mut ObjectFunction,
    pub upvalues: Vec<*mut ObjectUpvalue>,
}

impl ObjectClosure {
    pub fn new(
        function: *mut ObjectFunction,
        upvalues: Vec<*mut ObjectUpvalue>,
    ) -> Self {
        Self {
            type_: ObjectType::Closure,
            is_marked: false,
            function,
            upvalues,
        }
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
    pub fn new(name: *mut ObjectString, arity: u8) -> Self {
        Self {
            type_: ObjectType::Function,
            is_marked: false,
            name,
            arity,
            upvalues: 0,
            chunk: Default::default(),
        }
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
        Self {
            type_: ObjectType::Upvalue,
            is_marked: false,
            location,
            closed: Default::default(),
        }
    }
}
