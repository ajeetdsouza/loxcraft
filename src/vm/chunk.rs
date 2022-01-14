use crate::syntax::ast::{Expr, ExprInfix, ExprLiteral, ExprPrefix, OpInfix, OpPrefix, Visitor};
use crate::vm::op;
use crate::vm::value::Value;

use anyhow::Result;

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

    pub fn compile(&mut self, source: &Expr) {
        self.visit_expr(source).unwrap();
    }

    fn emit_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    fn emit_constant(&mut self, value: Value) {
        let idx = self.constants.len() as u8;
        self.constants.push(value);

        self.emit_byte(op::CONSTANT);
        self.emit_byte(idx);
    }

    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        // if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
        //     print!("{:>4} ", "|");
        // } else {
        //     print!("{:>4} ", self.lines[offset]);
        // }

        let instruction = self.code[offset];
        match instruction {
            op::CONSTANT => self.disassemble_instruction_constant("op::CONSTANT", offset),
            op::ADD => self.disassemble_instruction_simple("op::ADD", offset),
            op::SUBTRACT => self.disassemble_instruction_simple("op::SUBTRACT", offset),
            op::MULTIPLY => self.disassemble_instruction_simple("op::MULTIPLY", offset),
            op::DIVIDE => self.disassemble_instruction_simple("op::DIVIDE", offset),
            op::NEGATE => self.disassemble_instruction_simple("op::NEGATE", offset),
            op::RETURN => self.disassemble_instruction_simple("op::RETURN", offset),
            _ => self.disassemble_instruction_unknown(offset),
        }
    }

    fn disassemble_instruction_simple(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }

    fn disassemble_instruction_constant(&self, name: &str, offset: usize) -> usize {
        let constant_idx = self.code[offset + 1];
        let constant_val = &self.constants[constant_idx as usize];
        println!("{:<24} {} '{:?}'", name, constant_idx, constant_val);
        offset + 2
    }

    fn disassemble_instruction_unknown(&self, offset: usize) -> usize {
        let instruction = self.code[offset];
        println!("unknown opcode: {:#04x}", instruction);
        offset + 1
    }
}

impl Visitor for Chunk {
    type Result = Result<()>;

    fn visit_expr_literal(&mut self, expr: &ExprLiteral) -> Self::Result {
        let value = match expr {
            ExprLiteral::Number(number) => Value::Number(*number),
            _ => todo!(),
        };
        self.emit_constant(value);
        Ok(())
    }

    fn visit_expr_infix(&mut self, expr: &ExprInfix) -> Self::Result {
        self.visit_expr(&expr.rt)?;
        self.visit_expr(&expr.lt)?;

        let op = match expr.op {
            OpInfix::Add => op::ADD,
            OpInfix::Subtract => op::SUBTRACT,
            OpInfix::Multiply => op::MULTIPLY,
            OpInfix::Divide => op::DIVIDE,
            _ => todo!(),
        };
        self.emit_byte(op);

        Ok(())
    }

    fn visit_expr_prefix(&mut self, expr: &ExprPrefix) -> Self::Result {
        self.visit_expr(&expr.expr)?;

        let op = match expr.op {
            OpPrefix::Negate => op::NEGATE,
            _ => todo!(),
        };
        self.emit_byte(op);

        Ok(())
    }
}
