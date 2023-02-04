use std::fmt::{self, Debug, Display, Formatter};
use std::mem;
use std::ops::Not;

use crate::object::{Object, ObjectType};
use crate::util;

const _: () = assert!(mem::size_of::<Value>() == 8);

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Value(u64);

impl Default for Value {
    fn default() -> Self {
        Self::NIL
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{self}")
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            write!(f, "nil")
        } else if self.is_true() {
            write!(f, "true")
        } else if self.is_false() {
            write!(f, "false")
        } else if self.is_number() {
            write!(f, "{}", self.as_number())
        } else if self.is_object() {
            write!(f, "{}", self.as_object())
        } else {
            util::unreachable()
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self(value as u64 | Self::FALSE.0)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        unsafe { mem::transmute(value) }
    }
}

impl<O: Into<Object>> From<O> for Value {
    fn from(object: O) -> Self {
        Self((unsafe { object.into().common } as u64) | Self::SIGN_BIT | Self::QNAN)
    }
}

impl Not for Value {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::from(!self.to_bool())
    }
}

impl Value {
    const SIGN_BIT: u64 = 0x8000000000000000;
    const QNAN: u64 = 0x7ffc000000000000;

    pub const NIL: Self = Self(Self::QNAN | 0b01);
    pub const FALSE: Self = Self(Self::QNAN | 0b10);
    pub const TRUE: Self = Self(Self::QNAN | 0b11);

    pub fn type_(self) -> ValueType {
        if self.is_nil() {
            ValueType::Nil
        } else if self.is_bool() {
            ValueType::Bool
        } else if self.is_number() {
            ValueType::Number
        } else if self.is_object() {
            ValueType::Object(self.as_object().type_())
        } else {
            util::unreachable()
        }
    }

    pub fn is_nil(self) -> bool {
        self == Self::NIL
    }

    pub fn is_bool(self) -> bool {
        Self(self.0 | 0b01) == Self::TRUE
    }

    pub const fn is_number(self) -> bool {
        (self.0 & Self::QNAN) != Self::QNAN
    }

    pub const fn is_object(self) -> bool {
        self.0 & (Self::QNAN | Self::SIGN_BIT) == (Self::QNAN | Self::SIGN_BIT)
    }

    pub fn is_false(self) -> bool {
        Self(self.0) == Self::FALSE
    }

    pub fn is_true(self) -> bool {
        Self(self.0) == Self::TRUE
    }

    /// # Safety
    /// This is undefined behavior if the [`Value`] is not of type [`ValueType::Bool`].
    pub fn as_bool(self) -> bool {
        debug_assert!(self.is_bool());
        self == Self::TRUE
    }

    /// # Safety
    /// This is undefined behavior if the [`Value`] is not of type [`ValueType::Number`].
    pub const fn as_number(self) -> f64 {
        debug_assert!(self.is_number());
        unsafe { mem::transmute(self) }
    }

    /// # Safety
    /// This is undefined behavior if the [`Value`] is not of type [`ValueType::Object`].
    pub const fn as_object(self) -> Object {
        debug_assert!(self.is_object());
        Object { common: (self.0 & !(Self::SIGN_BIT | Self::QNAN)) as _ }
    }

    pub const fn to_bool(self) -> bool {
        !matches!(self, Self::FALSE | Self::NIL)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ValueType {
    Nil,
    Bool,
    Number,
    Object(ObjectType),
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Bool => write!(f, "bool"),
            Self::Number => write!(f, "number"),
            Self::Object(type_) => write!(f, "{type_}"),
        }
    }
}

impl From<ObjectType> for ValueType {
    fn from(type_: ObjectType) -> Self {
        Self::Object(type_)
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;
    use crate::object::ObjectCommon;

    #[test]
    fn convert_to_and_from_values() {
        let value = false;
        assert_eq!(Value::from(value).as_bool(), value);

        let value = true;
        assert_eq!(Value::from(value).as_bool(), value);

        let value = 0.0;
        assert_eq!(Value::from(value).as_number(), value);

        let value = f64::NAN;
        assert!(Value::from(value).as_number().is_nan());

        let value = Object { common: ptr::null_mut() };
        assert_eq!(Value::from(value).as_object(), value);
    }

    #[test]
    fn value_is_nil() {
        assert!(Value::NIL.is_nil());
        assert!(!Value::from(false).is_nil());
        assert!(!Value::from(true).is_nil());
        assert!(!Value::from(0.0).is_nil());
        assert!(!Value::from(f64::NAN).is_nil());
        assert!(!Value::from(Object { common: ptr::null_mut() }).is_nil());
    }

    #[test]
    fn value_is_bool() {
        assert!(!Value::NIL.is_bool());
        assert!(Value::from(false).is_bool());
        assert!(Value::from(true).is_bool());
        assert!(!Value::from(0.0).is_bool());
        assert!(!Value::from(f64::NAN).is_bool());
        assert!(!Value::from(Object { common: ptr::null_mut() }).is_bool());
    }

    #[test]
    fn value_is_number() {
        assert!(!Value::NIL.is_number());
        assert!(!Value::from(false).is_number());
        assert!(!Value::from(true).is_number());
        assert!(Value::from(0.0).is_number());
        assert!(Value::from(f64::NAN).is_number());
        assert!(!Value::from(Object { common: ptr::null_mut() }).is_number());
    }

    #[test]
    fn value_is_object() {
        assert!(!Value::NIL.is_object());
        assert!(!Value::from(false).is_object());
        assert!(!Value::from(true).is_object());
        assert!(!Value::from(0.0).is_object());
        assert!(!Value::from(f64::NAN).is_object());
        assert!(Value::from(Object { common: ptr::null_mut() }).is_object());
    }

    #[test]
    fn value_to_bool() {
        // Falsey
        assert!(!Value::NIL.to_bool());
        assert!(!Value::FALSE.to_bool());

        // Truthy
        assert!(Value::TRUE.to_bool());
        assert!(Value::from(0.0).to_bool());
        assert!(Value::from(f64::NAN).to_bool());
        assert!(Value::from(Object { common: ptr::null_mut() }).to_bool());
    }

    #[test]
    fn value_type() {
        assert_eq!(Value::NIL.type_(), ValueType::Nil);
        assert_eq!(Value::FALSE.type_(), ValueType::Bool);
        assert_eq!(Value::TRUE.type_(), ValueType::Bool);
        assert_eq!(Value::from(0.0).type_(), ValueType::Number);
        assert_eq!(Value::from(f64::NAN).type_(), ValueType::Number);
        assert_eq!(
            Value::from(
                &mut ObjectCommon { type_: ObjectType::String, is_marked: false } as *mut _
            )
            .type_(),
            ValueType::Object(ObjectType::String)
        );
    }
}
