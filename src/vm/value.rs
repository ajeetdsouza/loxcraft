#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }
}
