use crate::vm::op::Op;
use crate::vm::value::{Function, Native, Object, Value};

use fnv::FnvHashMap;
use thiserror::Error;

use std::io::Write;
use std::mem;
use std::rc::Rc;

use super::op::ConstantIdx;

pub struct VM<W> {
    pub frame: CallFrame,
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: FnvHashMap<Rc<String>, Value>,
    stdout: W,
    debug: bool,
}

impl<W> VM<W> {
    pub fn new(stdout: W, debug: bool) -> Self {
        let mut globals = FnvHashMap::default();
        globals.insert(Rc::new("clock".to_string()), Value::Object(Object::Native(Native::Clock)));

        Self {
            frame: CallFrame::new(Rc::new(Function::new("", 0))),
            frames: Vec::new(),
            stack: Vec::new(),
            globals,
            stdout,
            debug,
        }
    }
}

impl<W: Write> VM<W> {
    pub fn run(&mut self, function: Function) {
        #[cfg(feature = "profiler")]
        let guard = pprof::ProfilerGuard::new(100).unwrap();

        self.frame = CallFrame::new(Rc::new(function));
        if let Err(e) = self.run_internal() {
            println!("{}", e);
            self.dump_trace();
        }

        #[cfg(feature = "profiler")]
        if let Ok(report) = guard.report().build() {
            use pprof::protos::Message;
            use std::fs::{self, File};
            use std::path::PathBuf;

            let dir = PathBuf::from("/tmp/lox");
            fs::create_dir_all(&dir).unwrap();

            let file = File::create(dir.join("flamegraph.svg")).unwrap();
            report.flamegraph(file).unwrap();

            let mut file = File::create(dir.join("profile.pb")).unwrap();
            let profile = report.pprof().unwrap();
            let mut content = Vec::new();
            profile.encode(&mut content).unwrap();
            file.write_all(&content).unwrap();

            println!("profile written to {}", dir.display());
        };
    }

    fn run_internal(&mut self) -> Result<(), RuntimeError> {
        self.stack.clear();

        while self.frame.ip < self.frame.function.chunk.code.len() {
            if self.debug {
                self.frame.function.chunk.dump_op(self.frame.ip);
            }

            match self.read_op() {
                Op::Constant(constant_idx) => {
                    let value = self.read_constant(constant_idx).clone();
                    self.push(value);
                }
                Op::Nil => self.push(Value::Nil),
                Op::False => self.push(Value::Bool(false)),
                Op::True => self.push(Value::Bool(true)),
                Op::Pop => {
                    self.pop();
                }
                Op::GetLocal(stack_idx) => {
                    let value = self.stack[self.frame.slot + stack_idx as usize].clone();
                    self.push(value);
                }
                Op::SetLocal(stack_idx) => {
                    let value = self.peek(0).clone();
                    self.stack[self.frame.slot + stack_idx as usize] = value;
                }
                Op::GetGlobal(constant_idx) => {
                    let name = &match self.read_constant(constant_idx) {
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
                Op::DefineGlobal(constant_idx) => {
                    let name = match self.read_constant(constant_idx) {
                        Value::Object(Object::String(string)) => string.clone(),
                        value => panic!(
                            "expected identifier of type 'string', got type '{}'",
                            value.type_()
                        ),
                    };
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                Op::SetGlobal(constant_idx) => {
                    let name = match self.read_constant(constant_idx) {
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
                Op::Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(a == b));
                }
                Op::Greater => {
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
                Op::Less => {
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
                Op::Add => {
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
                Op::Subtract => {
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
                Op::Multiply => {
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
                Op::Divide => {
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
                Op::Not => {
                    let value = self.pop();
                    self.push(Value::Bool(!value.bool()));
                }
                Op::Negate => match self.pop() {
                    Value::Number(value) => self.push(Value::Number(-value)),
                    value => return Err(RuntimeError::type_unary_op("OP_NEGATE", value.type_())),
                },
                Op::Print => {
                    let value = self.pop();
                    writeln!(self.stdout, "{}", value).unwrap();
                }
                Op::Jump(offset) => {
                    self.frame.ip += offset as usize;
                }
                Op::JumpIfFalse(offset) => {
                    if !self.peek(0).bool() {
                        self.frame.ip += offset as usize;
                    }
                }
                Op::Loop(offset) => {
                    self.frame.ip -= offset as usize;
                }
                Op::Call(arg_count) => {
                    let arg_count = arg_count as usize;
                    let function = self.peek(arg_count).clone();
                    self.call_value(function, arg_count)?;
                }
                Op::Return => {
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
            }

            if self.debug {
                self.dump_stack();
            }
        }

        if self.debug {
            assert!(
                self.stack.is_empty(),
                "the stack should always be empty after executing a program"
            );
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
        function: Rc<Function>,
        arg_count: usize,
    ) -> Result<(), RuntimeError> {
        if arg_count != function.arity {
            return Err(RuntimeError::arity_mismatch(&function.name, function.arity, arg_count));
        }
        // The function itself as well as its parameters have already been
        // pushed to the stack, so we start the CallFrame at that slot.
        let slot = self.stack.len() - arg_count - 1;
        let mut frame = CallFrame::new_at(function, slot);
        mem::swap(&mut self.frame, &mut frame);
        self.frames.push(frame);
        Ok(())
    }

    fn call_native(&mut self, native_: &Native, arg_count: usize) -> Result<(), RuntimeError> {
        let slot = self.stack.len() - arg_count;
        let function = native_.function();
        let args = self.stack.split_off(slot);
        self.stack.pop(); // pop off the native object

        let value = function(&args)?;
        self.stack.push(value);
        Ok(())
    }

    fn read_op(&mut self) -> Op {
        let op = self.frame.function.chunk.code[self.frame.ip];
        self.frame.ip += 1;
        op
    }

    fn read_constant(&self, idx: ConstantIdx) -> &Value {
        &self.frame.function.chunk.constants[idx as usize]
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
    function: Rc<Function>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    pub fn new(function: Rc<Function>) -> Self {
        Self::new_at(function, 0)
    }

    pub fn new_at(function: Rc<Function>, slot: usize) -> Self {
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
