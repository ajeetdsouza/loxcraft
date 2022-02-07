use crate::vm::chunk::Chunk;
use crate::vm::native;

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

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Object {
    Function(Function),
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

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

impl Function {
    pub fn new(name: &str, arity: usize) -> Self {
        Self { name: name.to_string(), arity, chunk: Chunk::new() }
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
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl PartialOrd for Function {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Native {
    pub id: u8,
}

impl Native {
    pub fn new(id: u8) -> Self {
        Self { id }
    }

    pub fn function<W>(&self) -> native::Function<W> {
        match self.id {
            native::CLOCK => Some(native::clock),
            _ => None,
        }
    }
}

impl fmt::Display for Native {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<function (native)>")
    }
}

impl PartialOrd for Native {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}
