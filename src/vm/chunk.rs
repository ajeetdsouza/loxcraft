use crate::vm::op;
use crate::vm::value::Value;

use gc::{Finalize, Trace};

#[derive(Clone, Debug, Default, Finalize, Trace)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self::default()
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
            op::GET_LOCAL => self.dump_instruction_byte("OP_GET_LOCAL", offset),
            op::SET_LOCAL => self.dump_instruction_byte("OP_SET_LOCAL", offset),
            op::GET_GLOBAL => self.dump_instruction_constant("OP_GET_GLOBAL", offset),
            op::DEFINE_GLOBAL => self.dump_instruction_constant("OP_DEFINE_GLOBAL", offset),
            op::SET_GLOBAL => self.dump_instruction_constant("OP_SET_GLOBAL", offset),
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
            op::JUMP => self.dump_instruction_jump("OP_JUMP", false, offset),
            op::JUMP_IF_FALSE => self.dump_instruction_jump("OP_JUMP_IF_FALSE", false, offset),
            op::LOOP => self.dump_instruction_jump("OP_LOOP", true, offset),
            op::CALL => self.dump_instruction_byte("OP_CALL", offset),
            op::RETURN => self.dump_instruction_simple("OP_RETURN", offset),
            _ => self.dump_instruction_unknown(offset),
        }
    }

    fn dump_instruction_simple(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }

    fn dump_instruction_byte(&self, name: &str, offset: usize) -> usize {
        let byte = self.code[offset + 1];
        println!("{:<24} {}", name, byte);
        offset + 2
    }

    fn dump_instruction_constant(&self, name: &str, offset: usize) -> usize {
        let idx = self.code[offset + 1];
        let val = &self.constants[idx as usize];
        println!("{:<24} {} {}", name, idx, val);
        offset + 2
    }

    fn dump_instruction_jump(&self, name: &str, neg: bool, offset: usize) -> usize {
        let jump = ((self.code[offset + 1] as usize) << 8) | (self.code[offset + 2] as usize);
        let offset_new = if neg { offset + 3 - jump } else { offset + 3 + jump };
        println!("{:>16} {:>4} -> {}", name, offset, offset_new);
        offset + 3
    }

    fn dump_instruction_unknown(&self, offset: usize) -> usize {
        let instruction = self.code[offset];
        println!("unknown opcode: {:#04x}", instruction);
        offset + 1
    }
}
