use std::ops::Index;

use arrayvec::ArrayVec;
use lox_common::error::{OverflowError, Result};
use lox_common::types::Span;

use crate::op;
use crate::value::Value;

#[derive(Debug, Default)]
pub struct Chunk {
    pub ops: Vec<u8>,
    pub constants: ArrayVec<Value, 256>,
    pub constant_count: usize,
    pub spans: VecRun<Span>,
}

impl Chunk {
    pub fn write_u8(&mut self, byte: u8, span: &Span) {
        self.ops.push(byte);
        self.spans.push(span.clone());
    }

    /// Writes a constant to the [`Chunk`] and returns its index. If an equal
    /// [`Value`] is already present, then its index is returned instead.
    pub fn write_constant(&mut self, value: Value, span: &Span) -> Result<u8> {
        if self.constant_count == 256 {
            return Err((OverflowError::TooManyConstants.into(), span.clone()));
        }
        self.constant_count += 1;

        let idx = match self.constants.iter().position(|&constant| constant == value) {
            Some(idx) => idx,
            None => {
                self.constants.push(value);
                self.constants.len() - 1
            }
        };
        Ok(idx.try_into().unwrap())
    }

    pub fn debug(&self, name: &str) {
        eprintln!("== {name} ==");
        let mut idx = 0;
        while idx < self.ops.len() {
            idx = self.debug_op(idx);
        }
    }

    pub fn debug_op(&self, idx: usize) -> usize {
        eprint!("{idx:04} ");
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
            op::GET_UPVALUE => self.debug_op_byte("OP_GET_UPVALUE", idx),
            op::SET_UPVALUE => self.debug_op_byte("OP_SET_UPVALUE", idx),
            op::GET_PROPERTY => self.debug_op_constant("OP_GET_PROPERTY", idx),
            op::SET_PROPERTY => self.debug_op_constant("OP_SET_PROPERTY", idx),
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
            op::CALL => self.debug_op_byte("OP_CALL", idx),
            op::CLOSURE => {
                let mut idx = idx + 1;
                let constant_idx = self.ops[idx];
                let constant = &self.constants[constant_idx as usize];
                eprintln!("{name:16} {constant_idx:>4} '{constant}'", name = "OP_CLOSURE");

                let function = unsafe { constant.object().function };
                for _ in 0..unsafe { (*function).upvalue_count } {
                    let offset = idx;

                    idx += 1;
                    let is_local = self.ops[idx];
                    let label = if is_local == 0 { "upvalue" } else { "local" };

                    idx += 1;
                    let upvalue_idx = self.ops[idx];

                    eprintln!("{offset:04} |                     {label} {upvalue_idx}");
                }

                idx + 1
            }
            op::CLOSE_UPVALUE => self.debug_op_simple("OP_CLOSE_UPVALUE", idx),
            op::RETURN => self.debug_op_simple("OP_RETURN", idx),
            op::CLASS => self.debug_op_constant("OP_CLASS", idx),
            op::INHERIT => self.debug_op_simple("OP_INHERIT", idx),
            op::METHOD => self.debug_op_constant("OP_METHOD", idx),
            byte => self.debug_op_simple(&format!("OP_UNKNOWN({byte:#X})"), idx),
        }
    }

    fn debug_op_simple(&self, name: &str, idx: usize) -> usize {
        eprintln!("{name}");
        idx + 1
    }

    fn debug_op_byte(&self, name: &str, idx: usize) -> usize {
        let byte = self.ops[idx + 1];
        eprintln!("{name:16} {byte:>4}");
        idx + 2
    }

    fn debug_op_constant(&self, name: &str, idx: usize) -> usize {
        let constant_idx = self.ops[idx + 1];
        let constant = &self.constants[constant_idx as usize];
        eprintln!("{name:16} {constant_idx:>4} '{constant}'");
        idx + 2
    }

    fn debug_op_jump(&self, name: &str, idx: usize, is_forward: bool) -> usize {
        let to_offset = u16::from_le_bytes([self.ops[idx + 1], self.ops[idx + 2]]);
        let offset_sign = if is_forward { 1 } else { -1 };
        // The +3 is to account for the 3 byte jump instruction.
        let to_idx = (idx as isize) + (to_offset as isize) * offset_sign + 3;
        eprintln!("{name:16} {idx:>4} -> {to_idx}");
        idx + 3
    }
}

/// Run-length encoded [`Vec`]. Useful for storing data with a lot of contiguous
/// runs of the same value.
#[derive(Debug, Default)]
pub struct VecRun<T> {
    values: Vec<Run<T>>,
}

impl<T: Eq> VecRun<T> {
    fn push(&mut self, value: T) {
        match self.values.last_mut() {
            Some(run) if run.value == value && run.count < u8::MAX => {
                run.count += 1;
            }
            _ => self.values.push(Run { value, count: 1 }),
        };
    }
}

impl<T> Index<usize> for VecRun<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let mut count = index;
        for run in &self.values {
            match count.checked_sub(run.count as usize) {
                Some(remaining) => count = remaining,
                None => return &run.value,
            }
        }
        panic!("index out of bounds");
    }
}

#[derive(Debug)]
struct Run<T> {
    value: T,
    count: u8,
}
