use crate::vm::op::Op;
use crate::vm::value::{Closure, Function, Native, Value};

use fnv::FnvHashMap;
use thiserror::Error;

use std::io::Write;
use std::mem;
use std::rc::Rc;

use super::op::ConstantIdx;

pub struct VM<W1, W2> {
    pub frame: CallFrame,
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: FnvHashMap<Rc<String>, Value>,

    stdout: W1,
    stderr: W2,

    debug: bool,
    profile: bool,
}

impl<W1, W2> VM<W1, W2> {
    pub fn new(stdout: W1, stderr: W2, debug: bool, profile: bool) -> Self {
        let mut globals = FnvHashMap::default();
        globals.insert(Rc::new("clock".to_string()), Value::Native(Native::Clock));

        let closure = Closure { function: Rc::new(Function::new("", 0)) };
        Self {
            frame: CallFrame::new(closure),
            frames: Vec::new(),
            stack: Vec::new(),
            globals,
            stdout,
            stderr,
            debug,
            profile,
        }
    }
}

impl<W1: Write, W2: Write> VM<W1, W2> {
    pub fn run(&mut self, function: Function) {
        let guard = if self.profile { Some(pprof::ProfilerGuard::new(100).unwrap()) } else { None };

        let closure = Closure { function: Rc::new(function) };
        self.frame = CallFrame::new(closure);
        if let Err(e) = self.run_internal() {
            writeln!(self.stderr, "{e}").unwrap();
            self.dump_trace();
        }

        if let Some(guard) = guard {
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
    }

    fn run_internal(&mut self) -> Result<(), RuntimeError> {
        self.stack.clear();

        while self.frame.ip < self.frame.closure.function.chunk.code.len() {
            if self.debug {
                self.frame.closure.function.chunk.dump_op(self.frame.ip);
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
                        Value::String(string) => string.clone(),
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
                        Value::String(string) => string.clone(),
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
                        Value::String(string) => string.clone(),
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
                        (Value::String(a), Value::String(b)) => {
                            let value = Value::String(Rc::new(a.to_string() + b.as_ref()));
                            self.push(value);
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
                    writeln!(self.stdout, "{value}").unwrap();
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
                Op::Closure(constant_idx) => match &self.read_constant(constant_idx) {
                    Value::Function(function) => {
                        let closure = Value::Closure(Closure { function: function.clone() });
                        self.push(closure);
                    }
                    value => {
                        panic!("expected value of type 'function', got type '{}'", value.type_())
                    }
                },
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

        if !self.stack.is_empty() {
            self.dump_stack();
            panic!("stack was not empty after program execution")
        }

        Ok(())
    }

    fn dump_trace(&mut self) {
        writeln!(&mut self.stderr, "Traceback (most recent call last):").unwrap();
        writeln!(&mut self.stderr, "  in {}", self.frame.closure.function).unwrap();
        for frame in self.frames.iter().rev() {
            writeln!(&mut self.stderr, "  in {}", frame.closure.function).unwrap();
        }
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), RuntimeError> {
        match &callee {
            Value::Closure(closure) => self.call_closure(closure, arg_count),
            Value::Native(native) => self.call_native(native, arg_count),
            value => return Err(RuntimeError::value_not_callable(value.type_())),
        }
    }

    fn call_closure(&mut self, closure: &Closure, arg_count: usize) -> Result<(), RuntimeError> {
        if arg_count != closure.function.arity {
            return Err(RuntimeError::arity_mismatch(
                &closure.function.name,
                closure.function.arity,
                arg_count,
            ));
        }
        // The function itself as well as its parameters have already been
        // pushed to the stack, so we start the CallFrame at that slot.
        let slot = self.stack.len() - arg_count - 1;
        let mut frame = CallFrame::new_at(closure.clone(), slot);
        mem::swap(&mut self.frame, &mut frame);
        self.frames.push(frame);
        Ok(())
    }

    fn call_native(&mut self, native_: &Native, arg_count: usize) -> Result<(), RuntimeError> {
        let slot = self.stack.len() - arg_count;
        let function = native_.function();
        let args = self.stack.split_off(slot);
        self.stack.pop(); // pop off the native value

        let value = function(&args)?;
        self.stack.push(value);
        Ok(())
    }

    fn read_op(&mut self) -> Op {
        let op = self.frame.closure.function.chunk.code[self.frame.ip];
        self.frame.ip += 1;
        op
    }

    fn read_constant(&self, idx: ConstantIdx) -> &Value {
        &self.frame.closure.function.chunk.constants[idx as usize]
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
            print!("[ {value} ]");
        }
        println!();
    }
}

#[derive(Debug)]
pub struct CallFrame {
    closure: Closure,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    pub fn new(closure: Closure) -> Self {
        Self::new_at(closure, 0)
    }

    pub fn new_at(closure: Closure, slot: usize) -> Self {
        Self { closure, ip: 0, slot }
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

    fn value_not_callable(type_: &str) -> Self {
        Self::TypeError(format!("'{type_}' value is not callable"))
    }

    fn type_binary_op(op: &str, type1: &str, type2: &str) -> Self {
        Self::TypeError(format!("unsupported operand type(s) for {op}: '{type1}' and '{type2}'",))
    }

    fn type_unary_op(op: &str, type_: &str) -> Self {
        Self::TypeError(format!("unsupported operand type for {op}: '{type_}'"))
    }
}
