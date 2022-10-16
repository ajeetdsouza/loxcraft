use std::hash::BuildHasherDefault;
use std::io::Write;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{hint, ptr};

use arrayvec::ArrayVec;
use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use lox_common::error::{
    AttributeError, ErrorS, IoError, NameError, OverflowError, Result,
    TypeError,
};
use rustc_hash::FxHasher;

use crate::allocator::GLOBAL;
use crate::compiler::Compiler;
use crate::gc::{Gc, GcAlloc};
use crate::object::{
    ObjectClass, ObjectClosure, ObjectFunction, ObjectInstance, ObjectString,
    ObjectType, ObjectUpvalue,
};
use crate::op;
use crate::value::{Native, Value};

const GC_HEAP_GROW_FACTOR: usize = 2;
const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * STACK_MAX_PER_FRAME;
const STACK_MAX_PER_FRAME: usize = u8::MAX as usize + 1;

pub struct VM {
    pub globals:
        HashMap<*mut ObjectString, Value, BuildHasherDefault<FxHasher>>,
    pub open_upvalues: Vec<*mut ObjectUpvalue>,

    pub gc: Gc,
    next_gc: usize,

    /// `frames` is the current stack of frames running in the [`VM`].
    ///
    /// The topmost frame points to the currently running closure, but does not
    /// include a valid instruction pointer / stack pointer.
    frames: ArrayVec<CallFrame, FRAMES_MAX>,

    /// `stack` can be safely accessed without bounds checking because:
    /// - Each frame can store a theoretical maximum of `STACK_MAX_PER_FRAME`
    ///   values on the stack.
    /// - The frame count can never exceed `MAX_FRAMES`, otherwise we throw a
    ///   stack overflow error.
    /// - Thus, we can statically allocate a stack of size
    ///   `STACK_MAX = FRAMES_MAX * STACK_MAX_PER_FRAME` and we are
    ///   guaranteed to never exceed this size.
    stack: Pin<Box<[Value; STACK_MAX]>>,
    stack_top: *mut Value,
}

impl VM {
    pub fn run(
        &mut self,
        source: &str,
        stdout: &mut impl Write,
    ) -> Result<(), Vec<ErrorS>> {
        let function = Compiler::compile(source, &mut self.gc)?;
        self.run_function(function, stdout).map_err(|e| vec![e])
    }

    pub fn run_function(
        &mut self,
        function: *mut ObjectFunction,
        stdout: &mut impl Write,
    ) -> Result<()> {
        self.push(function.into());
        let mut closure = self.alloc(ObjectClosure::new(function, Vec::new()));
        self.pop();

        let mut ip = unsafe { (*(*closure).function).chunk.ops.as_ptr() };
        let mut stack = self.stack_top;

        self.frames.clear();
        self.frames.push(CallFrame {
            closure,
            ip: ptr::null(),
            stack: ptr::null_mut(),
        });

        /// Reads an instruction / byte from the current [`Chunk`].
        macro_rules! read_u8 {
            () => {{
                #[allow(unused_unsafe)]
                {
                    let byte = unsafe { *ip };
                    ip = unsafe { ip.add(1) };
                    byte
                }
            }};
        }
        /// Reads a 16-bit value from the current [`Chunk`].
        macro_rules! read_u16 {
            () => {{
                let byte1 = read_u8!();
                let byte2 = read_u8!();
                u16::from_le_bytes([byte1, byte2])
            }};
        }

        /// Reads a [`Value`] from the current [`Chunk`].
        macro_rules! read_value {
            () => {{
                #[allow(unused_unsafe)]
                {
                    let constant_idx = read_u8!() as usize;
                    let function = unsafe { (*closure).function };
                    *unsafe {
                        (*function).chunk.constants.get_unchecked(constant_idx)
                    }
                }
            }};
        }
        /// Reads an [`Object`] from the current [`Chunk`].
        macro_rules! read_object {
            () => {{
                #[allow(unused_unsafe)]
                {
                    let constant = read_value!();
                    match constant {
                        Value::Object(object) => object,
                        _ => unsafe { hint::unreachable_unchecked() },
                    }
                }
            }};
        }

        macro_rules! bail {
            ($error:expr) => {{
                #[allow(unused_unsafe)]
                {
                    let function = unsafe { (*closure).function };
                    let idx = unsafe {
                        ip.offset_from((*function).chunk.ops.as_ptr())
                    } as usize;
                    let span = unsafe { (*function).chunk.spans[idx].clone() };
                    return Err(($error.into(), span));
                }
            }};
        }

        loop {
            if cfg!(feature = "debug-trace") {
                let function = unsafe { (*closure).function };
                let idx =
                    unsafe { ip.offset_from((*function).chunk.ops.as_ptr()) };
                unsafe { (*function).chunk.debug_op(idx as usize) };
            }

            /// Binary operator that acts on any [`Value`].
            macro_rules! binary_op {
                    ($op:tt) => {{
                        let b = self.pop();
                        let a = self.pop();
                        self.push((a $op b).into());
                    }};
                }
            /// Binary operator that only acts on [`Value::Number`].
            macro_rules! binary_op_number {
                    ($op:tt) => {{
                        let b = self.pop();
                        let a = self.pop();
                        match (a, b) {
                            (Value::Number(a), Value::Number(b)) => self.push((a $op b).into()),
                            _ => bail!(TypeError::UnsupportedOperandInfix {
                                op: stringify!($op).to_string(),
                                lt_type: a.type_().to_string(),
                                rt_type: b.type_().to_string(),
                            }),
                        };
                    }};
                }

            match read_u8!() {
                op::CONSTANT => {
                    let constant = read_value!();
                    self.push(constant);
                }
                op::NIL => self.push(Value::Nil),
                op::TRUE => self.push(true.into()),
                op::FALSE => self.push(false.into()),
                op::POP => {
                    self.pop();
                }
                op::GET_LOCAL => {
                    let stack_idx = read_u8!() as usize;
                    let local = unsafe { *stack.add(stack_idx) };
                    self.push(local);
                }
                op::SET_LOCAL => {
                    let stack_idx = read_u8!() as usize;
                    let local = unsafe { stack.add(stack_idx) };
                    let value = self.peek(0);
                    unsafe { *local = *value };
                }
                op::GET_GLOBAL => {
                    let name = read_object!();
                    match self.globals.get(unsafe { &name.string }) {
                        Some(value) => self.push(*value),
                        None => {
                            bail!(NameError::NotDefined {
                                name: unsafe {
                                    (*name.string).value.to_string()
                                }
                            })
                        }
                    }
                }
                op::DEFINE_GLOBAL => {
                    let name = read_object!();
                    let value = self.pop();
                    self.globals.insert(unsafe { name.string }, value);
                }
                op::SET_GLOBAL => {
                    let name = read_object!();
                    let value = self.peek(0);
                    match self.globals.entry(unsafe { name.string }) {
                        Entry::Occupied(mut entry) => {
                            entry.insert(unsafe { *value })
                        }
                        Entry::Vacant(_) => {
                            bail!(NameError::NotDefined {
                                name: unsafe {
                                    (*name.string).value.to_string()
                                }
                            })
                        }
                    };
                }
                op::GET_UPVALUE => {
                    let upvalue_idx = read_u8!() as usize;
                    let object = *unsafe {
                        (*closure).upvalues.get_unchecked(upvalue_idx)
                    };
                    let value = unsafe { *(*object).location };
                    self.push(value);
                }
                op::SET_UPVALUE => {
                    let upvalue_idx = read_u8!() as usize;
                    let object = *unsafe {
                        (*closure).upvalues.get_unchecked(upvalue_idx)
                    };
                    let value = unsafe { (*object).location };
                    unsafe { *value = *self.peek(0) };
                }
                op::GET_PROPERTY => {
                    // TODO: Do we really need to peek here?
                    let name = unsafe { read_object!().string };
                    let instance = match unsafe { *self.peek(0) } {
                        Value::Object(object)
                            if unsafe {
                                (*object.common).type_ == ObjectType::Instance
                            } =>
                        unsafe { object.instance },
                        value => bail!(AttributeError::NoSuchAttribute {
                            type_: value.type_().to_string(),
                            name: unsafe { (*name).value.to_string() },
                        }),
                    };

                    match unsafe { (*instance).fields.get(&name) } {
                        Some(value) => {
                            // Pop the instance from the stack.
                            self.pop();
                            self.push(*value);
                        }
                        None => bail!(AttributeError::NoSuchAttribute {
                            type_: unsafe {
                                (*(*(*instance).class).name).value.to_string()
                            },
                            name: unsafe { (*name).value.to_string() },
                        }),
                    }
                }
                op::SET_PROPERTY => {
                    // TODO: Do we really need to peek here?
                    let name = unsafe { read_object!().string };
                    let instance = match unsafe { *self.peek(0) } {
                        Value::Object(object)
                            if unsafe {
                                (*object.common).type_ == ObjectType::Instance
                            } =>
                        unsafe { object.instance },
                        value => bail!(AttributeError::NoSuchAttribute {
                            type_: value.type_().to_string(),
                            name: unsafe { (*name).value.to_string() },
                        }),
                    };
                    let value = unsafe { *self.peek(1) };
                    unsafe { (*instance).fields.insert(name, value) };

                    // Pop the instance.
                    self.pop();
                }
                op::EQUAL => binary_op!(==),
                op::NOT_EQUAL => binary_op!(!=),
                op::GREATER => binary_op_number!(>),
                op::GREATER_EQUAL => binary_op_number!(>=),
                op::LESS => binary_op_number!(<),
                op::LESS_EQUAL => binary_op_number!(<=),
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
                            match unsafe {
                                ((*o1.common).type_, (*o2.common).type_)
                            } {
                                (ObjectType::String, ObjectType::String) => {
                                    let string = unsafe {
                                        [(*o1.string).value, (*o2.string).value]
                                    }
                                    .concat();
                                    let string = self.alloc(string);
                                    self.pop();
                                    self.pop();
                                    self.push(string.into());
                                }
                                _ => {
                                    bail!(TypeError::UnsupportedOperandInfix {
                                        op: "+".to_string(),
                                        lt_type: a.type_().to_string(),
                                        rt_type: b.type_().to_string(),
                                    })
                                }
                            };
                        }
                        _ => bail!(TypeError::UnsupportedOperandInfix {
                            op: "+".to_string(),
                            lt_type: a.type_().to_string(),
                            rt_type: b.type_().to_string(),
                        }),
                    }
                }
                op::SUBTRACT => binary_op_number!(-),
                op::MULTIPLY => binary_op_number!(*),
                op::DIVIDE => binary_op_number!(/),
                op::NOT => {
                    let value = self.pop();
                    self.push(!value);
                }
                op::NEGATE => {
                    let value = self.pop();
                    match value {
                        Value::Number(number) => self.push((-number).into()),
                        _ => bail!(TypeError::UnsupportedOperandPrefix {
                            op: "-".to_string(),
                            rt_type: value.type_().to_string(),
                        }),
                    }
                }
                op::PRINT => {
                    let value = self.pop();
                    if writeln!(stdout, "{value}").is_err() {
                        bail!(IoError::WriteError {
                            file: "stdout".to_string()
                        });
                    };
                }
                op::JUMP => {
                    let offset = read_u16!() as usize;
                    ip = unsafe { ip.add(offset) };
                }
                op::JUMP_IF_FALSE => {
                    let offset = read_u16!() as usize;
                    let value = self.peek(0);
                    if !(unsafe { *value }.bool()) {
                        ip = unsafe { ip.add(offset) };
                    }
                }
                op::LOOP => {
                    let offset = read_u16!() as usize;
                    ip = unsafe { ip.sub(offset) };
                }
                op::CALL => {
                    let arg_count = read_u8!();
                    let callee = unsafe { *self.peek(arg_count as usize) };
                    match callee {
                        Value::Object(object) => match unsafe {
                            (*object.common).type_
                        } {
                            ObjectType::Class => {
                                let instance =
                                    self.alloc(ObjectInstance::new(unsafe {
                                        object.class
                                    }));
                                self.push(instance.into());
                            }
                            ObjectType::Closure => {
                                // Save the current state of the VM.
                                let frame = unsafe {
                                    self.frames.last_mut().unwrap_unchecked()
                                };
                                frame.ip = ip;
                                frame.stack = stack;

                                // Check if the function arity is correct.
                                let function =
                                    unsafe { (*object.closure).function };
                                if arg_count != unsafe { (*function).arity } {
                                    bail!(TypeError::ArityMismatch {
                                        name: unsafe {
                                            (*(*function).name).value
                                        }
                                        .to_string(),
                                        exp_args: unsafe { (*function).arity },
                                        got_args: arg_count,
                                    });
                                }

                                // Set up the current closure.
                                closure = unsafe { object.closure };
                                ip = unsafe { (*function).chunk.ops.as_ptr() };
                                stack = self.peek(arg_count as usize);

                                // Set up the current frame.
                                if self.frames.len() >= self.frames.capacity() {
                                    bail!(OverflowError::StackOverflow);
                                }
                                let frame = CallFrame {
                                    closure,
                                    ip: ptr::null(),
                                    stack: ptr::null_mut(),
                                };
                                unsafe { self.frames.push_unchecked(frame) };
                            }
                            _ => {
                                bail!(TypeError::NotCallable {
                                    type_: callee.type_().to_string()
                                })
                            }
                        },
                        Value::Native(native) => {
                            self.pop();
                            let value = match native {
                                Native::Clock => {
                                    if arg_count != 0 {
                                        bail!(TypeError::ArityMismatch {
                                            name: "clock".to_string(),
                                            exp_args: 0,
                                            got_args: arg_count,
                                        });
                                    }
                                    SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs_f64()
                                        .into()
                                }
                            };
                            self.push(value);
                        }
                        _ => {
                            bail!(TypeError::NotCallable {
                                type_: callee.type_().to_string()
                            })
                        }
                    }
                }
                op::CLOSURE => {
                    let function = unsafe { read_object!().function };

                    let upvalue_count =
                        unsafe { (*function).upvalues } as usize;
                    let mut upvalues = Vec::with_capacity(upvalue_count);

                    for _ in 0..upvalue_count {
                        let is_local = read_u8!();
                        let upvalue_idx = read_u8!();

                        let upvalue = if is_local != 0 {
                            let location =
                                unsafe { stack.add(upvalue_idx as usize) };
                            println!("capture location: {}", unsafe {
                                location.offset_from(stack)
                            });
                            self.capture_upvalue(location)
                        } else {
                            unsafe {
                                *(*closure)
                                    .upvalues
                                    .get_unchecked(upvalue_idx as usize)
                            }
                        };
                        upvalues.push(upvalue);
                    }

                    let closure =
                        self.alloc(ObjectClosure::new(function, upvalues));
                    self.push(closure.into());
                }
                op::CLOSE_UPVALUE => {
                    let last = self.peek(0);
                    self.close_upvalues(last);
                    self.pop();
                }
                op::RETURN => {
                    let value = self.pop();
                    self.close_upvalues(stack);

                    self.stack_top = stack;
                    unsafe { self.frames.pop().unwrap_unchecked() };

                    match self.frames.last_mut() {
                        Some(frame) => {
                            closure = frame.closure;
                            ip = frame.ip;
                            stack = frame.stack;
                        }
                        None => break,
                    }
                    self.push(value);
                }
                op::CLASS => {
                    let name = unsafe { read_object!().string };
                    let class = self.alloc(ObjectClass::new(name)).into();
                    self.push(class);
                }
                _ => unsafe { hint::unreachable_unchecked() },
            }

            if cfg!(feature = "debug-trace") {
                eprint!("     ");
                let mut stack_ptr = stack;
                while stack_ptr < self.stack_top {
                    eprint!("[ {} ]", unsafe { *stack_ptr });
                    stack_ptr = unsafe { stack_ptr.add(1) };
                }
                eprintln!();
            }
        }

        debug_assert!(
            stack == self.stack_top,
            "VM finished executing but stack is not empty"
        );
        Ok(())
    }

    fn alloc<T>(&mut self, object: impl GcAlloc<T>) -> T {
        if !cfg!(feature = "disable-gc")
            && (cfg!(feature = "stress-gc")
                || GLOBAL.allocated_bytes() > self.next_gc)
        {
            self.gc();
        }
        self.gc.alloc(object)
    }

    fn gc(&mut self) {
        if cfg!(feature = "debug-gc") {
            println!("-- gc begin");
        }

        let mut stack_ptr = self.stack.as_ptr();
        while stack_ptr < self.stack_top {
            self.gc.mark(unsafe { *stack_ptr });
            stack_ptr = unsafe { stack_ptr.add(1) };
        }

        for (&name, &value) in &self.globals {
            self.gc.mark(name);
            self.gc.mark(value);
        }

        for frame in &self.frames {
            self.gc.mark(frame.closure);
        }

        for &upvalue in &self.open_upvalues {
            self.gc.mark(upvalue);
        }

        self.gc.trace();
        self.gc.sweep();

        self.next_gc = GLOBAL.allocated_bytes() * GC_HEAP_GROW_FACTOR;

        if cfg!(feature = "debug-gc") {
            println!("-- gc end");
        }
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
        match self
            .open_upvalues
            .iter()
            .find(|&&upvalue| unsafe { (*upvalue).location } == location)
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
}

impl Default for VM {
    fn default() -> Self {
        // TODO: tune these later.
        let mut gc = Gc::default();

        let mut globals = HashMap::with_capacity_and_hasher(
            256,
            BuildHasherDefault::default(),
        );
        let clock = gc.alloc("clock");
        globals.insert(clock, Native::Clock.into());

        let mut stack = Box::pin([Value::default(); STACK_MAX]);
        let stack_top = stack.as_mut_ptr();

        Self {
            globals,
            open_upvalues: Vec::with_capacity(256),
            gc,
            next_gc: 1024 * 1024,
            frames: ArrayVec::new(),
            stack,
            stack_top,
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
