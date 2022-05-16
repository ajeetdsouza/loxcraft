use crate::chunk::Chunk;
use crate::vm::RuntimeError;

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Closure(Closure),
    Function(Rc<Function>),
    Native(Native),
    Nil,
    Number(f64),
    String(Rc<String>),
}

impl Value {
    pub fn bool(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn type_(&self) -> &str {
        match self {
            Value::Bool(_) => "bool",
            Value::Closure(_) | Value::Function(_) => "function",
            Value::Native(_) => "native",
            Value::Nil => "nil",
            Value::Number(_) => "number",
            Value::String(_) => "string",
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(bool) => write!(f, "{bool}"),
            Value::Closure(closure) => write!(f, "{closure}"),
            Value::Function(function) => write!(f, "{function}"),
            Value::Native(native) => write!(f, "{native}"),
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{number}"),
            Value::String(string) => write!(f, "{string}"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(b1), Value::Bool(b2)) => b1 == b2,
            // Functions are only equal if they point to the same value.
            (Value::Function(f1), Value::Function(f2)) => Rc::ptr_eq(f1, f2),
            (Value::Native(n1), Value::Native(n2)) => n1 == n2,
            (Value::Nil, Value::Nil) => true,
            (Value::Number(n1), Value::Number(n2)) => n1 == n2,
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    /// Only numbers can be ordered.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::Number(n1), Value::Number(n2)) => n1.partial_cmp(n2),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub function: Rc<Function>,
}

impl Display for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.function)
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

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

impl Display for Native {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
    use crate::value::Value;
    use crate::vm::RuntimeError;

    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn clock(_args: &[Value]) -> Result<Value, RuntimeError> {
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        Ok(Value::Number(elapsed as f64 / 1e9))
    }
}
