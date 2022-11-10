use std::hash::BuildHasherDefault;
use std::io::Write;
use std::{hint, mem, ptr};

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
    ObjectBoundMethod, ObjectClass, ObjectClosure, ObjectFunction, ObjectInstance, ObjectString,
    ObjectType, ObjectUpvalue,
};
use crate::value::{Native, Value};
use crate::{op, util};

const GC_HEAP_GROW_FACTOR: usize = 2;
const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * STACK_MAX_PER_FRAME;
const STACK_MAX_PER_FRAME: usize = u8::MAX as usize + 1;

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
}

impl VM {
    pub fn run(&mut self, source: &str, stdout: &mut impl Write) -> Result<(), Vec<ErrorS>> {
        let function = Compiler::compile(source, &mut self.gc)?;
        self.run_function(function, stdout).map_err(|e| vec![e])
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn run_function(
        &mut self,
        function: *mut ObjectFunction,
        stdout: &mut impl Write,
    ) -> Result<()> {
        self.stack_top = self.stack.as_mut_ptr();

        self.frames.clear();
        self.frame = CallFrame {
            closure: self.gc.alloc(ObjectClosure::new(function, ArrayVec::new())),
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
                op::NIL => self.push(Value::Nil),
                op::TRUE => self.push(true.into()),
                op::FALSE => self.push(false.into()),
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
                    let name = self.read_value().object();
                    match self.globals.get(unsafe { &name.string }) {
                        Some(value) => self.push(*value),
                        None => {
                            return self.err(NameError::NotDefined {
                                name: unsafe { (*name.string).value.to_string() },
                            });
                        }
                    }
                }
                op::DEFINE_GLOBAL => {
                    let name = self.read_value().object();
                    let value = self.pop();
                    self.globals.insert(unsafe { name.string }, value);
                }
                op::SET_GLOBAL => {
                    let name = self.read_value().object();
                    let value = self.peek(0);
                    match self.globals.entry(unsafe { name.string }) {
                        Entry::Occupied(mut entry) => entry.insert(unsafe { *value }),
                        Entry::Vacant(_) => {
                            return self.err(NameError::NotDefined {
                                name: unsafe { (*name.string).value.to_string() },
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
                    let name = unsafe { self.read_value().object().string };
                    let instance = match unsafe { *self.peek(0) } {
                        Value::Object(object)
                            if unsafe { (*object.common).type_ == ObjectType::Instance } =>
                        unsafe { object.instance },
                        value => {
                            return self.err(AttributeError::NoSuchAttribute {
                                type_: value.type_().to_string(),
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    };

                    if let Some(&field) = unsafe { (*instance).fields.get(&name) } {
                        // Pop the instance from the stack.
                        self.pop();
                        self.push(field);
                    } else if let Some(&method) = unsafe { (*(*instance).class).methods.get(&name) }
                    {
                        let bound_method = self.alloc(ObjectBoundMethod::new(instance, method));
                        self.pop();
                        self.push(bound_method.into());
                    } else {
                        return self.err(AttributeError::NoSuchAttribute {
                            type_: unsafe { (*(*(*instance).class).name).value.to_string() },
                            name: unsafe { (*name).value.to_string() },
                        });
                    }
                }
                op::SET_PROPERTY => {
                    let name = unsafe { self.read_value().object().string };
                    let instance = match unsafe { *self.peek(0) } {
                        Value::Object(object)
                            if unsafe { (*object.common).type_ == ObjectType::Instance } =>
                        unsafe { object.instance },
                        value => {
                            return self.err(AttributeError::NoSuchAttribute {
                                type_: value.type_().to_string(),
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    };
                    let value = unsafe { *self.peek(1) };
                    unsafe { (*instance).fields.insert(name, value) };

                    // Pop the instance.
                    self.pop();
                }
                op::GET_SUPER => {
                    let name = unsafe { self.read_value().object().string };
                    let super_ = unsafe { self.pop().object().class };
                    let instance = unsafe { (*self.peek(0)).object().instance };

                    if let Some(&method) = unsafe { (*super_).methods.get(&name) } {
                        let bound_method = self.alloc(ObjectBoundMethod::new(instance, method));
                        self.pop();
                        self.push(bound_method.into());
                    } else {
                        return self.err(AttributeError::NoSuchAttribute {
                            type_: unsafe { (*(*super_).name).value.to_string() },
                            name: unsafe { (*name).value.to_string() },
                        });
                    }
                }
                op::EQUAL => self.binary_op(|a, b| (a == b).into()),
                op::NOT_EQUAL => self.binary_op(|a, b| (a != b).into()),
                op::GREATER => self.binary_op_number(|a, b| (a > b).into(), ">")?,
                op::GREATER_EQUAL => self.binary_op_number(|a, b| (a >= b).into(), ">=")?,
                op::LESS => self.binary_op_number(|a, b| (a < b).into(), "<")?,
                op::LESS_EQUAL => self.binary_op_number(|a, b| (a <= b).into(), "<=")?,
                // op::ADD is a special case, because it works on strings as
                // well as numbers.
                op::ADD => {
                    let b = unsafe { *self.peek(0) };
                    let a = unsafe { *self.peek(1) };
                    match (a, b) {
                        (Value::Number(n1), Value::Number(n2)) => {
                            self.pop();
                            self.pop();
                            self.push((n1 + n2).into())
                        }
                        (Value::Object(o1), Value::Object(o2)) => {
                            match unsafe { ((*o1.common).type_, (*o2.common).type_) } {
                                (ObjectType::String, ObjectType::String) => {
                                    let string =
                                        unsafe { [(*o1.string).value, (*o2.string).value] }
                                            .concat();
                                    let string = self.alloc(string);
                                    self.pop();
                                    self.pop();
                                    self.push(string.into());
                                }
                                _ => {
                                    return self.err(TypeError::UnsupportedOperandInfix {
                                        op: "+".to_string(),
                                        lt_type: a.type_().to_string(),
                                        rt_type: b.type_().to_string(),
                                    });
                                }
                            };
                        }
                        _ => {
                            return self.err(TypeError::UnsupportedOperandInfix {
                                op: "+".to_string(),
                                lt_type: a.type_().to_string(),
                                rt_type: b.type_().to_string(),
                            });
                        }
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
                    match value {
                        Value::Number(number) => self.push((-number).into()),
                        _ => {
                            return self.err(TypeError::UnsupportedOperandPrefix {
                                op: "-".to_string(),
                                rt_type: value.type_().to_string(),
                            });
                        }
                    }
                }
                op::PRINT => {
                    let value = self.pop();
                    if writeln!(stdout, "{value}").is_err() {
                        return self.err(IoError::WriteError { file: "stdout".to_string() });
                    };
                }
                op::JUMP => {
                    let offset = self.read_u16() as usize;
                    self.frame.ip = unsafe { self.frame.ip.add(offset) };
                }
                op::JUMP_IF_FALSE => {
                    let offset = self.read_u16() as usize;
                    let value = self.peek(0);
                    if !(unsafe { *value }.bool()) {
                        self.frame.ip = unsafe { self.frame.ip.add(offset) };
                    }
                }
                op::LOOP => {
                    let offset = self.read_u16() as usize;
                    self.frame.ip = unsafe { self.frame.ip.sub(offset) };
                }
                op::CALL => {
                    let arg_count = self.read_u8();
                    let callee = unsafe { *self.peek(arg_count as usize) };
                    self.call_value(callee, arg_count)?;
                }
                op::INVOKE => {
                    let name = unsafe { self.read_value().object().string };
                    let arg_count = self.read_u8();
                    let instance = unsafe { (*self.peek(arg_count as usize)).object().instance };

                    match unsafe { (*instance).fields.get(&name) } {
                        Some(&value) => self.call_value(value, arg_count)?,
                        None => match unsafe { (*(*instance).class).methods.get(&name) } {
                            Some(&method) => self.call_closure(method, arg_count)?,
                            None => {
                                return self.err(AttributeError::NoSuchAttribute {
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
                    let name = unsafe { self.read_value().object().string };
                    let arg_count = self.read_u8();
                    let super_ = unsafe { self.pop().object().class };

                    match unsafe { (*super_).methods.get(&name) } {
                        Some(&method) => self.call_closure(method, arg_count)?,
                        None => {
                            return self.err(AttributeError::NoSuchAttribute {
                                type_: unsafe { (*(*super_).name).value.to_string() },
                                name: unsafe { (*name).value.to_string() },
                            });
                        }
                    }
                }
                op::CLOSURE => {
                    let function = unsafe { self.read_value().object().function };

                    let upvalue_count = unsafe { (*function).upvalue_count };
                    let mut upvalues = ArrayVec::new();

                    for _ in 0..upvalue_count {
                        let is_local = self.read_u8();
                        let upvalue_idx = self.read_u8();

                        let upvalue = if is_local != 0 {
                            let location = unsafe { self.frame.stack.add(upvalue_idx as usize) };
                            self.capture_upvalue(location)
                        } else {
                            unsafe {
                                *(*self.frame.closure).upvalues.get_unchecked(upvalue_idx as usize)
                            }
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
                    let name = unsafe { self.read_value().object().string };
                    let class = self.alloc(ObjectClass::new(name)).into();
                    self.push(class);
                }
                op::INHERIT => {
                    let class = unsafe { (*self.peek(0)).object().class };
                    let super_ = match unsafe { *self.peek(1) } {
                        Value::Object(object)
                            if unsafe { (*object.common).type_ } == ObjectType::Class =>
                        unsafe { object.class },
                        value => {
                            return self.err(TypeError::SuperclassInvalidType {
                                type_: value.type_().to_string(),
                            });
                        }
                    };
                    unsafe { (*class).methods = (*super_).methods.clone() };
                    self.pop();
                }
                op::METHOD => {
                    let name = unsafe { self.read_value().object().string };
                    let method = unsafe { (*self.peek(0)).object().closure };
                    let class = unsafe { (*self.peek(1)).object().class };
                    unsafe { (*class).methods.insert(name, method) };
                    self.pop();
                }
                _ => unsafe { hint::unreachable_unchecked() },
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

        debug_assert!(
            self.frame.stack == self.stack_top,
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
        println!("done marking");
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

    fn call_value(&mut self, value: Value, arg_count: u8) -> Result<()> {
        match value {
            Value::Object(object) => match unsafe { (*object.common).type_ } {
                ObjectType::BoundMethod => {
                    let method = unsafe { object.bound_method };
                    unsafe { *self.peek(arg_count as usize) = (*method).this.into() };
                    self.call_closure(unsafe { (*method).closure }, arg_count)?;
                }
                ObjectType::Class => {
                    let class = unsafe { object.class };
                    let instance = self.alloc(ObjectInstance::new(class));
                    unsafe { *self.peek(arg_count as usize) = instance.into() };

                    match unsafe { (*class).methods.get(&self.init_string) } {
                        Some(&init) => self.call_closure(init, arg_count)?,
                        None => {
                            if arg_count != 0 {
                                return self.err(TypeError::ArityMismatch {
                                    name: "init".to_string(),
                                    exp_args: 0,
                                    got_args: arg_count,
                                });
                            }
                        }
                    }
                }
                ObjectType::Closure => self.call_closure(unsafe { object.closure }, arg_count)?,
                _ => return self.err(TypeError::NotCallable { type_: value.type_().to_string() }),
            },
            Value::Native(native) => {
                self.pop();
                let value = match native {
                    Native::Clock => {
                        if arg_count != 0 {
                            return self.err(TypeError::ArityMismatch {
                                name: "clock".to_string(),
                                exp_args: 0,
                                got_args: arg_count,
                            });
                        }
                        // TODO: find an alternative that works on WASM.
                        util::now().into()
                    }
                };
                self.push(value);
            }
            _ => return self.err(TypeError::NotCallable { type_: value.type_().to_string() }),
        }
        Ok(())
    }

    fn call_closure(&mut self, closure: *mut ObjectClosure, arg_count: u8) -> Result<()> {
        // Set up the next frame.
        if self.frames.len() >= self.frames.capacity() {
            return self.err(OverflowError::StackOverflow);
        }

        let function = unsafe { (*closure).function };
        let mut frame = CallFrame {
            closure,
            ip: unsafe { (*function).chunk.ops.as_ptr() },
            stack: self.peek(arg_count as usize),
        };

        // Check if the function arity is correct.
        if arg_count != unsafe { (*function).arity } {
            return self.err(TypeError::ArityMismatch {
                name: unsafe { (*(*function).name).value }.to_string(),
                exp_args: unsafe { (*function).arity },
                got_args: arg_count,
            });
        }

        // Set up the current closure.
        mem::swap(&mut frame, &mut self.frame);
        self.frames.push(frame);

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
        let (Value::Number(a), Value::Number( b)) = (a, b) else {
            return self.err(TypeError::UnsupportedOperandInfix {
                op: op_str.to_string(),
                lt_type: a.type_().to_string(),
                rt_type: b.type_().to_string(),
            });
        };
        self.push(op(a, b));
        Ok(())
    }

    /// Reads an instruction / byte from the current [`Chunk`].
    #[inline(always)]
    fn read_u8(&mut self) -> u8 {
        let byte = unsafe { *self.frame.ip };
        self.frame.ip = unsafe { self.frame.ip.add(1) };
        byte
    }

    /// Reads a 16-bit value from the current [`Chunk`].
    #[inline(always)]
    fn read_u16(&mut self) -> u16 {
        let byte1 = self.read_u8();
        let byte2 = self.read_u8();
        u16::from_le_bytes([byte1, byte2])
    }

    /// Reads a [`Value`] from the current [`Chunk`].
    #[inline(always)]
    fn read_value(&mut self) -> Value {
        let constant_idx = self.read_u8() as usize;
        let function = unsafe { (*self.frame.closure).function };
        *unsafe { (*function).chunk.constants.get_unchecked(constant_idx) }
    }

    /// Pushes a [`Value`] to the stack.
    #[inline(always)]
    fn push(&mut self, value: Value) {
        unsafe { *self.stack_top = value };
        self.stack_top = unsafe { self.stack_top.add(1) };
    }

    /// Pops a [`Value`] from the stack.
    #[inline(always)]
    fn pop(&mut self) -> Value {
        self.stack_top = unsafe { self.stack_top.sub(1) };
        unsafe { *self.stack_top }
    }

    /// Peeks a [`Value`] from the stack.
    #[inline(always)]
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

    /// Wraps an error in a span by checking the offset of the last executed
    /// instruction.
    fn err(&self, err: impl Into<Error>) -> Result<()> {
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
        let clock = gc.alloc("clock");
        globals.insert(clock, Native::Clock.into());

        let frames = ArrayVec::new();
        let frame =
            CallFrame { closure: ptr::null_mut(), ip: ptr::null_mut(), stack: ptr::null_mut() };

        let stack = Box::new([Value::default(); STACK_MAX]);
        let stack_top = ptr::null_mut();

        let init_string = gc.alloc("init");

        Self {
            globals,
            open_upvalues: Vec::with_capacity(256),
            gc,
            next_gc: 1024 * 1024,
            frames,
            frame,
            stack,
            stack_top,
            init_string,
        }
    }
}

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
