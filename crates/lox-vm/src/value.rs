use std::fmt::{self, Display, Formatter};
use std::ops::{Deref, Not};

#[derive(Clone, Copy, Default)]
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
            (Self::Object(a), Self::Object(b)) => a == b || unsafe { &**a == &**b },
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
            ObjectType::String(string) => write!(f, "{string}"),
        }
    }
}

impl From<&'static str> for Object {
    fn from(string: &'static str) -> Self {
        Self { is_marked: false, type_: ObjectType::String(string) }
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match (&self.type_, &other.type_) {
            (ObjectType::String(a), ObjectType::String(b)) => a.as_ptr() == b.as_ptr(),
        }
    }
}

pub enum ObjectType {
    String(&'static str),
}

#[cfg(test)]
mod tests {
    use crate::value::Value;

    use std::mem;

    #[test]
    fn size_of_value() {
        assert_eq!(mem::size_of::<Value>(), 16);
    }
}
