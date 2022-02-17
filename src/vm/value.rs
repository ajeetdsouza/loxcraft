use crate::vm::chunk::Chunk;
use crate::vm::vm::RuntimeError;

use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    Object(Object),
}

impl Value {
    pub fn bool(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn type_(&self) -> &str {
        match self {
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::Number(_) => "number",
            Value::Object(object) => match object {
                Object::String(_) => "string",
                Object::Function(_) => "function",
                Object::Native(_) => "native",
            },
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(bool) => write!(f, "{}", bool),
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{}", number),
            Value::Object(object) => write!(f, "{}", object),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Object {
    Function(Rc<Function>),
    Native(Native),
    String(Rc<String>),
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::Function(function) => write!(f, "{}", function),
            Object::Native(native) => write!(f, "{}", native),
            Object::String(string) => write!(f, "{}", string),
        }
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // Functions are only equal if they point to the same object.
            (Object::Function(f1), Object::Function(f2)) => Rc::ptr_eq(f1, f2),
            (Object::Native(n1), Object::Native(n2)) => n1 == n2,
            (Object::String(s1), Object::String(s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl PartialOrd for Object {
    /// Always returns [`None`], since objects cannot be ordered.
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

impl Function {
    /// Creates a new function with an empty chunk.
    pub fn new<S: Into<String>>(name: S, arity: usize) -> Self {
        Function { name: name.into(), arity, chunk: Chunk::new() }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<function {}>", self.name)
        }
    }
}

impl PartialEq for Function {
    /// Always returns `false`, since functions cannot be compared. When
    /// actually comparing functions, we use pointer equality.
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl PartialOrd for Function {
    /// Always returns [`None`], since functions cannot be ordered.
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Native {
    Clock,
}

impl Native {
    /// Returns the function pointer for the given native.
    pub fn function(&self) -> fn(&[Value]) -> Result<Value, RuntimeError> {
        match self {
            Native::Clock => native::clock,
        }
    }
}

impl fmt::Display for Native {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native function>")
    }
}

impl PartialOrd for Native {
    /// Always returns [`None`], since native functions cannot be ordered.
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

mod native {
    use crate::vm::value::Value;
    use crate::vm::vm::RuntimeError;

    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn clock(_args: &[Value]) -> Result<Value, RuntimeError> {
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        Ok(Value::Number(elapsed as f64 / 1e9))
    }
}
