mod chunk;
mod op;
mod value;

pub use crate::vm::chunk::Chunk;
use crate::vm::value::Value;

use anyhow::{bail, Context, Result};

use std::ops::{Add, Div, Mul, Sub};

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

    pub fn run(&mut self) -> Result<()> {
        loop {
            if self.debug {
                print!("{:>5}", "");
                for value in self.stack.iter() {
                    print!("[ {:?} ]", value);
                }
                println!();
                self.chunk.disassemble_instruction(self.ip);
            }

            match self.read_byte() {
                op::CONSTANT => {
                    let value = self.read_constant().clone();
                    self.push(value);
                }
                op::NIL => self.push(Value::Nil),
                op::FALSE => self.push(Value::Bool(false)),
                op::TRUE => self.push(Value::Bool(true)),
                op::EQUAL => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::Bool(a == b));
                }
                op::GREATER => {
                    let b = self.pop_number()?;
                    let a = self.pop_number()?;
                    self.push(Value::Bool(a > b));
                }
                op::LESS => {
                    let b = self.pop_number()?;
                    let a = self.pop_number()?;
                    self.push(Value::Bool(a < b));
                }
                op::ADD => self.op_binary(Add::add)?,
                op::SUBTRACT => self.op_binary(Sub::sub)?,
                op::MULTIPLY => self.op_binary(Mul::mul)?,
                op::DIVIDE => self.op_binary(Div::div)?,
                op::NOT => {
                    let value = self.pop()?;
                    self.push(Value::Bool(value.is_truthy()));
                }
                op::NEGATE => {
                    let value = self.pop_number()?;
                    self.push(Value::Number(-value));
                }
                op::RETURN => {
                    println!("{:?}", self.pop());
                    return Ok(());
                }
                _ => bail!("unknown opcode"),
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

    fn op_binary<F: Fn(f64, f64) -> f64>(&mut self, f: F) -> Result<()> {
        let b = self.pop_number()?;
        let a = self.pop_number()?;
        self.push(Value::Number(f(a, b)));
        Ok(())
    }

    fn pop(&mut self) -> Result<Value> {
        self.stack.pop().context("stack underflow")
    }

    fn pop_number(&mut self) -> Result<f64> {
        match self.pop()? {
            Value::Number(n) => Ok(n),
            value => bail!("expected a number, got a {:?}", value),
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
}
