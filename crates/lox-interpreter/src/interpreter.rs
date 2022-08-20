use crate::env::Env;
use crate::error::{Error, IoError, Result, SyntaxError, TypeError};
use crate::object::{Class, Function, Native, Object};

use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Program, Stmt, StmtS, Var};

use std::io::Write;

pub struct Interpreter {
    globals: Env,
    stdout: Box<dyn Write>,
}

impl Interpreter {
    pub fn new(stdout: Box<dyn Write>) -> Self {
        let mut globals = Env::default();
        globals.insert_unchecked("clock", Object::Native(Native::Clock));
        Self { globals, stdout: Box::new(stdout) }
    }

    pub fn run(&mut self, program: &Program) -> Result<()> {
        // TODO: Ranges can be duplicated. Find another way to index.
        let env = &mut self.globals.clone();
        for stmt_s in &program.stmts {
            self.run_stmt(env, stmt_s)?;
        }
        Ok(())
    }

    pub fn run_stmt(&mut self, env: &mut Env, stmt_s: &StmtS) -> Result<()> {
        let (stmt, _) = stmt_s;
        match stmt {
            Stmt::Block(block) => {
                let env = &mut Env::with_parent(env);
                for stmt_s in &block.stmts {
                    self.run_stmt(env, stmt_s)?;
                }
            }
            Stmt::Class(class) => {
                let methods = class
                    .methods
                    .iter()
                    .map(|decl| {
                        (decl.name.to_string(), Function { decl: decl.clone(), env: env.clone() })
                    })
                    .collect();
                let object = Object::Class(Class { decl: class.clone(), methods });
                self.insert_var(env, &class.name, object);
            }
            Stmt::Expr(expr) => {
                self.run_expr(env, &expr.value)?;
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
            }
            Stmt::Fun(fun) => {
                let object = Object::Function(Function { decl: *fun.clone(), env: env.clone() });
                self.insert_var(env, &fun.name, object);
            }
            Stmt::If(if_) => {
                let cond = self.run_expr(env, &if_.cond)?;
                if cond.bool() {
                    self.run_stmt(env, &if_.then)?;
                } else if let Some(else_) = &if_.else_ {
                    self.run_stmt(env, else_)?;
                }
            }
            Stmt::Print(print) => {
                let value = self.run_expr(env, &print.value)?;
                writeln!(self.stdout, "{}", value).map_err(|_| {
                    Error::IoError(IoError::WriteError { file: "stdout".to_string() })
                })?;
            }
            Stmt::Return(return_) => {
                let object = match &return_.value {
                    Some(value) => self.run_expr(env, value)?,
                    None => Object::Nil,
                };
                return Err(Error::SyntaxError(SyntaxError::ReturnOutsideFunction { object }));
            }
            Stmt::Var(var) => {
                let value = match &var.value {
                    Some(value) => self.run_expr(env, value)?,
                    None => Object::Nil,
                };
                self.insert_var(env, &var.var.name, value);
            }
            Stmt::While(while_) => {
                while self.run_expr(env, &while_.cond)?.bool() {
                    self.run_stmt(env, &while_.body)?;
                }
            }
            Stmt::Error => unreachable!("interpreter started despite parsing errors"),
        }
        Ok(())
    }

    fn run_expr(&mut self, env: &mut Env, expr_s: &ExprS) -> Result<Object> {
        let (expr, _) = expr_s;
        match expr {
            Expr::Assign(assign) => {
                let value = self.run_expr(env, &assign.value)?;
                self.set_var(env, &assign.var, value.clone())?;
                Ok(value)
            }
            Expr::Call(call) => {
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.run_expr(env, arg))
                    .collect::<Result<Vec<_>>>()?;
                let callee = self.run_expr(env, &call.callee)?;
                callee.call(self, env, args)
            }
            Expr::Get(get) => {
                let object = self.run_expr(env, &get.object)?;
                object.get(&get.name)
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
                        })),
                    },
                    OpPrefix::Not => Ok(Object::Bool(!rt.bool())),
                }
            }
            Expr::Set(set) => {
                let value = self.run_expr(env, &set.value)?;
                let mut object = self.run_expr(env, &set.object)?;
                object.set(&set.name, &value)?;
                Ok(value)
            }
            Expr::Var(var) => self.get_var(env, &var.var),
        }
    }

    fn get_var(&self, env: &Env, var: &Var) -> Result<Object> {
        match var.depth {
            Some(depth) => Ok(env.get_at(&var.name, depth).unwrap_or_else(|_| {
                unreachable!("variable was resolved but could not be found: {:?}", &var.name)
            })),
            None => self.globals.get(&var.name),
        }
    }

    fn set_var(&mut self, env: &mut Env, var: &Var, value: Object) -> Result<()> {
        match var.depth {
            Some(depth) => Ok(env.set_at(&var.name, value, depth).unwrap_or_else(|_| {
                unreachable!("variable was resolved but could not be found: {:?}", &var.name)
            })),
            None => self.globals.set(&var.name, value),
        }
    }

    pub fn insert_var(&self, env: &mut Env, name: &str, value: Object) {
        env.insert_unchecked(name, value);
    }
}
