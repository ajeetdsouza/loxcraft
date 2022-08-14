use crate::error::{Error, NameError, Result};
use crate::interpreter::Locals;

use lox_syntax::ast::{Expr, ExprS, Program, Span, Stmt, StmtS};
use rustc_hash::FxHashSet;

#[derive(Debug, Default)]
pub struct Resolver {
    locals: Locals,
    scopes: Vec<FxHashSet<String>>,
}

impl Resolver {
    pub fn resolve(mut self, program: &Program) -> Result<Locals> {
        for stmt_s in &program.stmts {
            self.resolve_stmt(stmt_s)?;
        }
        Ok(self.locals)
    }

    fn resolve_stmt(&mut self, stmt_s: &StmtS) -> Result<()> {
        let (stmt, span) = stmt_s;
        match stmt {
            Stmt::Block(block) => {
                self.begin_scope();
                for stmt_s in &block.stmts {
                    self.resolve_stmt(stmt_s)?;
                }
                self.end_scope();
            }
            Stmt::Expr(expr) => self.resolve_expr(&expr.value),
            Stmt::For(for_) => {
                self.begin_scope();
                if let Some(init) = &for_.init {
                    self.resolve_stmt(init)?;
                }
                self.resolve_stmt(&for_.body)?;
                if let Some(incr) = &for_.incr {
                    self.resolve_expr(incr);
                }
                self.end_scope();
            }
            Stmt::Fun(fun) => {
                self.define(&fun.name, span)?;
                self.begin_scope();
                for param in &fun.params {
                    self.define(param, span)?;
                }
                for stmt_s in &fun.body.stmts {
                    self.resolve_stmt(stmt_s)?;
                }
                self.end_scope();
            }
            Stmt::If(if_) => {
                self.resolve_expr(&if_.cond);
                self.resolve_stmt(&if_.then)?;
                if let Some(else_) = &if_.else_ {
                    self.resolve_stmt(else_)?;
                }
            }
            Stmt::Print(print) => self.resolve_expr(&print.value),
            Stmt::Return(return_) => {
                if let Some(value) = &return_.value {
                    self.resolve_expr(value);
                }
            }
            Stmt::Var(var) => {
                if let Some(value) = &var.value {
                    self.resolve_expr(value);
                }
                self.define(&var.name, span)?;
            }
            Stmt::While(while_) => {
                self.resolve_expr(&while_.cond);
                self.resolve_stmt(&while_.body)?;
            }
            Stmt::Error => unreachable!("interpreter started despite parsing errors"),
        }
        Ok(())
    }

    fn resolve_expr(&mut self, expr_s: &ExprS) {
        let (expr, span) = expr_s;
        match expr {
            Expr::Assign(assign) => {
                self.resolve_expr(&assign.value);
                self.access(&assign.name, span);
            }
            Expr::Call(call) => {
                for arg in &call.args {
                    self.resolve_expr(arg);
                }
                self.resolve_expr(&call.callee);
            }
            Expr::Literal(_) => {}
            Expr::Infix(infix) => {
                self.resolve_expr(&infix.lt);
                self.resolve_expr(&infix.rt);
            }
            Expr::Prefix(prefix) => {
                self.resolve_expr(&prefix.rt);
            }
            Expr::Variable(var) => self.access(&var.name, span),
        }
    }

    fn define(&mut self, name: &str, span: &Span) -> Result<()> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains(name) {
                Err(Error::NameError(NameError::AlreadyDefined {
                    name: name.to_string(),
                    span: span.clone(),
                }))
            } else {
                scope.insert(name.to_string());
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn access(&mut self, name: &str, span: &Span) {
        for (depth, scope) in self.scopes.iter_mut().rev().enumerate() {
            if scope.contains(name) {
                self.locals.insert(span.clone(), depth);
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
