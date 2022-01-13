use iota::iota;

use std::ops::{Add, Div, Mul, Neg, Sub};

struct VM<'a> {
    chunk: &'a Chunk,
    ip: usize,
    stack: Vec<Value>,
    debug: bool,
}

impl<'a> VM<'a> {
    fn new(chunk: &'a Chunk) -> VM {
        VM {
            chunk,
            ip: 0,
            stack: Vec::new(),
            debug: true,
        }
    }

    fn run(&mut self) {
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
                OP_CONSTANT => {
                    let value = self.read_constant().clone();
                    self.stack.push(value);
                }
                OP_ADD => self.op_binary(Add::add),
                OP_SUBTRACT => self.op_binary(Sub::sub),
                OP_MULTIPLY => self.op_binary(Mul::mul),
                OP_DIVIDE => self.op_binary(Div::div),
                OP_NEGATE => self.op_unary(Neg::neg),
                OP_RETURN => {
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

#[derive(Debug, Default)]
struct Chunk {
    code: Vec<u8>,
    lines: Vec<usize>,
    constants: Vec<Value>,
}

#[derive(Clone, Debug)]
enum Value {
    Number(f64),
}

impl Chunk {
    fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    fn add_constant(&mut self, value: Value) -> u8 {
        let idx = self.constants.len() as u8;
        self.constants.push(value);
        idx
    }

    fn disassemble(&self, name: &str) {
        println!("== {} ==", name);
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            print!("{:>4} ", "|");
        } else {
            print!("{:>4} ", self.lines[offset]);
        }

        let instruction = self.code[offset];
        match instruction {
            OP_CONSTANT => self.disassemble_instruction_constant("OP_CONSTANT", offset),
            OP_ADD => self.disassemble_instruction_simple("OP_ADD", offset),
            OP_SUBTRACT => self.disassemble_instruction_simple("OP_SUBTRACT", offset),
            OP_MULTIPLY => self.disassemble_instruction_simple("OP_MULTIPLY", offset),
            OP_DIVIDE => self.disassemble_instruction_simple("OP_DIVIDE", offset),
            OP_NEGATE => self.disassemble_instruction_simple("OP_NEGATE", offset),
            OP_RETURN => self.disassemble_instruction_simple("OP_RETURN", offset),
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

iota! {
    pub const OP_CONSTANT: u8 = iota;
            , OP_ADD
            , OP_SUBTRACT
            , OP_MULTIPLY
            , OP_DIVIDE
            , OP_NEGATE
            , OP_RETURN
}
