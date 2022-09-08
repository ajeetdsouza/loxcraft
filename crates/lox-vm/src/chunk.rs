use crate::op;
use crate::value::{ObjectType, Value};

#[derive(Default)]
pub struct Chunk {
    pub ops: Vec<u8>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn write_u8(&mut self, byte: u8) {
        self.ops.push(byte);
    }

    pub fn write_constant(&mut self, value: Value) -> u8 {
        self.constants.push(value);
        let idx = self.constants.len() - 1;
        idx.try_into().expect("too many constants")
    }

    pub fn debug(&self, name: &str) {
        println!("== {name} ==");
        let mut idx = 0;
        while idx < self.ops.len() {
            idx = self.debug_op(idx);
        }
    }

    fn debug_op(&self, idx: usize) -> usize {
        print!("{idx:04} ");
        match self.ops[idx] {
            op::CONSTANT => self.debug_op_constant("OP_CONSTANT", idx),
            op::NIL => self.debug_op_simple("OP_NIL", idx),
            op::TRUE => self.debug_op_simple("OP_TRUE", idx),
            op::FALSE => self.debug_op_simple("OP_FALSE", idx),
            op::POP => self.debug_op_simple("OP_POP", idx),
            op::EQUAL => self.debug_op_simple("OP_EQUAL", idx),
            op::NOT_EQUAL => self.debug_op_simple("OP_NOT_EQUAL", idx),
            op::GREATER => self.debug_op_simple("OP_GREATER", idx),
            op::GREATER_EQUAL => self.debug_op_simple("OP_GREATER_EQUAL", idx),
            op::LESS => self.debug_op_simple("OP_LESS", idx),
            op::LESS_EQUAL => self.debug_op_simple("OP_LESS_EQUAL", idx),
            op::ADD => self.debug_op_simple("OP_ADD", idx),
            op::SUBTRACT => self.debug_op_simple("OP_SUBTRACT", idx),
            op::MULTIPLY => self.debug_op_simple("OP_MULTIPLY", idx),
            op::DIVIDE => self.debug_op_simple("OP_DIVIDE", idx),
            op::NOT => self.debug_op_simple("OP_NOT", idx),
            op::NEGATE => self.debug_op_simple("OP_NEGATE", idx),
            op::PRINT => self.debug_op_simple("OP_PRINT", idx),
            op::RETURN => self.debug_op_simple("OP_RETURN", idx),
            byte => self.debug_op_simple(&format!("OP_UNKNOWN({byte:#X})"), idx),
        }
    }

    fn debug_op_simple(&self, name: &str, idx: usize) -> usize {
        println!("{name}");
        idx + 1
    }

    fn debug_op_constant(&self, name: &str, idx: usize) -> usize {
        let constant_idx = self.ops[idx + 1];
        let constant = &self.constants[constant_idx as usize];
        println!("{name:16} {constant_idx:>4} '{constant}'");
        idx + 2
    }
}
