use std::fmt::{self, Display, Formatter};
use std::hint;
use std::ops::Not;

use crate::chunk::Chunk;

#[derive(Clone, Copy, Debug, Default)]
pub enum Value {
    #[default]
    Nil,
    Boolean(bool),
    Native(Native),
    Number(f64),
    Object(*mut Object),
}

impl Value {
    pub fn as_object(&self) -> *mut Object {
        match self {
            Value::Object(object) => *object,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            Self::Nil | Self::Boolean(false) => false,
            _ => true,
        }
    }

    pub fn type_(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Boolean(_) => "bool",
            Self::Native(_) => "native",
            Self::Number(_) => "number",
            Self::Object(object) => match &(unsafe { &**object }).type_ {
                ObjectType::Closure(_) | ObjectType::Function(_) => "function",
                ObjectType::String(_) => "string",
                ObjectType::Upvalue(upvalue) => unsafe { *upvalue.location }.type_(),
            },
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Boolean(boolean) => write!(f, "{boolean}"),
            Self::Native(native) => {
                let name = match native {
                    Native::Clock => "clock",
                };
                write!(f, "<native {}>", name)
            }
            Self::Number(number) => write!(f, "{number}"),
            Self::Object(object) => write!(f, "{}", unsafe { &**object }),
        }
    }
}

impl From<bool> for Value {
    fn from(boolean: bool) -> Self {
        Self::Boolean(boolean)
    }
}

impl From<f64> for Value {
    fn from(number: f64) -> Self {
        Self::Number(number)
    }
}

impl From<Native> for Value {
    fn from(native: Native) -> Self {
        Self::Native(native)
    }
}

impl From<*mut Object> for Value {
    fn from(object: *mut Object) -> Self {
        Self::Object(object)
    }
}

impl Not for Value {
    type Output = Self;

    fn not(self) -> Self::Output {
        (!self.bool()).into()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Native {
    Clock,
}

pub struct Object {
    pub is_marked: bool,
    pub type_: ObjectType,
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.type_ {
            ObjectType::Closure(closure) => write!(f, "{}", unsafe { &*closure.function }),
            ObjectType::Function(function) => match function.name.as_str() {
                "" => write!(f, "<script>"),
                name => write!(f, "<function {name}>"),
            },
            ObjectType::String(string) => write!(f, "{string}"),
            ObjectType::Upvalue(_) => write!(f, "<upvalue>"),
        }
    }
}

macro_rules! derive_from_object {
    ($object:tt, $type_:ty) => {
        impl From<$type_> for Object {
            fn from(object: $type_) -> Self {
                Self { is_marked: false, type_: ObjectType::$object(object) }
            }
        }
    };
}

derive_from_object!(Closure, Closure);
derive_from_object!(Function, Function);
derive_from_object!(String, &'static str);
derive_from_object!(Upvalue, Upvalue);

pub enum ObjectType {
    Closure(Closure),
    Function(Function),
    String(&'static str),
    Upvalue(Upvalue),
}

#[derive(Debug)]
pub struct Closure {
    pub function: *mut Object,
    pub upvalues: Vec<*mut Object>,
}

#[derive(Debug)]
pub struct Function {
    pub name: *mut Object,
    pub arity: u8,
    pub upvalues: u8,
    pub chunk: Chunk,
}

#[derive(Debug)]
pub struct Upvalue {
    pub location: *mut Value,
    pub closed: Value,
}

/// Unsafe extension functions. Use only when you are certain what the
/// underlying type is.
pub trait ObjectExt {
    fn as_function(self) -> &'static Function;
    fn as_str(self) -> &'static str;
    fn as_upvalue(self) -> &'static Upvalue;

    fn type_(self) -> &'static ObjectType;
}

impl ObjectExt for *mut Object {
    fn as_function(self) -> &'static Function {
        match self.type_() {
            ObjectType::Function(function) => function,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    fn as_str(self) -> &'static str {
        match self.type_() {
            ObjectType::String(string) => string,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    fn as_upvalue(self) -> &'static Upvalue {
        match self.type_() {
            ObjectType::Upvalue(upvalue) => upvalue,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    fn type_(self) -> &'static ObjectType {
        unsafe { &(*self).type_ }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(mem::size_of::<Value>(), 16);
        assert_eq!(mem::size_of::<*mut Object>(), 8);
    }
}
