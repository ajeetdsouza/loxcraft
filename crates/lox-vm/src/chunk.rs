use crate::op::{ConstantIdx, JumpOffset, Op};
use crate::value::Value;

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Op>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self { code: Vec::new(), constants: Vec::new() }
    }

    pub fn dump(&self, name: &str) {
        println!("== {name} ==");
        for idx in 0..self.code.len() {
            self.dump_op(idx);
        }
    }

    pub fn dump_op(&self, idx: usize) {
        print!("{:04} ", idx);
        match self.code[idx] {
            Op::Constant(constant_idx) => self.dump_op_constant("OP_CONSTANT", constant_idx),
            Op::Nil => self.dump_op_simple("OP_NIL"),
            Op::False => self.dump_op_simple("OP_FALSE"),
            Op::True => self.dump_op_simple("OP_TRUE"),
            Op::Pop => self.dump_op_simple("OP_POP"),
            Op::GetLocal(stack_idx) => self.dump_op_byte("OP_GET_LOCAL", stack_idx),
            Op::SetLocal(stack_idx) => self.dump_op_byte("OP_SET_LOCAL", stack_idx),
            Op::GetGlobal(constant_idx) => self.dump_op_constant("OP_GET_GLOBAL", constant_idx),
            Op::DefineGlobal(constant_idx) => {
                self.dump_op_constant("OP_DEFINE_GLOBAL", constant_idx)
            }
            Op::SetGlobal(constant_idx) => self.dump_op_constant("OP_SET_GLOBAL", constant_idx),
            Op::Equal => self.dump_op_simple("OP_EQUAL"),
            Op::Greater => self.dump_op_simple("OP_GREATER"),
            Op::Less => self.dump_op_simple("OP_LESS"),
            Op::Add => self.dump_op_simple("OP_ADD"),
            Op::Subtract => self.dump_op_simple("OP_SUBTRACT"),
            Op::Multiply => self.dump_op_simple("OP_MULTIPLY"),
            Op::Divide => self.dump_op_simple("OP_DIVIDE"),
            Op::Not => self.dump_op_simple("OP_NOT"),
            Op::Negate => self.dump_op_simple("OP_NEGATE"),
            Op::Print => self.dump_op_simple("OP_PRINT"),
            Op::Jump(jump_offset) => self.dump_op_jump("OP_JUMP", idx, jump_offset, false),
            Op::JumpIfFalse(jump_offset) => {
                self.dump_op_jump("OP_JUMP_IF_FALSE", idx, jump_offset, false)
            }
            Op::Loop(jump_offset) => self.dump_op_jump("OP_LOOP", idx, jump_offset, true),
            Op::Call(arg_count) => self.dump_op_byte("OP_CALL", arg_count),
            Op::Closure(constant_idx) => println!(
                "{:<16} {:>4} {}",
                "OP_CLOSURE", constant_idx, self.constants[constant_idx as usize]
            ),
            Op::Return => self.dump_op_simple("OP_RETURN"),
        }
    }

    fn dump_op_byte(&self, name: &str, byte: u8) {
        println!("{:<24} {}", name, byte);
    }

    fn dump_op_constant(&self, name: &str, idx: ConstantIdx) {
        let val = &self.constants[idx as usize];
        println!("{:<24} {} {}", name, idx, val);
    }

    fn dump_op_jump(&self, name: &str, from: usize, offset: JumpOffset, reverse: bool) {
        let to = if reverse { from + 1 - offset as usize } else { from + 1 + offset as usize };
        println!("{:<16} {:>4} -> {}", name, from, to);
    }

    fn dump_op_simple(&self, name: &str) {
        println!("{name}");
    }
}
