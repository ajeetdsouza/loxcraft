use crate::env::Env;
use crate::error::{Error, IoError, NameError, Result, SyntaxError, TypeError};
use crate::object::{Callable, Function, Native, Object};
use crate::resolver::Resolver;

use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Program, Span, Stmt, StmtS};
use rustc_hash::FxHashMap;

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Locals = FxHashMap<Span, usize>;

#[derive(Debug)]
pub struct Interpreter<Stdout> {
    globals: Env,
    locals: Locals,
    stdout: Stdout,
}

impl<Stdout: Write> Interpreter<Stdout> {
    pub fn new(stdout: Stdout) -> Self {
        let mut globals = Env::default();
        globals.set("clock", Object::Native(Native::Clock));
        Self { globals, locals: FxHashMap::default(), stdout }
    }

    pub fn run(&mut self, program: &Program) -> Result<()> {
        self.locals = Resolver::default().resolve(program)?;
        let globals = &mut self.globals.clone();
        for stmt in &program.stmts {
            self.run_stmt(globals, stmt)?;
        }
        Ok(())
    }

    fn run_stmt(&mut self, env: &mut Env, stmt_s: &StmtS) -> Result<()> {
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
                env.set(&fun.name, object);
                Ok(())
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
                writeln!(self.stdout, "{}", value).map_err(|_| {
                    Error::IoError(IoError::WriteError {
                        file: "stdout".to_string(),
                        span: span.clone(),
                    })
                })?;
                Ok(())
            }
            Stmt::Return(return_) => {
                let object = match &return_.value {
                    Some(value) => self.run_expr(env, value)?,
                    None => Object::Nil,
                };
                Err(Error::SyntaxError(SyntaxError::ReturnOutsideFunction {
                    object,
                    span: span.clone(),
                }))
            }
            Stmt::Var(var) => {
                let value = match &var.value {
                    Some(value) => self.run_expr(env, value)?,
                    None => Object::Nil,
                };
                env.set(&var.name, value);
                Ok(())
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

    fn run_expr(&mut self, env: &mut Env, expr_s: &ExprS) -> Result<Object> {
        let (expr, span) = expr_s;
        match expr {
            Expr::Assign(assign) => {
                let value = self.run_expr(env, &assign.value)?;
                env.set(&assign.name, value.clone());
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
                            Err(Error::TypeError(TypeError::ArityMismatch {
                                name: function.name().to_string(),
                                exp_args,
                                got_args,
                                span: span.clone(),
                            }))
                        } else {
                            let env = &mut Env::with_parent(&function.env);
                            for (param, arg) in function.params().iter().zip(args) {
                                env.set(param, arg);
                            }
                            for stmt in function.stmts().iter() {
                                match self.run_stmt(env, stmt) {
                                    Err(Error::SyntaxError(
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
                            Err(Error::TypeError(TypeError::ArityMismatch {
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
                    _ => Err(Error::TypeError(TypeError::NotCallable {
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
                                Err(Error::TypeError(TypeError::UnsupportedOperandInfix {
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
                        val => Err(Error::TypeError(TypeError::UnsupportedOperandPrefix {
                            op: prefix.op.to_string(),
                            rt_type: val.type_().to_string(),
                            span: span.clone(),
                        })),
                    },
                    OpPrefix::Not => Ok(Object::Bool(!rt.bool())),
                }
            }
            Expr::Variable(var) => match self.locals.get(span) {
                Some(depth) => Ok(env.get_at(&var.name, *depth).unwrap_or_else(|| {
                    unreachable!("variable was resolved but could not be found: {:?}", var.name)
                })),
                None => self.globals.get(&var.name).ok_or_else(|| {
                    Error::NameError(NameError::NotDefined {
                        name: var.name.to_string(),
                        span: span.clone(),
                    })
                }),
            },
        }
    }
}
