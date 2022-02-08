pub mod chunk;
pub mod compiler;
mod native;
mod op;
mod value;

use crate::vm::value::{Function, Native, Object, Value};

use fnv::FnvHashMap;
use gc::Gc;
use thiserror::Error;

use std::io::Write;
use std::mem;
use std::rc::Rc;
use std::time::Instant;

pub struct VM<W> {
    pub frame: CallFrame,
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: FnvHashMap<Rc<String>, Value>,
    stdout: W,
    debug: bool,
    pub start_time: Instant,
}

impl<W> VM<W> {
    pub fn new(stdout: W) -> Self {
        let mut globals = FnvHashMap::default();
        globals.insert(
            Rc::new("clock".to_string()),
            Value::Object(Object::Native(Native::new(native::CLOCK))),
        );

        Self {
            frame: CallFrame::new(Gc::new(Function::new("", 0))),
            frames: Vec::new(),
            stack: Vec::new(),
            globals,
            stdout,
            debug: false,
            start_time: Instant::now(),
        }
    }
}

impl<W: Write> VM<W> {
    pub fn run(&mut self, function: Function) {
        self.frame = CallFrame::new(Gc::new(function));
        if let Err(e) = self.run_internal() {
            println!("{}", e);
            self.dump_trace();
        }
    }

    fn run_internal(&mut self) -> Result<(), RuntimeError> {
        self.stack.clear();

        while self.frame.ip < self.frame.function.chunk.code.len() {
            if self.debug {
                self.frame.function.chunk.dump_instruction(self.frame.ip);
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
                    let slot_idx = self.read_u8();
                    let value = self.stack[self.frame.slot + slot_idx as usize].clone();
                    self.push(value);
                }
                op::SET_LOCAL => {
                    let slot_idx = self.read_u8();
                    let value = self.peek(0).clone();
                    self.stack[self.frame.slot + slot_idx as usize] = value;
                }
                op::GET_GLOBAL => {
                    let name = &match self.read_constant() {
                        Value::Object(Object::String(string)) => string.clone(),
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
                        Value::Object(Object::String(string)) => string.clone(),
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
                        Value::Object(Object::String(string)) => string.clone(),
                        value => panic!(
                            "expected identifier of type 'string', got type '{}'",
                            value.type_()
                        ),
                    };
                    let value = self.peek(0).clone();

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
                            let object = Object::String(Rc::new(a.to_string() + b.as_ref()));
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
                    self.push(Value::Bool(!value.bool()));
                }
                op::NEGATE => match self.pop() {
                    Value::Number(value) => self.push(Value::Number(-value)),
                    value => return Err(RuntimeError::type_unary_op("OP_NEGATE", value.type_())),
                },
                op::PRINT => {
                    let value = self.pop();
                    writeln!(self.stdout, "{}", value).unwrap();
                }
                op::JUMP => {
                    let offset = self.read_u16();
                    self.frame.ip += offset as usize;
                }
                op::JUMP_IF_FALSE => {
                    let offset = self.read_u16();
                    if !self.peek(0).bool() {
                        self.frame.ip += offset as usize;
                    }
                }
                op::LOOP => {
                    let offset = self.read_u16();
                    self.frame.ip -= offset as usize;
                }
                op::CALL => {
                    let arg_count = self.read_u8() as usize;
                    let function = self.peek(arg_count).clone();
                    self.call_value(function, arg_count)?;
                }
                op::RETURN => {
                    let result = self.pop();
                    let frame = match self.frames.pop() {
                        Some(frame) => frame,
                        None => {
                            self.pop();
                            break;
                        }
                    };
                    self.stack.truncate(self.frame.slot);
                    self.push(result);
                    self.frame = frame;
                }
                op => panic!("encountered an unknown opcode: {:#04x}", op),
            }

            if self.debug {
                self.dump_stack();
                // self.dump_trace();
            }
        }

        Ok(())
    }

    fn dump_trace(&self) {
        println!("Traceback (most recent call last):");
        println!("  in {}", self.frame.function);
        for frame in self.frames.iter().rev() {
            println!("  in {}", frame.function);
        }
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), RuntimeError> {
        match &callee {
            Value::Object(Object::Function(function)) => {
                self.call_function(function.clone(), arg_count)
            }
            Value::Object(Object::Native(native)) => self.call_native(native, arg_count),
            value => return Err(RuntimeError::object_not_callable(value.type_())),
        }
    }

    fn call_function(
        &mut self,
        function: Gc<Function>,
        arg_count: usize,
    ) -> Result<(), RuntimeError> {
        if arg_count != function.arity {
            return Err(RuntimeError::arity_mismatch(&function.name, function.arity, arg_count));
        }
        let slot = self.stack.len() - arg_count - 1;
        let mut frame = CallFrame::new_at(function, slot);
        mem::swap(&mut self.frame, &mut frame);
        self.frames.push(frame);
        Ok(())
    }

    fn call_native(&mut self, native_: &Native, arg_count: usize) -> Result<(), RuntimeError> {
        let slot = self.stack.len() - arg_count;
        let function = native_.function().unwrap();
        let args = self.stack.split_off(slot);
        self.stack.pop(); // pop off the native object
        let result = function(self, &args);
        self.stack.push(result);
        Ok(())
    }

    fn read_u8(&mut self) -> u8 {
        let value = self.frame.function.chunk.code[self.frame.ip];
        self.frame.ip += 1;
        value
    }

    fn read_u16(&mut self) -> u16 {
        let mut value = self.read_u8() as u16;
        value = (value << 8) | self.read_u8() as u16;
        value
    }

    fn read_constant(&mut self) -> &Value {
        let constant_idx = self.read_u8() as usize;
        &self.frame.function.chunk.constants[constant_idx]
    }

    fn peek(&mut self, idx: usize) -> &Value {
        &self.stack[self.stack.len() - idx - 1]
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn dump_stack(&self) {
        print!("{:>5}", "");
        for value in self.stack.iter() {
            print!("[ {} ]", value);
        }
        println!();
    }
}

#[derive(Debug)]
pub struct CallFrame {
    function: Gc<Function>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    pub fn new(function: Gc<Function>) -> Self {
        Self::new_at(function, 0)
    }

    pub fn new_at(function: Gc<Function>, slot: usize) -> Self {
        Self { function, ip: 0, slot }
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
    fn arity_mismatch(name: &str, expected: usize, got: usize) -> Self {
        Self::TypeError(format!(
            "{name}() takes {expected} positional arguments but {got} were given",
        ))
    }

    fn name_not_defined(name: &str) -> Self {
        Self::NameError(format!("name '{name}' is not defined"))
    }

    fn object_not_callable(type_: &str) -> Self {
        Self::TypeError(format!("'{type_}' object is not callable"))
    }

    fn type_binary_op(op: &str, type1: &str, type2: &str) -> Self {
        Self::TypeError(format!("unsupported operand type(s) for {op}: '{type1}' and '{type2}'",))
    }

    fn type_unary_op(op: &str, type_: &str) -> Self {
        Self::TypeError(format!("unsupported operand type for {op}: '{type_}'"))
    }
}
