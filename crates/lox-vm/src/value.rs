use std::fmt::{self, Display, Formatter};
use std::hint;
use std::ops::Not;

use crate::object::Object;

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
            Self::Object(object) => *object,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    pub fn bool(&self) -> bool {
        !matches!(self, Self::Boolean(false) | Self::Nil)
    }

    pub fn type_(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "bool",
            Self::Native(_) => "native",
            Self::Nil => "nil",
            Self::Number(_) => "number",
            Self::Object(object) => object.type_(),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean(boolean) => write!(f, "{boolean}"),
            Self::Native(native) => {
                let name = match native {
                    Native::Clock => "clock",
                };
                write!(f, "<native {}>", name)
            }
            Self::Nil => write!(f, "nil"),
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
