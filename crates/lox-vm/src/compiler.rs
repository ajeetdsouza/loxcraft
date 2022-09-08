use crate::chunk::Chunk;
use crate::intern::Intern;
use crate::op;
use crate::value::Value;
use lox_syntax;
use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Program, Stmt, StmtS};

pub struct Compiler {
    chunk: Chunk,
}

impl Compiler {
    pub fn compile(source: &str, intern: &mut Intern) -> Chunk {
        let mut compiler = Compiler { chunk: Chunk::default() };
        let (program, errors) = lox_syntax::parse(source);
        assert!(errors.is_empty());
        compiler.compile_program(&program, intern);
        compiler.chunk
    }

    fn compile_program(&mut self, program: &Program, intern: &mut Intern) {
        for stmt in &program.stmts {
            self.compile_stmt(stmt, intern);
        }
        self.emit_u8(op::RETURN);
    }

    fn compile_stmt(&mut self, (stmt, span): &StmtS, intern: &mut Intern) {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(&expr.value, intern);
                self.emit_u8(op::POP);
            }
            Stmt::Print(print) => {
                self.compile_expr(&print.value, intern);
                self.emit_u8(op::PRINT);
            }
            Stmt::Var(var) => {
                match &var.value {
                    Some(value) => self.compile_expr(value, intern),
                    None => self.emit_u8(op::NIL),
                }
                let (name, _) = intern.insert_str(&var.var.name);
                self.emit_u8(op::DEFINE_GLOBAL);
                self.emit_constant(name.into());
            }
            _ => unimplemented!(),
        }
    }

    fn compile_expr(&mut self, (expr, span): &ExprS, intern: &mut Intern) {
        match expr {
            Expr::Assign(assign) => {
                let (name, _) = intern.insert_str(&assign.var.name);
                self.compile_expr(&assign.value, intern);
                self.emit_u8(op::SET_GLOBAL);
                self.emit_constant(name.into());
            }
            Expr::Infix(infix) => {
                self.compile_expr(&infix.lt, intern);
                self.compile_expr(&infix.rt, intern);
                match infix.op {
                    OpInfix::Add => self.emit_u8(op::ADD),
                    OpInfix::Subtract => self.emit_u8(op::SUBTRACT),
                    OpInfix::Multiply => self.emit_u8(op::MULTIPLY),
                    OpInfix::Divide => self.emit_u8(op::DIVIDE),
                    OpInfix::Less => self.emit_u8(op::LESS),
                    OpInfix::LessEqual => self.emit_u8(op::LESS_EQUAL),
                    OpInfix::Greater => self.emit_u8(op::GREATER),
                    OpInfix::GreaterEqual => self.emit_u8(op::GREATER_EQUAL),
                    OpInfix::Equal => self.emit_u8(op::EQUAL),
                    OpInfix::NotEqual => self.emit_u8(op::NOT_EQUAL),
                    OpInfix::LogicAnd => todo!(),
                    OpInfix::LogicOr => todo!(),
                };
            }
            Expr::Literal(literal) => {
                let value = match literal {
                    ExprLiteral::Bool(bool) => (*bool).into(),
                    ExprLiteral::Nil => Value::Nil,
                    ExprLiteral::Number(number) => (*number).into(),
                    ExprLiteral::String(string) => {
                        let (object, _) = intern.insert_str(string);
                        unsafe { (*object).is_marked = true };
                        object.into()
                    }
                };
                self.emit_u8(op::CONSTANT);
                self.emit_constant(value);
            }
            Expr::Prefix(prefix) => {
                self.compile_expr(&prefix.rt, intern);
                match prefix.op {
                    OpPrefix::Negate => self.emit_u8(op::NEGATE),
                    OpPrefix::Not => self.emit_u8(op::NOT),
                };
            }
            Expr::Var(var) => {
                let (name, _) = intern.insert_str(&var.var.name);
                self.emit_u8(op::GET_GLOBAL);
                self.emit_constant(name.into())
            }
            _ => unimplemented!(),
        }
    }

    fn emit_u8(&mut self, byte: u8) {
        self.chunk.write_u8(byte);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant_idx = self.chunk.write_constant(value);
        self.emit_u8(constant_idx);
    }
}
