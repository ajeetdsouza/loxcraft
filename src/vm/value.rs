use gc::{Finalize, Gc, Trace};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    Object(Object),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn type_(&self) -> &str {
        match self {
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::Number(_) => "number",
            Value::Object(object) => match object {
                Object::String(_) => "string",
            },
        }
    }
}

#[derive(Clone, Debug, Finalize, PartialEq, PartialOrd, Trace)]
pub enum Object {
    String(Gc<String>),
}
