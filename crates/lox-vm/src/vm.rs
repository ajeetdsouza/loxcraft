use std::hash::BuildHasherDefault;
use std::io::Write;
use std::{mem, ptr};

use arrayvec::ArrayVec;
use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use lox_common::error::{
    AttributeError, Error, ErrorS, IoError, NameError, OverflowError, Result, TypeError,
};
use rustc_hash::FxHasher;

use crate::allocator::GLOBAL;
use crate::compiler::Compiler;
use crate::gc::{Gc, GcAlloc};
use crate::object::{
    Native, ObjectBoundMethod, ObjectClass, ObjectClosure, ObjectFunction, ObjectInstance,
    ObjectNative, ObjectString, ObjectType, ObjectUpvalue,
};
use crate::value::Value;
use crate::{op, util};

const GC_HEAP_GROW_FACTOR: usize = 2;
const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * STACK_MAX_PER_FRAME;
const STACK_MAX_PER_FRAME: usize = u8::MAX as usize + 1;

#[derive(Debug)]
pub struct VM {
    pub globals: HashMap<*mut ObjectString, Value, BuildHasherDefault<FxHasher>>,
    pub open_upvalues: Vec<*mut ObjectUpvalue>,

    pub gc: Gc,
    next_gc: usize,

    /// `frames` is the current stack of frames running in the [`VM`].
    ///
    /// The topmost frame points to the currently running closure, but does not
    /// include a valid instruction pointer / stack pointer.
    frames: ArrayVec<CallFrame, FRAMES_MAX>,
    frame: CallFrame,

    /// `stack` can be safely accessed without bounds checking because:
    /// - Each frame can store a theoretical maximum of `STACK_MAX_PER_FRAME`
    ///   values on the stack.
    /// - The frame count can never exceed `MAX_FRAMES`, otherwise we throw a
    ///   stack overflow error.
    /// - Thus, we can statically allocate a stack of size
    ///   `STACK_MAX = FRAMES_MAX * STACK_MAX_PER_FRAME` and we are
    ///   guaranteed to never exceed this size.
    stack: Box<[Value; STACK_MAX]>,
    stack_top: *mut Value,

    init_string: *mut ObjectString,
    pub source: String,
}

impl VM {
    pub fn run(&mut self, source: &str, stdout: &mut impl Write) -> Result<(), Vec<ErrorS>> {
        let offset = self.source.len();

        self.source.reserve(source.len() + 1);
        self.source.push_str(source);
        self.source.push('\n');

        let function = Compiler::compile(source, offset, &mut self.gc)?;
        self.run_function(function, stdout).map_err(|e| vec![e])
    }

    fn run_function(
        &mut self,
        function: *mut ObjectFunction,
        stdout: &mut impl Write,
    ) -> Result<()> {
        self.stack_top = self.stack.as_mut_ptr();

        self.frames.clear();
        self.frame = CallFrame {
            closure: self.gc.alloc(ObjectClosure::new(function, Vec::new())),
            ip: unsafe { (*function).chunk.ops.as_ptr() },
            stack: self.stack_top,
        };

        loop {
            if cfg!(feature = "vm-trace") {
                let function = unsafe { (*self.frame.closure).function };
                let idx = unsafe { self.frame.ip.offset_from((*function).chunk.ops.as_ptr()) };
                unsafe { (*function).chunk.debug_op(idx as usize) };
            }

            match self.read_u8() {
                op::CONSTANT => {
                    let constant = self.read_value();
                    self.push(constant);
                }
                op::NIL => self.push(Value::NIL),
                op::TRUE => self.push(Value::TRUE),
                op::FALSE => self.push(Value::FALSE),
                op::POP => {
                    self.pop();
                }
                op::GET_LOCAL => {
                    let stack_idx = self.read_u8() as usize;
                    let local = unsafe { *self.frame.stack.add(stack_idx) };
                    self.push(local);
                }
                op::SET_LOCAL => {
                    let stack_idx = self.read_u8() as usize;
                    let local = unsafe { self.frame.stack.add(stack_idx) };
                    let value = self.peek(0);
                    unsafe { *local = *value };
                }
                op::GET_GLOBAL => {
                    let name = unsafe { self.read_value().as_object().string };
                    match self.globals.get(&name) {
                        Some(&value) => self.push(value),
                        None => {
                            return self.error(NameError::NotDefined {
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    }
                }
                op::DEFINE_GLOBAL => {
                    let name = unsafe { self.read_value().as_object().string };
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                op::SET_GLOBAL => {
                    let name = unsafe { self.read_value().as_object().string };
                    let value = unsafe { *self.peek(0) };
                    match self.globals.entry(name) {
                        Entry::Occupied(mut entry) => entry.insert(value),
                        Entry::Vacant(_) => {
                            return self.error(NameError::NotDefined {
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    };
                }
                op::GET_UPVALUE => {
                    let upvalue_idx = self.read_u8() as usize;
                    let object =
                        *unsafe { (*self.frame.closure).upvalues.get_unchecked(upvalue_idx) };
                    let value = unsafe { *(*object).location };
                    self.push(value);
                }
                op::SET_UPVALUE => {
                    let upvalue_idx = self.read_u8() as usize;
                    let object =
                        *unsafe { (*self.frame.closure).upvalues.get_unchecked(upvalue_idx) };
                    let value = unsafe { (*object).location };
                    unsafe { *value = *self.peek(0) };
                }
                op::GET_PROPERTY => {
                    let name = unsafe { self.read_value().as_object().string };
                    let instance = {
                        let value = unsafe { *self.peek(0) };
                        if value.is_object() {
                            let object = value.as_object();
                            if object.type_() == ObjectType::Instance {
                                unsafe { object.instance }
                            } else {
                                return self.error(AttributeError::NoSuchAttribute {
                                    type_: value.type_().to_string(),
                                    name: unsafe { (*name).value.to_string() },
                                });
                            }
                        } else {
                            return self.error(AttributeError::NoSuchAttribute {
                                type_: value.type_().to_string(),
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    };

                    match unsafe { (*instance).fields.get(&name) } {
                        Some(&field) => {
                            self.pop();
                            self.push(field);
                        }
                        None => match unsafe { (*(*instance).class).methods.get(&name) } {
                            Some(&method) => {
                                let bound_method =
                                    self.alloc(ObjectBoundMethod::new(instance, method));
                                self.pop();
                                self.push(bound_method.into());
                            }
                            None => {
                                return self.error(AttributeError::NoSuchAttribute {
                                    type_: unsafe {
                                        (*(*(*instance).class).name).value.to_string()
                                    },
                                    name: unsafe { (*name).value.to_string() },
                                });
                            }
                        },
                    }
                }
                op::SET_PROPERTY => {
                    let name = unsafe { self.read_value().as_object().string };
                    let instance = {
                        let value = self.pop();
                        if value.is_object() {
                            let object = value.as_object();
                            if object.type_() == ObjectType::Instance {
                                unsafe { object.instance }
                            } else {
                                return self.error(AttributeError::NoSuchAttribute {
                                    type_: value.type_().to_string(),
                                    name: unsafe { (*name).value.to_string() },
                                });
                            }
                        } else {
                            return self.error(AttributeError::NoSuchAttribute {
                                type_: value.type_().to_string(),
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    };
                    let value = unsafe { *self.peek(0) };
                    unsafe { (*instance).fields.insert(name, value) };
                }
                op::GET_SUPER => {
                    let name = unsafe { self.read_value().as_object().string };
                    let super_ = unsafe { self.pop().as_object().class };
                    match unsafe { (*super_).methods.get(&name) } {
                        Some(&method) => {
                            let instance = unsafe { (*self.peek(0)).as_object().instance };
                            let bound_method = self.alloc(ObjectBoundMethod::new(instance, method));
                            self.pop();
                            self.push(bound_method.into());
                        }
                        None => {
                            return self.error(AttributeError::NoSuchAttribute {
                                type_: unsafe { (*(*super_).name).value.to_string() },
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    }
                }
                op::EQUAL => self.binary_op(|a, b| (a == b).into()),
                op::NOT_EQUAL => self.binary_op(|a, b| (a != b).into()),
                op::GREATER => self.binary_op_number(|a, b| (a > b).into(), ">")?,
                op::GREATER_EQUAL => self.binary_op_number(|a, b| (a >= b).into(), ">=")?,
                op::LESS => self.binary_op_number(|a, b| (a < b).into(), "<")?,
                op::LESS_EQUAL => self.binary_op_number(|a, b| (a <= b).into(), "<=")?,
                op::ADD => {
                    let b = self.pop();
                    let a = self.pop();

                    if a.is_number() && b.is_number() {
                        self.push((a.as_number() + b.as_number()).into())
                    } else if a.is_object() && b.is_object() {
                        let a = a.as_object();
                        let b = b.as_object();

                        if a.type_() == ObjectType::String && b.type_() == ObjectType::String {
                            let result = unsafe { [(*a.string).value, (*b.string).value] }.concat();
                            let result = Value::from(self.alloc(result));
                            self.push(result);
                        } else {
                            return self.error(TypeError::UnsupportedOperandInfix {
                                op: "+".to_string(),
                                lt_type: a.type_().to_string(),
                                rt_type: b.type_().to_string(),
                            });
                        }
                    } else {
                        return self.error(TypeError::UnsupportedOperandInfix {
                            op: "+".to_string(),
                            lt_type: a.type_().to_string(),
                            rt_type: b.type_().to_string(),
                        });
                    }
                }
                op::SUBTRACT => self.binary_op_number(|a, b| (a - b).into(), "-")?,
                op::MULTIPLY => self.binary_op_number(|a, b| (a * b).into(), "*")?,
                op::DIVIDE => self.binary_op_number(|a, b| (a / b).into(), "/")?,
                op::NOT => {
                    let value = self.pop();
                    self.push(!value);
                }
                op::NEGATE => {
                    let value = self.pop();
                    if value.is_number() {
                        self.push(Value::from(-value.as_number()))
                    } else {
                        return self.error(TypeError::UnsupportedOperandPrefix {
                            op: "-".to_string(),
                            rt_type: value.type_().to_string(),
                        });
                    }
                }
                op::PRINT => {
                    let value = self.pop();
                    if writeln!(stdout, "{value}").is_err() {
                        return self.error(IoError::WriteError { file: "stdout".to_string() });
                    };
                }
                op::JUMP => {
                    let offset = self.read_u16() as usize;
                    self.frame.ip = unsafe { self.frame.ip.add(offset) };
                }
                op::JUMP_IF_FALSE => {
                    let offset = self.read_u16() as usize;
                    let value = self.peek(0);
                    if !(unsafe { *value }.to_bool()) {
                        self.frame.ip = unsafe { self.frame.ip.add(offset) };
                    }
                }
                op::LOOP => {
                    let offset = self.read_u16() as usize;
                    self.frame.ip = unsafe { self.frame.ip.sub(offset) };
                }
                op::CALL => {
                    let arg_count = self.read_u8() as usize;
                    let callee = unsafe { *self.peek(arg_count) };
                    self.call_value(callee, arg_count)?;
                }
                op::INVOKE => {
                    let name = unsafe { self.read_value().as_object().string };
                    let arg_count = self.read_u8() as usize;
                    let instance = unsafe { (*self.peek(arg_count)).as_object().instance };

                    match unsafe { (*instance).fields.get(&name) } {
                        Some(&value) => self.call_value(value, arg_count)?,
                        None => match unsafe { (*(*instance).class).methods.get(&name) } {
                            Some(&method) => self.call_closure(method, arg_count)?,
                            None => {
                                return self.error(AttributeError::NoSuchAttribute {
                                    type_: unsafe {
                                        (*(*(*instance).class).name).value.to_string()
                                    },
                                    name: unsafe { (*name).value.to_string() },
                                });
                            }
                        },
                    }
                }
                op::SUPER_INVOKE => {
                    let name = unsafe { self.read_value().as_object().string };
                    let arg_count = self.read_u8() as usize;
                    let super_ = unsafe { self.pop().as_object().class };

                    match unsafe { (*super_).methods.get(&name) } {
                        Some(&method) => self.call_closure(method, arg_count)?,
                        None => {
                            return self.error(AttributeError::NoSuchAttribute {
                                type_: unsafe { (*(*super_).name).value.to_string() },
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    }
                }
                op::CLOSURE => {
                    let function = unsafe { self.read_value().as_object().function };

                    let upvalue_count = unsafe { (*function).upvalue_count } as usize;
                    let mut upvalues = Vec::with_capacity(upvalue_count);

                    for _ in 0..upvalue_count {
                        let is_local = self.read_u8();
                        let upvalue_idx = self.read_u8() as usize;

                        let upvalue = if is_local != 0 {
                            let location = unsafe { self.frame.stack.add(upvalue_idx) };
                            self.capture_upvalue(location)
                        } else {
                            unsafe { *(*self.frame.closure).upvalues.get_unchecked(upvalue_idx) }
                        };
                        upvalues.push(upvalue);
                    }

                    let closure = self.alloc(ObjectClosure::new(function, upvalues));
                    self.push(closure.into());
                }
                op::CLOSE_UPVALUE => {
                    let last = self.peek(0);
                    self.close_upvalues(last);
                    self.pop();
                }
                op::RETURN => {
                    let value = self.pop();
                    self.close_upvalues(self.frame.stack);

                    self.stack_top = self.frame.stack;
                    match self.frames.pop() {
                        Some(frame) => self.frame = frame,
                        None => break,
                    }
                    self.push(value);
                }
                op::CLASS => {
                    let name = unsafe { self.read_value().as_object().string };
                    let class = self.alloc(ObjectClass::new(name)).into();
                    self.push(class);
                }
                op::INHERIT => {
                    let class = unsafe { self.pop().as_object().class };
                    let super_ = {
                        let value = unsafe { *self.peek(0) };
                        if value.is_object() {
                            let object = value.as_object();
                            if object.type_() == ObjectType::Class {
                                unsafe { object.class }
                            } else {
                                return self.error(TypeError::SuperclassInvalidType {
                                    type_: value.type_().to_string(),
                                });
                            }
                        } else {
                            return self.error(TypeError::SuperclassInvalidType {
                                type_: value.type_().to_string(),
                            });
                        }
                    };
                    unsafe { (*class).methods = (*super_).methods.clone() };
                }
                op::METHOD => {
                    let name = unsafe { self.read_value().as_object().string };
                    let method = unsafe { self.pop().as_object().closure };
                    let class = unsafe { (*self.peek(0)).as_object().class };
                    unsafe { (*class).methods.insert(name, method) };
                }
                _ => util::unreachable(),
            }

            if cfg!(feature = "vm-trace") {
                eprint!("     ");
                let mut stack_ptr = self.frame.stack;
                while stack_ptr < self.stack_top {
                    eprint!("[ {} ]", unsafe { *stack_ptr });
                    stack_ptr = unsafe { stack_ptr.add(1) };
                }
                eprintln!();
            }
        }

        debug_assert_eq!(
            self.frame.stack, self.stack_top,
            "VM finished executing but stack is not empty"
        );
        Ok(())
    }

    fn alloc<T>(&mut self, object: impl GcAlloc<T>) -> T {
        if !cfg!(feature = "gc-off")
            && (cfg!(feature = "gc-stress") || GLOBAL.allocated_bytes() > self.next_gc)
        {
            self.gc();
        }
        self.gc.alloc(object)
    }

    fn gc(&mut self) {
        if cfg!(feature = "gc-trace") {
            eprintln!("-- gc begin");
        }

        self.gc.mark(self.init_string);

        let mut stack_ptr = self.stack.as_ptr();
        while stack_ptr < self.stack_top {
            self.gc.mark(unsafe { *stack_ptr });
            stack_ptr = unsafe { stack_ptr.add(1) };
        }

        for (&name, &value) in &self.globals {
            self.gc.mark(name);
            self.gc.mark(value);
        }

        self.gc.mark(self.frame.closure);
        for frame in &self.frames {
            self.gc.mark(frame.closure);
        }

        for &upvalue in &self.open_upvalues {
            self.gc.mark(upvalue);
        }

        self.gc.trace();
        self.gc.sweep();

        self.next_gc = GLOBAL.allocated_bytes() * GC_HEAP_GROW_FACTOR;

        if cfg!(feature = "gc-trace") {
            eprintln!("-- gc end");
        }
    }

    fn call_value(&mut self, value: Value, arg_count: usize) -> Result<()> {
        if value.is_object() {
            let object = value.as_object();
            match unsafe { (*object.common).type_ } {
                ObjectType::BoundMethod => {
                    let method = unsafe { object.bound_method };
                    unsafe { *self.peek(arg_count) = (*method).this.into() };
                    self.call_closure(unsafe { (*method).closure }, arg_count)?;
                }
                ObjectType::Class => {
                    let class = unsafe { object.class };
                    let instance = self.alloc(ObjectInstance::new(class));
                    unsafe { *self.peek(arg_count) = instance.into() };

                    match unsafe { (*class).methods.get(&self.init_string) } {
                        Some(&init) => self.call_closure(init, arg_count)?,
                        None => {
                            if arg_count != 0 {
                                return self.error(TypeError::ArityMismatch {
                                    name: "init".to_string(),
                                    exp_args: 0,
                                    got_args: arg_count,
                                });
                            }
                        }
                    }
                }
                ObjectType::Closure => self.call_closure(unsafe { object.closure }, arg_count)?,
                ObjectType::Native => {
                    self.pop();
                    let value = match { unsafe { (*object.native).native } } {
                        Native::Clock => {
                            if arg_count != 0 {
                                return self.error(TypeError::ArityMismatch {
                                    name: "clock".to_string(),
                                    exp_args: 0,
                                    got_args: arg_count,
                                });
                            }
                            util::now().into()
                        }
                    };
                    self.push(value);
                }
                _ => {
                    return self.error(TypeError::NotCallable { type_: value.type_().to_string() });
                }
            }
        } else {
            return self.error(TypeError::NotCallable { type_: value.type_().to_string() });
        }
        Ok(())
    }

    fn call_closure(&mut self, closure: *mut ObjectClosure, arg_count: usize) -> Result<()> {
        if self.frames.len() >= self.frames.capacity() {
            return self.error(OverflowError::StackOverflow);
        }

        let function = unsafe { (*closure).function };
        let arity = unsafe { (*function).arity } as usize;
        if arg_count != arity {
            return self.error(TypeError::ArityMismatch {
                name: unsafe { (*(*function).name).value }.to_string(),
                exp_args: arity,
                got_args: arg_count,
            });
        }

        let frame = CallFrame {
            closure,
            ip: unsafe { (*function).chunk.ops.as_ptr() },
            stack: self.peek(arg_count),
        };
        unsafe { self.frames.push_unchecked(mem::replace(&mut self.frame, frame)) };

        Ok(())
    }

    /// Binary operator that acts on any [`Value`].
    fn binary_op(&mut self, op: fn(Value, Value) -> Value) {
        let b = self.pop();
        let a = self.pop();
        self.push(op(a, b));
    }

    /// Binary operator that acts on numbers.
    fn binary_op_number(&mut self, op: fn(f64, f64) -> Value, op_str: &str) -> Result<()> {
        let b = self.pop();
        let a = self.pop();

        if a.is_number() && b.is_number() {
            let value = op(a.as_number(), b.as_number());
            self.push(value);
            Ok(())
        } else {
            self.error(TypeError::UnsupportedOperandInfix {
                op: op_str.to_string(),
                lt_type: a.type_().to_string(),
                rt_type: b.type_().to_string(),
            })
        }
    }

    /// Reads an instruction / byte from the current [`Chunk`].
    fn read_u8(&mut self) -> u8 {
        let byte = unsafe { *self.frame.ip };
        self.frame.ip = unsafe { self.frame.ip.add(1) };
        byte
    }

    /// Reads a 16-bit value from the current [`Chunk`].
    fn read_u16(&mut self) -> u16 {
        let byte1 = self.read_u8();
        let byte2 = self.read_u8();
        u16::from_le_bytes([byte1, byte2])
    }

    /// Reads a [`Value`] from the current [`Chunk`].
    fn read_value(&mut self) -> Value {
        let constant_idx = self.read_u8() as usize;
        let function = unsafe { (*self.frame.closure).function };
        *unsafe { (*function).chunk.constants.get_unchecked(constant_idx) }
    }

    /// Pushes a [`Value`] to the stack.
    fn push(&mut self, value: Value) {
        unsafe { *self.stack_top = value };
        self.stack_top = unsafe { self.stack_top.add(1) };
    }

    /// Pops a [`Value`] from the stack.
    fn pop(&mut self) -> Value {
        self.stack_top = unsafe { self.stack_top.sub(1) };
        unsafe { *self.stack_top }
    }

    /// Peeks a [`Value`] from the stack.
    fn peek(&mut self, n: usize) -> *mut Value {
        unsafe { self.stack_top.sub(n + 1) }
    }

    fn capture_upvalue(&mut self, location: *mut Value) -> *mut ObjectUpvalue {
        match self.open_upvalues.iter().find(|&&upvalue| unsafe { (*upvalue).location } == location)
        {
            Some(&upvalue) => upvalue,
            None => {
                let upvalue = self.alloc(ObjectUpvalue::new(location));
                self.open_upvalues.push(upvalue);
                upvalue
            }
        }
    }

    fn close_upvalues(&mut self, last: *mut Value) {
        for idx in (0..self.open_upvalues.len()).rev() {
            let upvalue = *unsafe { self.open_upvalues.get_unchecked(idx) };
            if last <= unsafe { (*upvalue).location } {
                unsafe { (*upvalue).closed = *(*upvalue).location };
                unsafe { (*upvalue).location = &mut (*upvalue).closed };
                self.open_upvalues.swap_remove(idx);
            }
        }
    }

    /// Wraps an [`Error`] in a span using the offset of the last executed
    /// instruction.
    #[cold]
    fn error(&self, err: impl Into<Error>) -> Result<()> {
        let function = unsafe { (*self.frame.closure).function };
        let idx = unsafe { self.frame.ip.offset_from((*function).chunk.ops.as_ptr()) } as usize;
        let span = unsafe { (*function).chunk.spans[idx - 1].clone() };
        Err((err.into(), span))
    }
}

impl Default for VM {
    fn default() -> Self {
        let mut gc = Gc::default();

        let mut globals = HashMap::with_capacity_and_hasher(256, BuildHasherDefault::default());
        let clock_string = gc.alloc("clock");
        let clock_native = Value::from(gc.alloc(ObjectNative::new(Native::Clock)));
        globals.insert(clock_string, clock_native);

        let init_string = gc.alloc("init");

        Self {
            globals,
            open_upvalues: Vec::with_capacity(256),
            gc,
            next_gc: 1024 * 1024,
            frames: ArrayVec::new(),
            frame: CallFrame {
                closure: ptr::null_mut(),
                ip: ptr::null_mut(),
                stack: ptr::null_mut(),
            },
            stack: Box::new([Value::default(); STACK_MAX]),
            stack_top: ptr::null_mut(),
            init_string,
            source: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct CallFrame {
    closure: *mut ObjectClosure,
    /// Instruction pointer for the current Chunk.
    ///
    /// Accessing `ip` without bounds checking is safe, assuming that the
    /// compiler always outputs correct code. The program stops execution
    /// when it reaches `op::RETURN`.
    ip: *const u8,
    stack: *mut Value,
}
