mod chunk;
mod op;
mod value;

pub use crate::vm::chunk::Chunk;
use crate::vm::value::Value;

use gc::Gc;
use thiserror::Error;

use self::value::Object;

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

    pub fn run(&mut self) -> Result<(), RuntimeError> {
        while self.ip < self.chunk.code.len() {
            if self.debug {
                self.print_stack();
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
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(a == b));
                }
                op::GREATER => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Number(a), Value::Number(b)) => {
                            self.push(Value::Bool(a > b));
                        }
                        _ => {
                            return Err(RuntimeError::type_binary_op(
                                "OP_GREATER",
                                a.type_(),
                                b.type_(),
                            ))
                        }
                    }
                }
                op::LESS => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Number(a), Value::Number(b)) => {
                            self.push(Value::Bool(a < b));
                        }
                        _ => {
                            return Err(RuntimeError::type_binary_op(
                                "OP_LESS",
                                a.type_(),
                                b.type_(),
                            ))
                        }
                    }
                }
                op::ADD => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Number(a), Value::Number(b)) => self.push(Value::Number(a + b)),
                        (Value::Object(Object::String(a)), Value::Object(Object::String(b))) => {
                            let object = Object::String(Gc::new(a.to_string() + b.as_ref()));
                            self.push(Value::Object(object));
                        }
                        (val1, val2) => {
                            return Err(RuntimeError::type_binary_op(
                                "OP_ADD",
                                val1.type_(),
                                val2.type_(),
                            ))
                        }
                    }
                }
                op::SUBTRACT => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Number(a), Value::Number(b)) => self.push(Value::Number(a - b)),
                        (val1, val2) => {
                            return Err(RuntimeError::type_binary_op(
                                "OP_SUBTRACT",
                                val1.type_(),
                                val2.type_(),
                            ))
                        }
                    }
                }
                op::MULTIPLY => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Number(a), Value::Number(b)) => self.push(Value::Number(a * b)),
                        (val1, val2) => {
                            return Err(RuntimeError::type_binary_op(
                                "OP_MULTIPLY",
                                val1.type_(),
                                val2.type_(),
                            ))
                        }
                    }
                }
                op::DIVIDE => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Number(a), Value::Number(b)) => self.push(Value::Number(a / b)),
                        (val1, val2) => {
                            return Err(RuntimeError::type_binary_op(
                                "OP_DIVIDE",
                                val1.type_(),
                                val2.type_(),
                            ))
                        }
                    }
                }
                op::NOT => {
                    let value = self.pop();
                    self.push(Value::Bool(value.is_truthy()));
                }
                op::NEGATE => match self.pop() {
                    Value::Number(value) => self.push(Value::Number(-value)),
                    value => return Err(RuntimeError::type_unary_op("OP_NEGATE", value.type_())),
                },
                op::RETURN => {
                    println!("{:?}", self.pop());
                    return Ok(());
                }
                op => panic!("encountered an unknown opcode: {:#04x}", op),
            }
        }

        if self.debug {
            self.print_stack();
        }
        Ok(())
    }

    fn print_stack(&self) {
        print!("{:>5}", "");
        for value in self.stack.iter() {
            print!("[ {:?} ]", value);
        }
        println!();
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

    fn pop(&mut self) -> Value {
        self.stack
            .pop()
            .expect("stack underflow: tried to pop data, but the stack is empty")
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("TypeError: {0}")]
    TypeError(String),
}

impl RuntimeError {
    fn type_binary_op(op: &str, type1: &str, type2: &str) -> Self {
        RuntimeError::TypeError(format!(
            "unsupported operand type(s) for {op}: '{type1}' and '{type2}'",
        ))
    }

    fn type_unary_op(op: &str, type_: &str) -> Self {
        RuntimeError::TypeError(format!("unsupported operand type for {op}: '{type_}'"))
    }
}
