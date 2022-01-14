mod chunk;
mod op;
mod value;

pub use crate::vm::chunk::Chunk;
use crate::vm::value::Value;

use std::ops::{Add, Div, Mul, Neg, Sub};

pub struct VM<'a> {
    chunk: &'a Chunk,
    ip: usize,
    stack: Vec<Value>,
    debug: bool,
}

impl<'a> VM<'a> {
    pub fn new(chunk: &'a Chunk) -> Self {
        Self {
            chunk,
            ip: 0,
            stack: Vec::new(),
            debug: true,
        }
    }

    pub fn run(&mut self) {
        loop {
            if self.debug {
                print!("{:>10}", "");
                for value in self.stack.iter() {
                    print!("[ {:?} ]", value);
                }
                println!();
                self.chunk.disassemble_instruction(self.ip);
            }

            match self.read_byte() {
                op::CONSTANT => {
                    let value = self.read_constant().clone();
                    self.stack.push(value);
                }
                op::ADD => self.op_binary(Add::add),
                op::SUBTRACT => self.op_binary(Sub::sub),
                op::MULTIPLY => self.op_binary(Mul::mul),
                op::DIVIDE => self.op_binary(Div::div),
                op::NEGATE => self.op_unary(Neg::neg),
                op::RETURN => {
                    println!("{:?}", self.stack.pop());
                    return;
                }
                _ => {
                    println!("unknown opcode");
                }
            }
        }
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    fn read_constant(&mut self) -> &Value {
        let constant_idx = self.read_byte() as usize;
        &self.chunk.constants[constant_idx]
    }

    fn op_binary<F: Fn(f64, f64) -> f64>(&mut self, f: F) {
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        let result = match (a, b) {
            (Value::Number(a), Value::Number(b)) => Value::Number(f(a, b)),
            _ => panic!("invalid operands to binary op"),
        };
        self.stack.push(result);
    }

    fn op_unary<F: Fn(f64) -> f64>(&mut self, f: F) {
        let value = self.stack.pop().unwrap();
        let result = match value {
            Value::Number(n) => Value::Number(f(n)),
            _ => panic!("invalid operand to unary op"),
        };
        self.stack.push(result);
    }
}
