mod env;
pub mod error;
mod object;
mod resolver;

use crate::env::Env;
use crate::object::{Callable, Function, Native, Object};
use crate::resolver::Resolver;
use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Program, Span, Stmt, StmtS};

use rustc_hash::FxHashMap;
use thiserror::Error;

use std::time::{SystemTime, UNIX_EPOCH};

type Result<T, E = RuntimeError> = std::result::Result<T, E>;

type Locals = FxHashMap<Span, usize>;

#[derive(Debug)]
pub struct Interpreter {
    globals: Env,
    locals: Locals,
}

impl Default for Interpreter {
    fn default() -> Self {
        let mut globals = Env::default();
        globals
            .define("clock", Object::Native(Native::Clock), &(0..0))
            .unwrap_or_else(|_| unreachable!("unable to define clock in global scope"));
        Self { globals, locals: FxHashMap::default() }
    }
}

impl Interpreter {
    pub fn run(&mut self, program: &Program) -> Result<()> {
        self.locals = Resolver::default().resolve(program)?;
        let globals = &mut self.globals.clone();
        for stmt in &program.stmts {
            self.run_stmt(globals, stmt)?;
        }
        Ok(())
    }

    fn run_stmt(&self, env: &mut Env, stmt_s: &StmtS) -> Result<()> {
        let (stmt, span) = stmt_s;
        match stmt {
            Stmt::Block(block) => {
                let env = &mut Env::with_parent(env);
                for stmt in &block.stmts {
                    self.run_stmt(env, stmt)?;
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.run_expr(env, &expr.value)?;
                Ok(())
            }
            Stmt::For(for_) => {
                let env = &mut Env::with_parent(env);
                if let Some(init) = &for_.init {
                    self.run_stmt(env, init)?;
                }
                let cond = match &for_.cond {
                    Some(cond) => cond,
                    None => &(Expr::Literal(ExprLiteral::Bool(true)), 0..0),
                };
                while self.run_expr(env, cond)?.bool() {
                    self.run_stmt(env, &for_.body)?;
                    if let Some(incr) = &for_.incr {
                        self.run_expr(env, incr)?;
                    }
                }
                Ok(())
            }
            Stmt::Fun(fun) => {
                let object = Object::Function(Function { decl: *fun.clone(), env: env.clone() });
                env.define(&fun.name, object, span)
            }
            Stmt::If(if_) => {
                let cond = self.run_expr(env, &if_.cond)?;
                if cond.bool() {
                    self.run_stmt(env, &if_.then)?;
                } else if let Some(else_) = &if_.else_ {
                    self.run_stmt(env, else_)?;
                }
                Ok(())
            }
            Stmt::Print(print) => {
                let value = self.run_expr(env, &print.value)?;
                println!("{}", value);
                Ok(())
            }
            Stmt::Return(return_) => {
                let object = match &return_.value {
                    Some(value) => self.run_expr(env, value)?,
                    None => Object::Nil,
                };
                Err(RuntimeError::SyntaxError(SyntaxError::ReturnOutsideFunction {
                    object,
                    span: span.clone(),
                }))
            }
            Stmt::Var(var) => {
                let value = match &var.value {
                    Some(value) => self.run_expr(env, value)?,
                    None => Object::Nil,
                };
                env.define(&var.name, value, span)
            }
            Stmt::While(while_) => {
                while self.run_expr(env, &while_.cond)?.bool() {
                    self.run_stmt(env, &while_.body)?;
                }
                Ok(())
            }
            Stmt::Error => unreachable!("interpreter started despite parsing errors"),
        }
    }

    fn run_expr(&self, env: &mut Env, expr_s: &ExprS) -> Result<Object> {
        let (expr, span) = expr_s;
        match expr {
            Expr::Assign(assign) => {
                let value = self.run_expr(env, &assign.value)?;
                env.assign(&assign.name, value.clone(), span)?;
                Ok(value)
            }
            Expr::Call(call) => {
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.run_expr(env, arg))
                    .collect::<Result<Vec<_>>>()?;
                let callee = self.run_expr(env, &call.callee)?;

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
                            for stmt in function.stmts().iter() {
                                match self.run_stmt(env, stmt) {
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
                let lt = self.run_expr(env, &infix.lt)?;
                let mut rt = || self.run_expr(env, &infix.rt);
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
                let rt = self.run_expr(env, &prefix.rt)?;
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
            Expr::Variable(var) => match self.locals.get(span) {
                Some(depth) => Ok(env.read_at(&var.name, *depth)),
                None => self.globals.read(&var.name).ok_or_else(|| {
                    RuntimeError::NameError(NameError::NotDefined {
                        name: var.name.to_string(),
                        span: span.clone(),
                    })
                }),
            },
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
