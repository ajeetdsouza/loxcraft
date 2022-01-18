mod chunk;
mod op;
mod value;

pub use crate::vm::chunk::Chunk;
use crate::vm::value::Value;

use gc::Gc;
use thiserror::Error;

use std::collections::HashMap;

use self::value::Object;

pub struct VM {
    pub chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    debug: bool,
}

impl VM {
    pub fn new(chunk: Chunk) -> Self {
        Self {
            chunk,
            ip: 0,
            stack: Vec::new(),
            globals: HashMap::new(),
            debug: true,
        }
    }

    pub fn run(&mut self) -> Result<(), RuntimeError> {
        while self.ip < self.chunk.code.len() {
            if self.debug {
                self.chunk.dump_instruction(self.ip);
            }

            match self.read_u8() {
                op::CONSTANT => {
                    let value = self.read_constant().clone();
                    self.push(value);
                }
                op::NIL => self.push(Value::Nil),
                op::FALSE => self.push(Value::Bool(false)),
                op::TRUE => self.push(Value::Bool(true)),
                op::POP => {
                    self.pop();
                }
                op::GET_LOCAL => {
                    let slot = self.read_u8();
                    let value = self
                        .stack
                        .get(slot as usize)
                        .expect("tried to access a non-existent slot")
                        .clone();
                    self.push(value);
                }
                op::SET_LOCAL => {
                    let slot = self.read_u8();
                    let value = self.peek().clone();
                    *self
                        .stack
                        .get_mut(slot as usize)
                        .expect("tried to access a non-existent slot") = value;
                }
                op::GET_GLOBAL => {
                    let name = &match self.read_constant() {
                        Value::Object(Object::String(string)) => string.to_string(),
                        value => panic!(
                            "expected identifier of type 'string', got type '{}'",
                            value.type_()
                        ),
                    };
                    let value = match self.globals.get(name) {
                        Some(value) => value.clone(),
                        None => return Err(RuntimeError::name_not_defined(name)),
                    };
                    self.push(value);
                }
                op::DEFINE_GLOBAL => {
                    let name = match self.read_constant() {
                        Value::Object(Object::String(string)) => string.to_string(),
                        value => panic!(
                            "expected identifier of type 'string', got type '{}'",
                            value.type_()
                        ),
                    };
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                op::SET_GLOBAL => {
                    let name = match self.read_constant() {
                        Value::Object(Object::String(string)) => string.to_string(),
                        value => panic!(
                            "expected identifier of type 'string', got type '{}'",
                            value.type_()
                        ),
                    };
                    let value = self.peek().clone();

                    #[allow(clippy::map_entry)]
                    if self.globals.contains_key(&name) {
                        self.globals.insert(name, value);
                    } else {
                        return Err(RuntimeError::name_not_defined(&name));
                    }
                }
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
                    self.push(Value::Bool(value.bool()));
                }
                op::NEGATE => match self.pop() {
                    Value::Number(value) => self.push(Value::Number(-value)),
                    value => return Err(RuntimeError::type_unary_op("OP_NEGATE", value.type_())),
                },
                op::PRINT => println!("{:?}", self.pop()),
                op::JUMP => {
                    let offset = self.read_u16();
                    self.ip += offset as usize;
                }
                op::JUMP_IF_FALSE => {
                    let offset = self.read_u16();
                    if !self.peek().bool() {
                        self.ip += offset as usize;
                    }
                }
                op::LOOP => {
                    let offset = self.read_u16();
                    self.ip = self
                        .ip
                        .checked_sub(offset as usize)
                        .expect("instruction pointer is negative");
                }
                op::RETURN => {
                    println!("{:?}", self.pop());
                    break;
                }
                op => panic!("encountered an unknown opcode: {:#04x}", op),
            }

            if self.debug {
                print!("{:>5}", "");
                for value in self.stack.iter() {
                    print!("[ {:?} ]", value);
                }
                println!();
            }
        }

        Ok(())
    }

    fn read_u8(&mut self) -> u8 {
        let value = self.chunk.code[self.ip];
        self.ip += 1;
        value
    }

    fn read_u16(&mut self) -> u16 {
        let mut value = self.read_u8() as u16;
        value = (value << 8) | self.read_u8() as u16;
        value
    }

    fn read_constant(&mut self) -> &Value {
        let constant_idx = self.read_u8() as usize;
        &self.chunk.constants[constant_idx]
    }

    fn peek(&mut self) -> &Value {
        self.stack
            .last()
            .expect("stack underflow: tried to peek data, but the stack is empty")
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
    #[error("NameError: {0}")]
    NameError(String),
    #[error("TypeError: {0}")]
    TypeError(String),
}

impl RuntimeError {
    fn name_not_defined(name: &str) -> Self {
        Self::NameError(format!("name '{name}' is not defined"))
    }

    fn type_binary_op(op: &str, type1: &str, type2: &str) -> Self {
        Self::TypeError(format!(
            "unsupported operand type(s) for {op}: '{type1}' and '{type2}'",
        ))
    }

    fn type_unary_op(op: &str, type_: &str) -> Self {
        Self::TypeError(format!("unsupported operand type for {op}: '{type_}'"))
    }
}
