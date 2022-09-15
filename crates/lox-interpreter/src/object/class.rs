use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

use gc::{Finalize, Gc, Trace};
use lox_common::error::Result;
use lox_common::types::Span;
use lox_syntax::ast::StmtClass;
use rustc_hash::FxHashMap;

use crate::env::Env;
use crate::object::{Callable, Function, Instance, Object};
use crate::Interpreter;

#[derive(Clone, Debug, Finalize, Trace)]
pub struct Class(Gc<ClassImpl>);

impl Class {
    pub fn new(decl: &StmtClass, super_: Option<Class>, env: &mut Env) -> Self {
        let methods = {
            let mut env = env.clone();
            if let Some(super_) = &super_ {
                env = Env::with_parent(&env);
                env.insert_unchecked("super", super_.clone().into());
            };
            decl.methods.iter().map(|(decl, _)| (decl.name.to_string(), Function::new(decl, &env))).collect()
        };

        let class = ClassImpl { decl: decl.clone(), super_, methods };
        Self(Gc::new(class))
    }

    pub fn method(&self, name: &str, this: Object) -> Option<Object> {
        let function = self.method_helper(name)?;
        Some(Object::Function(function.bind(this)))
    }

    fn method_helper(&self, name: &str) -> Option<&Function> {
        if let Some(method) = self.methods.get(name) {
            Some(method)
        } else if let Some(super_) = &self.super_ {
            super_.method_helper(name)
        } else {
            None
        }
    }
}

impl Callable for Class {
    fn arity(&self) -> usize {
        match self.method_helper("init") {
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
        span: &Span,
    ) -> Result<Object> {
        let instance = Object::Instance(Instance::new(self));
        if let Some(init) = self.method("init", instance.clone()) {
            init.call(interpreter, env, args, span)?;
        }
        Ok(instance)
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<class {}>", self.name())
    }
}

impl Deref for Class {
    type Target = ClassImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for Class {}

#[allow(clippy::from_over_into)]
impl Into<Object> for Class {
    fn into(self) -> Object {
        Object::Class(self)
    }
}

impl PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, Finalize, Trace)]
pub struct ClassImpl {
    #[unsafe_ignore_trace]
    pub decl: StmtClass,
    pub super_: Option<Class>,
    pub methods: FxHashMap<String, Function>,
}
