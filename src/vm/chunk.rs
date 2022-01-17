use crate::syntax::ast::{
    Expr, ExprInfix, ExprLiteral, ExprPrefix, OpInfix, OpPrefix, Stmt, StmtExpr, StmtPrint, StmtVar,
};
use crate::vm::op;
use crate::vm::value::{Object, Value};

use anyhow::{Context, Result};
use gc::Gc;

type CompileResult<T = ()> = Result<T>;

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn compile(&mut self, source: &Stmt) {
        self.compile_stmt(source).unwrap();
    }

    fn emit_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    fn emit_constant(&mut self, value: Value) -> CompileResult {
        self.emit_byte(op::CONSTANT);
        let idx = self.make_constant(value)?;
        self.emit_byte(idx);
        Ok(())
    }

    fn make_constant(&mut self, value: Value) -> CompileResult<u8> {
        let idx = u8::try_from(self.constants.len())
            .ok()
            .context("cannot define more than 256 constants within a chunk")?;
        self.constants.push(value);
        Ok(idx)
    }

    pub fn dump(&self, name: &str) {
        println!("== {} ==", name);
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.dump_instruction(offset);
        }
    }

    pub fn dump_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        // if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
        //     print!("{:>4} ", "|");
        // } else {
        //     print!("{:>4} ", self.lines[offset]);
        // }

        let instruction = self.code[offset];
        match instruction {
            op::CONSTANT => self.dump_instruction_constant("OP_CONSTANT", offset),
            op::NIL => self.dump_instruction_simple("OP_NIL", offset),
            op::FALSE => self.dump_instruction_simple("OP_FALSE", offset),
            op::TRUE => self.dump_instruction_simple("OP_TRUE", offset),
            op::POP => self.dump_instruction_simple("OP_POP", offset),
            op::DEFINE_GLOBAL => self.dump_instruction_constant("OP_DEFINE_GLOBAL", offset),
            op::EQUAL => self.dump_instruction_simple("OP_EQUAL", offset),
            op::GREATER => self.dump_instruction_simple("OP_GREATER", offset),
            op::LESS => self.dump_instruction_simple("OP_LESS", offset),
            op::ADD => self.dump_instruction_simple("OP_ADD", offset),
            op::SUBTRACT => self.dump_instruction_simple("OP_SUBTRACT", offset),
            op::MULTIPLY => self.dump_instruction_simple("OP_MULTIPLY", offset),
            op::DIVIDE => self.dump_instruction_simple("OP_DIVIDE", offset),
            op::NOT => self.dump_instruction_simple("OP_NOT", offset),
            op::NEGATE => self.dump_instruction_simple("OP_NEGATE", offset),
            op::PRINT => self.dump_instruction_simple("OP_PRINT", offset),
            op::RETURN => self.dump_instruction_simple("OP_RETURN", offset),
            _ => self.dump_instruction_unknown(offset),
        }
    }

    fn dump_instruction_simple(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }

    fn dump_instruction_constant(&self, name: &str, offset: usize) -> usize {
        let constant_idx = self.code[offset + 1];
        let constant_val = &self.constants[constant_idx as usize];
        println!("{:<24} {} {:?}", name, constant_idx, constant_val);
        offset + 2
    }

    fn dump_instruction_unknown(&self, offset: usize) -> usize {
        let instruction = self.code[offset];
        println!("unknown opcode: {:#04x}", instruction);
        offset + 1
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> CompileResult {
        match stmt {
            Stmt::Expr(expr) => self.compile_stmt_expr(expr),
            Stmt::Print(print) => self.compile_stmt_print(print),
            Stmt::Var(var) => self.compile_stmt_var(var),
        }
    }

    fn compile_stmt_expr(&mut self, expr: &StmtExpr) -> CompileResult {
        self.compile_expr(&expr.expr)?;
        self.emit_byte(op::POP);
        Ok(())
    }

    fn compile_stmt_print(&mut self, print: &StmtPrint) -> CompileResult {
        self.compile_expr(&print.expr)?;
        self.emit_byte(op::PRINT);
        Ok(())
    }

    fn compile_stmt_var(&mut self, var: &StmtVar) -> CompileResult {
        self.compile_expr(&var.expr)?;

        self.emit_byte(op::DEFINE_GLOBAL);

        let name = Value::Object(Object::String(Gc::new(var.name.to_string())));
        let idx = self.make_constant(name)?;
        self.emit_byte(idx);

        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> CompileResult {
        match expr {
            Expr::Literal(literal) => self.compile_expr_literal(literal),
            Expr::Infix(infix) => self.compile_expr_infix(infix),
            Expr::Prefix(prefix) => self.compile_expr_prefix(prefix),
        }
    }

    fn compile_expr_literal(&mut self, expr: &ExprLiteral) -> CompileResult {
        match expr {
            ExprLiteral::Nil => self.emit_byte(op::NIL),
            ExprLiteral::Bool(false) => self.emit_byte(op::FALSE),
            ExprLiteral::Bool(true) => self.emit_byte(op::TRUE),
            ExprLiteral::Number(number) => {
                let value = Value::Number(*number);
                self.emit_constant(value)?;
            }
            ExprLiteral::String(string) => {
                let object = Object::String(Gc::new(string.to_string()));
                let value = Value::Object(object);
                self.emit_constant(value)?;
            }
        };
        Ok(())
    }

    fn compile_expr_infix(&mut self, expr: &ExprInfix) -> CompileResult {
        self.compile_expr(&expr.lt)?;
        self.compile_expr(&expr.rt)?;

        match expr.op {
            OpInfix::LogicAnd => todo!(),
            OpInfix::LogicOr => todo!(),
            OpInfix::Equal => self.emit_byte(op::EQUAL),
            OpInfix::NotEqual => {
                self.emit_byte(op::EQUAL);
                self.emit_byte(op::NOT);
            }
            OpInfix::Greater => self.emit_byte(op::GREATER),
            OpInfix::GreaterEqual => {
                self.emit_byte(op::LESS);
                self.emit_byte(op::NOT);
            }
            OpInfix::Less => self.emit_byte(op::LESS),
            OpInfix::LessEqual => {
                self.emit_byte(op::GREATER);
                self.emit_byte(op::NOT);
            }
            OpInfix::Add => self.emit_byte(op::ADD),
            OpInfix::Subtract => self.emit_byte(op::SUBTRACT),
            OpInfix::Multiply => self.emit_byte(op::MULTIPLY),
            OpInfix::Divide => self.emit_byte(op::DIVIDE),
        };

        Ok(())
    }

    fn compile_expr_prefix(&mut self, expr: &ExprPrefix) -> CompileResult {
        self.compile_expr(&expr.expr)?;

        match expr.op {
            OpPrefix::Negate => self.emit_byte(op::NEGATE),
            OpPrefix::Not => self.emit_byte(op::NOT),
        };

        Ok(())
    }
}
