use crate::env::Env;
use crate::object::{Callable, Object};
use crate::Interpreter;

use gc::{Finalize, Gc, Trace};
use lox_common::error::{Error, Result, SyntaxError, TypeError};
use lox_common::types::Span;
use lox_syntax::ast::{Spanned, Stmt, StmtFun};

use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

#[derive(Clone, Debug, Finalize, Trace)]
pub struct Function(Gc<FunctionImpl>);

impl Function {
    pub fn new(decl: &StmtFun, env: &Env) -> Self {
        Function(Gc::new(FunctionImpl { decl: decl.clone(), env: env.clone() }))
    }

    pub fn params(&self) -> &[String] {
        &self.decl.params
    }

    pub fn stmts(&self) -> &[Spanned<Stmt>] {
        &self.decl.body.stmts
    }

    pub fn bind(&self, this: Object) -> Function {
        let mut env = Env::with_parent(&self.env);
        env.insert_unchecked("this", this);
        Function::new(&self.decl, &env)
    }

    /// Checks if the function is a constructor.
    pub fn is_init(&self) -> bool {
        self.env.contains("this") && self.name() == "init"
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
        span: &Span,
    ) -> Result<Object> {
        let env = &mut Env::with_parent(&self.env);
        for (param, arg) in self.params().iter().zip(args) {
            interpreter.insert_var(env, param, arg);
        }
        for stmt_s in self.stmts().iter() {
            match interpreter.run_stmt(env, stmt_s) {
                Err(Error::SyntaxError(SyntaxError::ReturnOutsideFunction { .. })) => {
                    let object = interpreter.return_.take();
                    return if self.is_init() {
                        match object {
                            None => Ok(self.env.get("this").unwrap_or_else(|| {
                                unreachable!(r#""this" not present inside "init" function"#,)
                            })),
                            Some(object) => {
                                Err(Error::TypeError(TypeError::InitInvalidReturnType {
                                    type_: object.type_(),
                                    span: span.clone(),
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
            self.env
                .get("this")
                .unwrap_or_else(|| unreachable!(r#""this" not present inside "init" function"#))
        } else {
            Object::Nil
        })
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<function {}>", self.name())
    }
}

impl Deref for Function {
    type Target = FunctionImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for Function {}

#[allow(clippy::from_over_into)]
impl Into<Object> for Function {
    fn into(self) -> Object {
        Object::Function(self)
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, Finalize, Trace)]
pub struct FunctionImpl {
    #[unsafe_ignore_trace]
    pub decl: StmtFun,
    pub env: Env,
}
