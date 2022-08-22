use crate::env::Env;
use crate::error::{AttributeError, Error, Result, SyntaxError, TypeError};
use crate::Interpreter;

use lox_syntax::ast::{Spanned, Stmt, StmtClass, StmtFun};
use rustc_hash::FxHashMap;

use std::cell::RefCell;
use std::fmt::{self, Debug, Display, Formatter};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub enum Object {
    Bool(bool),
    Class(Class),
    Function(Function),
    Instance(Rc<RefCell<Instance>>),
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
            Object::Instance(instance) => write!(f, "<object {}>", instance.borrow().class.name()),
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

    pub fn type_(&self) -> String {
        match self {
            Object::Bool(_) => "bool".to_string(),
            Object::Class(_) => "class".to_string(),
            Object::Function(_) | Object::Native(_) => "function".to_string(),
            Object::Instance(instance) => instance.borrow().class.name().to_string(),
            Object::Nil => "nil".to_string(),
            Object::Number(_) => "number".to_string(),
            Object::String(_) => "string".to_string(),
        }
    }

    pub fn get(&self, name: &str) -> Result<Object> {
        let instance = match &self {
            Object::Instance(instance) => instance.borrow(),
            _ => {
                return Err(Error::AttributeError(AttributeError::NoSuchAttribute {
                    type_: self.type_().to_string(),
                    name: name.to_string(),
                }))
            }
        };

        if let Some(object) = instance.fields.get(name) {
            return Ok(object.clone());
        }

        instance.class.get_method(name, self.clone())
    }

    pub fn set(&mut self, name: &str, value: &Object) -> Result<()> {
        match &self {
            Object::Instance(instance) => {
                instance.borrow_mut().fields.insert(name.to_string(), value.clone());
                Ok(())
            }
            _ => Err(Error::AttributeError(AttributeError::NoSuchAttribute {
                type_: self.type_().to_string(),
                name: name.to_string(),
            })),
        }
    }

    pub fn call(
        &self,
        interpreter: &mut Interpreter,
        env: &mut Env,
        args: Vec<Object>,
    ) -> Result<Object> {
        match &self {
            Object::Class(class) => class.call(interpreter, env, args),
            Object::Function(function) => function.call(interpreter, env, args),
            Object::Native(native) => native.call(interpreter, env, args),
            object => {
                Err(Error::TypeError(TypeError::NotCallable { type_: object.type_().to_string() }))
            }
        }
    }
}

pub trait Callable {
    fn arity(&self) -> usize;

    fn name(&self) -> &str;

    fn call_unchecked(
        &self,
        interpreter: &mut Interpreter,
        env: &mut Env,
        args: Vec<Object>,
    ) -> Result<Object>;

    fn call(
        &self,
        interpreter: &mut Interpreter,
        env: &mut Env,
        args: Vec<Object>,
    ) -> Result<Object> {
        let exp_args = self.arity();
        let got_args = args.len();
        if exp_args != got_args {
            return Err(Error::TypeError(TypeError::ArityMismatch {
                name: self.name().to_string(),
                exp_args,
                got_args,
            }));
        }
        self.call_unchecked(interpreter, env, args)
    }
}

#[derive(Clone)]
pub struct Class {
    pub decl: StmtClass,
    pub super_: Option<Box<Class>>,
    pub methods: FxHashMap<String, Function>,
}

impl Debug for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<class {}>", self.name())
    }
}

impl Callable for Class {
    fn arity(&self) -> usize {
        match self.methods.get("init") {
            Some(function) => function.arity(),
            None => 0,
        }
    }

    fn name(&self) -> &str {
        &self.decl.name
    }

    fn call_unchecked(
        &self,
        interpreter: &mut Interpreter,
        env: &mut Env,
        args: Vec<Object>,
    ) -> Result<Object> {
        let instance = Object::Instance(Rc::new(RefCell::new(Instance {
            class: self.clone(),
            fields: FxHashMap::default(),
        })));
        if let Ok(init) = instance.get("init") {
            match init {
                Object::Function(function) => {
                    let constructor = Object::Function(function.bind(instance.clone()));
                    constructor.call(interpreter, env, args)?;
                }
                _ => unreachable!(
                    "expected {:?} of type {:?}, found {:?} instead",
                    "init",
                    "function",
                    init.type_()
                ),
            }
        }
        Ok(instance)
    }
}

impl Class {
    pub fn get_method(&self, name: &str, this: Object) -> Result<Object> {
        if let Some(method) = self.methods.get(name) {
            Ok(Object::Function(method.clone().bind(this)))
        } else if let Some(super_) = &self.super_ {
            super_.get_method(name, this)
        } else {
            Err(Error::AttributeError(AttributeError::NoSuchAttribute {
                type_: self.name().to_string(),
                name: name.to_string(),
            }))
        }
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

    fn call_unchecked(
        &self,
        interpreter: &mut Interpreter,
        _env: &mut Env,
        args: Vec<Object>,
    ) -> Result<Object> {
        let env = &mut Env::with_parent(&self.env);
        for (param, arg) in self.params().iter().zip(args) {
            interpreter.insert_var(env, param, arg);
        }
        for stmt_s in self.stmts().iter() {
            match interpreter.run_stmt(env, stmt_s) {
                Err(Error::SyntaxError(SyntaxError::ReturnOutsideFunction { object, .. })) => {
                    return if self.is_init() {
                        match object {
                            None => Ok(self.env.get("this").unwrap_or_else(|_| {
                                unreachable!(
                                    "{:?} not present inside {:?} function",
                                    "this", "init"
                                )
                            })),
                            Some(object) => {
                                Err(Error::TypeError(TypeError::InitInvalidReturnType {
                                    type_: object.type_().to_string(),
                                }))
                            }
                        }
                    } else {
                        Ok(object.unwrap_or(Object::Nil))
                    };
                }
                result => result?,
            }
        }
        Ok(if self.is_init() {
            self.env.get("this").unwrap_or_else(|_| {
                unreachable!("{:?} not present inside {:?} function", "this", "init")
            })
        } else {
            Object::Nil
        })
    }
}

impl Function {
    pub fn params(&self) -> &[String] {
        &self.decl.params
    }

    pub fn stmts(&self) -> &[Spanned<Stmt>] {
        &self.decl.body.stmts
    }

    pub fn bind(&self, this: Object) -> Function {
        let mut env = Env::with_parent(&self.env);
        env.insert_unchecked("this", this);
        Function { decl: self.decl.clone(), env }
    }

    /// Checks if the function is a constructor.
    pub fn is_init(&self) -> bool {
        self.env.contains("this") && self.name() == "init"
    }
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub class: Class,
    pub fields: FxHashMap<String, Object>,
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

    fn call_unchecked(
        &self,
        _interpreter: &mut Interpreter,
        _env: &mut Env,
        _args: Vec<Object>,
    ) -> Result<Object> {
        match self {
            Native::Clock => {
                let now =
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                        as f64;
                Ok(Object::Number(now / 1000.0))
            }
        }
    }
}
