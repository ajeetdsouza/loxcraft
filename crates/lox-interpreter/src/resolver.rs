use crate::error::{Error, NameError, Result};

use lox_syntax::ast::{Expr, ExprS, Program, Stmt, StmtFun, StmtS, Var};
use rustc_hash::FxHashSet;

#[derive(Debug, Default)]
pub struct Resolver {
    scopes: Vec<FxHashSet<String>>,
}

impl Resolver {
    pub fn resolve(mut self, program: &mut Program) -> Result<()> {
        for stmt_s in program.stmts.iter_mut() {
            self.resolve_stmt(stmt_s)?;
        }
        Ok(())
    }

    fn resolve_stmt(&mut self, stmt_s: &mut StmtS) -> Result<()> {
        let (stmt, _) = stmt_s;
        match stmt {
            Stmt::Block(block) => {
                self.begin_scope();
                for stmt_s in block.stmts.iter_mut() {
                    self.resolve_stmt(stmt_s)?;
                }
                self.end_scope();
            }
            Stmt::Class(class) => {
                if let Some(super_) = &mut class.super_ {
                    self.begin_scope();
                    self.define("super")?;
                    self.resolve_expr(super_);
                }
                self.define(&class.name)?;
                self.begin_scope();
                self.define("this")?;
                for method in class.methods.iter_mut() {
                    self.resolve_fun(method)?;
                }
                self.end_scope();
                if class.super_.is_some() {
                    self.end_scope();
                }
            }
            Stmt::Expr(expr) => self.resolve_expr(&mut expr.value),
            Stmt::For(for_) => {
                self.begin_scope();
                if let Some(init) = &mut for_.init {
                    self.resolve_stmt(init)?;
                }
                if let Some(cond) = &mut for_.cond {
                    self.resolve_expr(cond);
                }
                if let Some(incr) = &mut for_.incr {
                    self.resolve_expr(incr);
                }
                self.resolve_stmt(&mut for_.body)?;
                self.end_scope();
            }
            Stmt::Fun(fun) => {
                self.define(&fun.name)?;
                self.begin_scope();
                for param in &fun.params {
                    self.define(param)?;
                }
                for stmt_s in fun.body.stmts.iter_mut() {
                    self.resolve_stmt(stmt_s)?;
                }
                self.end_scope();
            }
            Stmt::If(if_) => {
                self.resolve_expr(&mut if_.cond);
                self.resolve_stmt(&mut if_.then)?;
                if let Some(else_) = &mut if_.else_ {
                    self.resolve_stmt(else_)?;
                }
            }
            Stmt::Print(print) => self.resolve_expr(&mut print.value),
            Stmt::Return(return_) => {
                if let Some(value_s) = &mut return_.value {
                    self.resolve_expr(value_s);
                }
            }
            Stmt::Var(var) => {
                if let Some(value) = &mut var.value {
                    self.resolve_expr(value);
                }
                self.define(&var.var.name)?;
            }
            Stmt::While(while_) => {
                self.resolve_expr(&mut while_.cond);
                self.resolve_stmt(&mut while_.body)?;
            }
            Stmt::Error => unreachable!("interpreter started despite parsing errors"),
        }
        Ok(())
    }

    fn resolve_expr(&mut self, expr_s: &mut ExprS) {
        let (expr, _) = expr_s;
        match expr {
            Expr::Assign(assign) => {
                self.resolve_expr(&mut assign.value);
                self.access(&mut assign.var);
            }
            Expr::Call(call) => {
                for arg in call.args.iter_mut() {
                    self.resolve_expr(arg);
                }
                self.resolve_expr(&mut call.callee);
            }
            Expr::Get(get) => {
                self.resolve_expr(&mut get.object);
            }
            Expr::Literal(_) => {}
            Expr::Infix(infix) => {
                self.resolve_expr(&mut infix.lt);
                self.resolve_expr(&mut infix.rt);
            }
            Expr::Prefix(prefix) => {
                self.resolve_expr(&mut prefix.rt);
            }
            Expr::Set(set) => {
                self.resolve_expr(&mut set.value);
                self.resolve_expr(&mut set.object);
            }
            Expr::Super(super_) => self.access(&mut super_.super_),
            Expr::Var(var) => self.access(&mut var.var),
        }
    }

    fn resolve_fun(&mut self, fun: &mut StmtFun) -> Result<()> {
        self.define(&fun.name)?;
        self.begin_scope();
        for param in &fun.params {
            self.define(param)?;
        }
        for stmt_s in fun.body.stmts.iter_mut() {
            self.resolve_stmt(stmt_s)?;
        }
        self.end_scope();
        Ok(())
    }

    fn define(&mut self, name: &str) -> Result<()> {
        if self.scopes.len() == 0 {
            return Ok(());
        }
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains(name) {
                return Err(Error::NameError(NameError::AlreadyDefined { name: name.to_string() }));
            } else {
                scope.insert(name.to_string());
            }
        }
        Ok(())
    }

    fn access(&mut self, var: &mut Var) {
        for (depth, scope) in self.scopes.iter_mut().rev().enumerate() {
            if scope.contains(&var.name) {
                var.depth = Some(depth);
                break;
            }
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(FxHashSet::default());
    }

    fn end_scope(&mut self) {
        self.scopes.pop().unwrap_or_else(|| unreachable!("attempted to pop global scope"));
    }
}
