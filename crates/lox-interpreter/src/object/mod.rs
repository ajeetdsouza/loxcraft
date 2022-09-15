mod callable;
mod class;
mod function;
mod instance;
mod native;

use std::fmt::{self, Debug, Display, Formatter};

pub use callable::Callable;
pub use class::Class;
pub use function::Function;
use gc::{Finalize, Trace};
pub use instance::Instance;
use lox_common::error::{AttributeError, Error, Result, TypeError};
use lox_common::types::Span;
pub use native::Native;

use crate::env::Env;
use crate::Interpreter;

#[derive(Clone, Debug, Finalize, Trace)]
pub enum Object {
    Bool(bool),
    Class(Class),
    Function(Function),
    Instance(Instance),
    Native(Native),
    Nil,
    Number(f64),
    String(String),
}

impl Object {
    pub fn bool(&self) -> bool {
        !matches!(self, Object::Nil | Object::Bool(false))
    }

    pub fn type_(&self) -> String {
        match self {
            Object::Bool(_) => "bool".to_string(),
            Object::Class(_) => "class".to_string(),
            Object::Function(_) | Object::Native(_) => "function".to_string(),
            Object::Instance(instance) => instance.class().name().to_string(),
            Object::Nil => "nil".to_string(),
            Object::Number(_) => "number".to_string(),
            Object::String(_) => "string".to_string(),
        }
    }

    pub fn get(&self, name: &str, span: &Span) -> Result<Object> {
        let instance = match &self {
            Object::Instance(instance) => instance,
            _ => {
                return Err((
                    Error::AttributeError(AttributeError::NoSuchAttribute {
                        type_: self.type_(),
                        name: name.to_string(),
                    }),
                    span.clone(),
                ));
            }
        };

        if let Some(object) = instance.get(name) {
            return Ok(object);
        }

        instance.class().method(name, self.clone()).ok_or_else(|| {
            (
                Error::AttributeError(AttributeError::NoSuchAttribute { type_: self.type_(), name: name.to_string() }),
                span.clone(),
            )
        })
    }

    pub fn set(&mut self, name: &str, value: &Object, span: &Span) -> Result<()> {
        match &self {
            Object::Instance(instance) => {
                instance.set(name, value.clone());
                Ok(())
            }
            _ => Err((
                Error::AttributeError(AttributeError::NoSuchAttribute { type_: self.type_(), name: name.to_string() }),
                span.clone(),
            )),
        }
    }

    pub fn call(&self, interpreter: &mut Interpreter, env: &mut Env, args: Vec<Object>, span: &Span) -> Result<Object> {
        match &self {
            Object::Class(class) => class.call(interpreter, env, args, span),
            Object::Function(function) => function.call(interpreter, env, args, span),
            Object::Native(native) => native.call(interpreter, env, args, span),
            object => Err((Error::TypeError(TypeError::NotCallable { type_: object.type_() }), span.clone())),
        }
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Object::Bool(bool) => write!(f, "{}", bool),
            Object::Class(class) => write!(f, "{}", class),
            Object::Function(function) => write!(f, "{}", function),
            Object::Instance(instance) => write!(f, "{}", instance),
            Object::Native(native) => write!(f, "{}", native),
            Object::Nil => write!(f, "nil"),
            Object::Number(number) => write!(f, "{}", number),
            Object::String(string) => write!(f, "{}", string),
        }
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Object::Bool(b1), Object::Bool(b2)) => b1 == b2,
            (Object::Class(c1), Object::Class(c2)) => c1 == c2,
            (Object::Function(f1), Object::Function(f2)) => f1 == f2,
            (Object::Instance(i1), Object::Instance(i2)) => i1 == i2,
            (Object::Native(n1), Object::Native(n2)) => n1 == n2,
            (Object::Nil, Object::Nil) => true,
            (Object::Number(n1), Object::Number(n2)) => n1 == n2,
            (Object::String(s1), Object::String(s2)) => s1 == s2,
            _ => false,
        }
    }
}
