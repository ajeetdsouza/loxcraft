use std::fmt::{self, Display, Formatter};
use std::hint;
use std::ops::Not;

use crate::object::{Object, ObjectType};

#[derive(Clone, Copy, Debug, Default)]
pub enum Value {
    #[default]
    Nil,
    Boolean(bool),
    Native(Native),
    Number(f64),
    Object(Object),
}

impl Value {
    pub fn object(&self) -> Object {
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
            Self::Object(object) => match unsafe { (*object.common).type_ } {
                ObjectType::Class => "class",
                ObjectType::Closure | ObjectType::Function => "function",
                ObjectType::String => "string",
                ObjectType::Upvalue => unsafe { *(*object.upvalue).location }.type_(),
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
            Self::Object(object) => write!(f, "{object}"),
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

impl<T: Into<Object>> From<T> for Value {
    fn from(object: T) -> Self {
        Self::Object(object.into())
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

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(mem::size_of::<Value>(), 16);
    }
}
