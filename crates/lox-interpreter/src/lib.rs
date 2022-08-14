mod env;
pub mod error;
mod object;

use crate::env::Env;
use crate::object::{Callable, Function, Native, Object};
use lox_syntax::ast::{Expr, ExprLiteral, OpInfix, OpPrefix, Program, Span, Stmt};

use thiserror::Error;

use std::ops::Range;
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T, E = RuntimeError> = std::result::Result<T, E>;

pub struct Interpreter {
    globals: Env,
}

impl Default for Interpreter {
    fn default() -> Self {
        let mut globals = Env::default();
        globals
            .define("clock", Object::Native(Native::Clock), &(0..0))
            .unwrap_or_else(|_| unreachable!("unable to define clock in global scope"));

        Self { globals }
    }
}

impl Interpreter {
    pub fn run(&mut self, program: &Program) -> Result<()> {
        for (stmt, span) in &program.stmts {
            Self::run_stmt(&mut self.globals, stmt, span)?;
        }
        Ok(())
    }

    fn run_stmt(env: &mut Env, stmt: &Stmt, span: &Range<usize>) -> Result<()> {
        match stmt {
            Stmt::Block(block) => {
                let env = &mut Env::with_parent(env);
                for (stmt, span) in &block.stmts {
                    Self::run_stmt(env, stmt, span)?;
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                Self::run_expr(env, &expr.value, span)?;
                Ok(())
            }
            Stmt::For(for_) => {
                let env = &mut Env::with_parent(env);
                if let Some((init, init_span)) = &for_.init {
                    Self::run_stmt(env, init, init_span)?;
                }
                let cond = match &for_.cond {
                    Some(cond) => cond,
                    None => &Expr::Literal(ExprLiteral::Bool(true)),
                };
                while Self::run_expr(env, cond, span)?.bool() {
                    let (body, body_span) = &for_.body;
                    Self::run_stmt(env, body, body_span)?;
                    if let Some(incr) = &for_.incr {
                        Self::run_expr(env, incr, span)?;
                    }
                }
                Ok(())
            }
            Stmt::Fun(fun) => {
                let object = Object::Function(Function { decl: *fun.clone(), env: env.clone() });
                env.define(&fun.name, object, span)
            }
            Stmt::If(if_) => {
                let cond = Self::run_expr(env, &if_.cond, span)?;
                if cond.bool() {
                    let (then, span) = &if_.then;
                    Self::run_stmt(env, then, span)?;
                } else if let Some((else_, span)) = &if_.else_ {
                    Self::run_stmt(env, else_, span)?;
                }
                Ok(())
            }
            Stmt::Print(print) => {
                let value = Self::run_expr(env, &print.value, span)?;
                println!("{}", value);
                Ok(())
            }
            Stmt::Return(return_) => {
                let object = match &return_.value {
                    Some(value) => Self::run_expr(env, value, span)?,
                    None => Object::Nil,
                };
                Err(RuntimeError::SyntaxError(SyntaxError::ReturnOutsideFunction {
                    object,
                    span: span.clone(),
                }))
            }
            Stmt::Var(var) => {
                let value = match &var.value {
                    Some(value) => Self::run_expr(env, value, span)?,
                    None => Object::Nil,
                };
                env.define(&var.name, value, span)
            }
            Stmt::While(while_) => {
                while Self::run_expr(env, &while_.cond, span)?.bool() {
                    let (body, span) = &while_.body;
                    Self::run_stmt(env, body, span)?;
                }
                Ok(())
            }
            Stmt::Error => unreachable!("interpreter started despite parsing errors"),
        }
    }

    fn run_expr(env: &mut Env, expr: &Expr, span: &Range<usize>) -> Result<Object> {
        match expr {
            Expr::Assign(assign) => {
                let value = Self::run_expr(env, &assign.value, span)?;
                env.assign(&assign.name, value.clone(), span)?;
                Ok(value)
            }
            Expr::Call(call) => {
                let args = call
                    .args
                    .iter()
                    .map(|arg| Self::run_expr(env, arg, span))
                    .collect::<Result<Vec<_>>>()?;
                let callee = Self::run_expr(env, &call.callee, span)?;

                match callee {
                    Object::Function(function) => {
                        let exp_args = function.arity();
                        let got_args = args.len();
                        if exp_args != got_args {
                            Err(RuntimeError::TypeError(TypeError::ArityMismatch {
                                name: function.name().to_string(),
                                exp_args,
                                got_args,
                                span: span.clone(),
                            }))
                        } else {
                            let env = &mut Env::with_parent(&function.env);
                            for (param, arg) in function.params().iter().zip(args) {
                                env.define(param, arg, span)?;
                            }
                            for (stmt, span) in function.stmts().iter() {
                                match Self::run_stmt(env, stmt, span) {
                                    Err(RuntimeError::SyntaxError(
                                        SyntaxError::ReturnOutsideFunction { object, .. },
                                    )) => return Ok(object),
                                    result => result?,
                                }
                            }
                            Ok(Object::Nil)
                        }
                    }
                    Object::Native(native) => {
                        let exp_args = native.arity();
                        let got_args = args.len();
                        if exp_args != got_args {
                            Err(RuntimeError::TypeError(TypeError::ArityMismatch {
                                name: native.name().to_string(),
                                exp_args,
                                got_args,
                                span: span.clone(),
                            }))
                        } else {
                            match native {
                                Native::Clock => {
                                    let now = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis()
                                        as f64;
                                    Ok(Object::Number(now / 1000.0))
                                }
                            }
                        }
                    }
                    _ => Err(RuntimeError::TypeError(TypeError::NotCallable {
                        type_: callee.type_().to_string(),
                        span: span.clone(),
                    })),
                }
            }
            Expr::Infix(infix) => {
                let lt = Self::run_expr(env, &infix.lt, span)?;
                let mut rt = || Self::run_expr(env, &infix.rt, span);
                match infix.op {
                    OpInfix::LogicAnd => Ok(Object::Bool(lt.bool() && rt()?.bool())),
                    OpInfix::LogicOr => Ok(Object::Bool(lt.bool() || rt()?.bool())),
                    op => {
                        let rt = rt()?;
                        match (op, lt, rt) {
                            (OpInfix::Add, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Number(a + b))
                            }
                            (OpInfix::Add, Object::String(a), Object::String(b)) => {
                                Ok(Object::String(a + &b))
                            }
                            (OpInfix::Subtract, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Number(a - b))
                            }
                            (OpInfix::Multiply, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Number(a * b))
                            }
                            (OpInfix::Divide, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Number(a / b))
                            }
                            (OpInfix::Less, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Bool(a < b))
                            }
                            (OpInfix::LessEqual, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Bool(a <= b))
                            }
                            (OpInfix::Greater, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Bool(a > b))
                            }
                            (OpInfix::GreaterEqual, Object::Number(a), Object::Number(b)) => {
                                Ok(Object::Bool(a >= b))
                            }
                            (OpInfix::Equal, a, b) => Ok(Object::Bool(a == b)),
                            (OpInfix::NotEqual, a, b) => Ok(Object::Bool(a != b)),
                            (op, a, b) => {
                                Err(RuntimeError::TypeError(TypeError::UnsupportedOperandInfix {
                                    op: op.to_string(),
                                    lt_type: a.type_().to_string(),
                                    rt_type: b.type_().to_string(),
                                    span: span.clone(),
                                }))
                            }
                        }
                    }
                }
            }
            Expr::Literal(literal) => Ok(match literal {
                ExprLiteral::Nil => Object::Nil,
                ExprLiteral::Bool(bool) => Object::Bool(*bool),
                ExprLiteral::Number(number) => Object::Number(*number),
                ExprLiteral::String(string) => Object::String(string.clone()),
            }),
            Expr::Prefix(prefix) => {
                let rt = Self::run_expr(env, &prefix.rt, span)?;
                match prefix.op {
                    OpPrefix::Negate => match &rt {
                        Object::Number(number) => Ok(Object::Number(-number)),
                        val => Err(RuntimeError::TypeError(TypeError::UnsupportedOperandPrefix {
                            op: prefix.op.to_string(),
                            rt_type: val.type_().to_string(),
                            span: span.clone(),
                        })),
                    },
                    OpPrefix::Not => Ok(Object::Bool(!rt.bool())),
                }
            }
            Expr::Variable(var) => env.read(&var.name, span),
        }
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("NameError: {0}")]
    NameError(NameError),
    #[error("SyntaxError: {0}")]
    SyntaxError(SyntaxError),
    #[error("TypeError: {0}")]
    TypeError(TypeError),
}

#[derive(Debug, Error)]
pub enum NameError {
    #[error("name {:?} is already defined", name)]
    AlreadyDefined { name: String, span: Span },
    #[error("name {:?} is not defined", name)]
    NotDefined { name: String, span: Span },
}

#[derive(Debug, Error)]
pub enum SyntaxError {
    #[error("\"return\" outside function")]
    ReturnOutsideFunction { object: Object, span: Span },
}

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("{name}() takes {exp_args} arguments but {got_args} were given")]
    ArityMismatch { name: String, exp_args: usize, got_args: usize, span: Span },
    #[error("{:?} object is not callable", type_)]
    NotCallable { type_: String, span: Span },
    #[error("unsupported operand type(s) for {}: {:?}", op, rt_type)]
    UnsupportedOperandPrefix { op: String, rt_type: String, span: Span },
    #[error("unsupported operand type(s) for {}: {:?} and {:?}", op, lt_type, rt_type)]
    UnsupportedOperandInfix { op: String, lt_type: String, rt_type: String, span: Span },
}
