use crate::env::Env;

use lox_syntax::ast::{Spanned, Stmt, StmtClass, StmtFun};

use std::fmt::{self, Debug, Display, Formatter};

#[derive(Clone, Debug)]
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

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Object::Bool(bool) => write!(f, "{}", bool),
            Object::Class(class) => write!(f, "<class {}>", class.name()),
            Object::Function(function) => write!(f, "<function {}>", function.name()),
            Object::Instance(instance) => write!(f, "<instance {}>", instance.class.name()),
            Object::Native(native) => write!(f, "<native {}>", native.name()),
            Object::Nil => write!(f, "nil"),
            Object::Number(number) => write!(f, "{}", number),
            Object::String(string) => write!(f, "{}", string),
        }
    }
}

// TODO: verify how this works once everything is a pointer.
impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Object::Bool(b1), Object::Bool(b2)) => b1 == b2,
            (Object::Native(n1), Object::Native(n2)) => n1 == n2,
            (Object::Nil, Object::Nil) => true,
            (Object::Number(n1), Object::Number(n2)) => n1 == n2,
            (Object::String(s1), Object::String(s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl Object {
    pub fn bool(&self) -> bool {
        !matches!(self, Object::Nil | Object::Bool(false))
    }

    pub fn type_(&self) -> &str {
        match self {
            Object::Bool(_) => "bool",
            Object::Class(_) => "class",
            Object::Function(_) | Object::Native(_) => "function",
            Object::Instance(instance) => instance.class.name(),
            Object::Nil => "nil",
            Object::Number(_) => "number",
            Object::String(_) => "string",
        }
    }
}

pub trait Callable {
    fn arity(&self) -> usize;
    fn name(&self) -> &str;
}

#[derive(Clone)]
pub struct Class {
    pub decl: StmtClass,
}

impl Debug for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<class {}>", self.name())
    }
}

impl Callable for Class {
    fn arity(&self) -> usize {
        0
    }

    fn name(&self) -> &str {
        &self.decl.name
    }
}

#[derive(Clone)]
pub struct Function {
    pub decl: StmtFun,
    pub env: Env,
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<function {}>", self.name())
    }
}

impl Callable for Function {
    fn arity(&self) -> usize {
        self.decl.params.len()
    }

    fn name(&self) -> &str {
        &self.decl.name
    }
}

impl Function {
    pub fn params(&self) -> &[String] {
        &self.decl.params
    }

    pub fn stmts(&self) -> &[Spanned<Stmt>] {
        &self.decl.body.stmts
    }
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub class: Class,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Native {
    Clock,
}

impl Callable for Native {
    fn arity(&self) -> usize {
        match self {
            Native::Clock => 0,
        }
    }

    fn name(&self) -> &str {
        match self {
            Native::Clock => "clock",
        }
    }
}
