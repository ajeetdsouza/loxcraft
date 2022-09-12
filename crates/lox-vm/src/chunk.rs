use crate::op;
use crate::value::Value;

#[derive(Default)]
pub struct Chunk {
    pub ops: Vec<u8>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn write_u8(&mut self, byte: u8) {
        self.ops.push(byte);
    }

    /// Writes a constant to the [`Chunk`] and returns its index. If an equal
    /// [`Value`] is already present, then its index is returned instead.
    pub fn write_constant(&mut self, value: Value) -> u8 {
        let idx = match self.constants.iter().position(|&constant| constant == value) {
            Some(idx) => idx,
            None => {
                self.constants.push(value);
                self.constants.len() - 1
            }
        };
        idx.try_into().expect("too many constants")
    }

    pub fn debug(&self, name: &str) {
        println!("== {name} ==");
        let mut idx = 0;
        while idx < self.ops.len() {
            idx = self.debug_op(idx);
        }
    }

    pub fn debug_op(&self, idx: usize) -> usize {
        print!("{idx:04} ");
        match self.ops[idx] {
            op::CONSTANT => self.debug_op_constant("OP_CONSTANT", idx),
            op::NIL => self.debug_op_simple("OP_NIL", idx),
            op::TRUE => self.debug_op_simple("OP_TRUE", idx),
            op::FALSE => self.debug_op_simple("OP_FALSE", idx),
            op::POP => self.debug_op_simple("OP_POP", idx),
            op::GET_LOCAL => self.debug_op_byte("OP_GET_LOCAL", idx),
            op::SET_LOCAL => self.debug_op_byte("OP_SET_LOCAL", idx),
            op::GET_GLOBAL => self.debug_op_constant("OP_GET_GLOBAL", idx),
            op::DEFINE_GLOBAL => self.debug_op_constant("OP_DEFINE_GLOBAL", idx),
            op::SET_GLOBAL => self.debug_op_constant("OP_SET_GLOBAL", idx),
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
            op::JUMP => self.debug_op_jump("OP_JUMP", idx, true),
            op::JUMP_IF_FALSE => self.debug_op_jump("OP_JUMP_IF_FALSE", idx, true),
            op::LOOP => self.debug_op_jump("OP_LOOP", idx, false),
            op::RETURN => self.debug_op_simple("OP_RETURN", idx),
            byte => self.debug_op_simple(&format!("OP_UNKNOWN({byte:#X})"), idx),
        }
    }

    fn debug_op_simple(&self, name: &str, idx: usize) -> usize {
        println!("{name}");
        idx + 1
    }

    fn debug_op_byte(&self, name: &str, idx: usize) -> usize {
        let byte = self.ops[idx + 1];
        println!("{name:16} {byte:>4}");
        idx + 2
    }

    fn debug_op_constant(&self, name: &str, idx: usize) -> usize {
        let constant_idx = self.ops[idx + 1];
        let constant = &self.constants[constant_idx as usize];
        println!("{name:16} {constant_idx:>4} '{constant}'");
        idx + 2
    }

    fn debug_op_jump(&self, name: &str, idx: usize, is_forward: bool) -> usize {
        let to_offset = u16::from_le_bytes([self.ops[idx + 1], self.ops[idx + 2]]);
        let offset_sign = if is_forward { 1 } else { -1 };
        // The +3 is to account for the 3 byte jump instruction.
        let to_idx = (idx as isize) + (to_offset as isize) * offset_sign + 3;
        println!("{name:16} {idx:>4} -> {to_idx}");
        idx + 3
    }
}