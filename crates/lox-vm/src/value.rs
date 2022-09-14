use crate::chunk::Chunk;
use std::fmt::{self, Display, Formatter};
use std::hint;
use std::ops::Not;

#[derive(Clone, Copy, Debug, Default)]
pub enum Value {
    #[default]
    Nil,
    Boolean(bool),
    Number(f64),
    Object(*mut Object),
}

impl Value {
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
            Self::Number(_) => "number",
            Self::Object(object) => match (unsafe { &**object }).type_ {
                ObjectType::Function(_) => "function",
                ObjectType::String(_) => "string",
            },
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Boolean(boolean) => write!(f, "{boolean}"),
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

pub struct Object {
    pub is_marked: bool,
    pub type_: ObjectType,
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.type_ {
            ObjectType::Function(function) => match function.name.to_str() {
                "" => write!(f, "<script>"),
                name => write!(f, "<function {name}>"),
            },
            ObjectType::String(string) => write!(f, "{string}"),
        }
    }
}

impl From<Function> for Object {
    fn from(function: Function) -> Self {
        Self { is_marked: false, type_: ObjectType::Function(function) }
    }
}

impl From<&'static str> for Object {
    fn from(string: &'static str) -> Self {
        Self { is_marked: false, type_: ObjectType::String(string) }
    }
}

pub enum ObjectType {
    Function(Function),
    String(&'static str),
}

#[derive(Debug)]
pub struct Function {
    pub name: *mut Object,
    pub arity: u8,
    pub chunk: Chunk,
}

pub trait ObjectExt {
    fn to_str(&self) -> &'static str;
}

impl ObjectExt for *mut Object {
    /// This must only be called when the underlying Object is a String. On any
    /// other Object type, this is undefined behavior.
    fn to_str(&self) -> &'static str {
        match unsafe { &(**self).type_ } {
            ObjectType::String(string) => string,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn sizes() {
        assert_eq!(mem::size_of::<Value>(), 16);
        assert_eq!(mem::size_of::<*mut Object>(), 8);
    }
}
