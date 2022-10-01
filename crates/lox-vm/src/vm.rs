use std::hash::BuildHasherDefault;
use std::hint;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use lox_common::error::{
    ErrorS, IoError, NameError, OverflowError, Result, TypeError,
};
use rustc_hash::FxHasher;

use crate::compiler::Compiler;
use crate::gc::{Gc, GcAlloc};
use crate::object::{
    ObjectClass, ObjectClosure, ObjectFunction, ObjectString, ObjectType,
    ObjectUpvalue,
};
use crate::op;
use crate::value::{Native, Value};

const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * STACK_MAX_PER_FRAME;
const STACK_MAX_PER_FRAME: usize = u8::MAX as usize + 1;

pub struct VM {
    pub globals:
        HashMap<*mut ObjectString, Value, BuildHasherDefault<FxHasher>>,
    pub open_upvalues: Vec<*mut ObjectUpvalue>,
    pub gc: Gc,
}

impl VM {
    pub fn run<W: Write>(
        &mut self,
        source: &str,
        stdout: &mut W,
    ) -> Result<(), Vec<ErrorS>> {
        let function = Compiler::compile(source, &mut self.gc)?;
        self.run_function(function, stdout).map_err(|e| vec![e])
    }

    pub fn run_function<W: Write>(
        &mut self,
        function: *mut ObjectFunction,
        stdout: &mut W,
    ) -> Result<()> {
        let closure = self.gc.alloc(ObjectClosure::new(function, Vec::new()));
        let ip = unsafe { (*(*closure).function).chunk.ops.as_ptr() };

        // Accessing `stack` without bounds checking is safe because:
        // - Each frame can store a theoretical maximum of `STACK_MAX_PER_FRAME`
        //   values on the stack.
        // - The frame count can never exceed `MAX_FRAMES`, otherwise we throw a
        //   stack overflow error.
        // - Thus, we can statically allocate a stack of size
        //   `STACK_MAX = FRAMES_MAX * STACK_MAX_PER_FRAME` and we are
        //   guaranteed to never exceed this size.
        let mut stack: [Value; STACK_MAX] = [Default::default(); STACK_MAX];
        let stack = stack.as_mut_ptr();
        let mut stack_top = stack;

        let mut frames: Vec<CallFrame> = Vec::with_capacity(256);
        let mut frame = CallFrame {
            closure,
            // Instruction pointer for the current Chunk.
            // Accessing `ip` without bounds checking is safe, assuming that the
            // compiler always outputs correct code. The program stops execution
            // when it reaches `op::RETURN`.
            ip,
            stack,
        };

        /// Reads an instruction / byte from the current [`Chunk`].
        macro_rules! read_u8 {
            () => {{
                #[allow(unused_unsafe)]
                {
                    let byte = unsafe { *frame.ip };
                    frame.ip = unsafe { frame.ip.add(1) };
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

        /// Pushes a value to the stack.
        macro_rules! push {
            ($value:expr) => {{
                #[allow(unused_unsafe)]
                {
                    let value = $value;
                    unsafe { *stack_top = value };
                    stack_top = unsafe { stack_top.add(1) };
                }
            }};
        }
        /// Pops a [`Value`] from the stack.
        macro_rules! pop {
            () => {{
                #[allow(unused_unsafe)]
                {
                    stack_top = unsafe { stack_top.sub(1) };
                    debug_assert!(stack_top >= frame.stack);
                    unsafe { *stack_top }
                }
            }};
        }
        /// Peeks at a [`Value`] from the stack.
        macro_rules! peek {
            () => {{
                #[allow(unused_unsafe)]
                {
                    let stack_ptr = unsafe { stack_top.sub(1) };
                    debug_assert!(stack_ptr >= frame.stack);
                    stack_ptr
                }
            }};
            ($n:expr) => {{
                #[allow(unused_unsafe)]
                {
                    unsafe { stack_top.sub($n + 1) }
                }
            }};
        }

        /// Reads a [`Value`] from the current [`Chunk`].
        macro_rules! read_value {
            () => {{
                #[allow(unused_unsafe)]
                {
                    let function = unsafe { (*frame.closure).function };
                    let constant_idx = read_u8!() as usize;
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
                    let function = unsafe { (*frame.closure).function };
                    let constant_idx = read_u8!() as usize;
                    let constant = *unsafe {
                        (*function).chunk.constants.get_unchecked(constant_idx)
                    };
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
                    let function = unsafe { (*frame.closure).function };
                    let idx = unsafe {
                        frame.ip.offset_from((*function).chunk.ops.as_ptr())
                    } as usize;
                    let span = unsafe { (*function).chunk.spans[idx].clone() };
                    return Err(($error.into(), span));
                }
            }};
        }

        loop {
            if cfg!(feature = "debug-trace") {
                let function = unsafe { (*frame.closure).function };
                let idx = unsafe {
                    frame.ip.offset_from((*function).chunk.ops.as_ptr())
                };
                unsafe { (*function).chunk.debug_op(idx as usize) };
            }

            /// Binary operator that acts on any [`Value`].
            macro_rules! binary_op {
                    ($op:tt) => {{
                        let b = pop!();
                        let a = pop!();
                        push!((a $op b).into());
                    }};
                }
            /// Binary operator that only acts on [`Value::Number`].
            macro_rules! binary_op_number {
                    ($op:tt) => {{
                        let b = pop!();
                        let a = pop!();
                        match (a, b) {
                            (Value::Number(a), Value::Number(b)) => push!((a $op b).into()),
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
                    push!(constant);
                }
                op::NIL => push!(Value::Nil),
                op::TRUE => push!(true.into()),
                op::FALSE => push!(false.into()),
                op::POP => {
                    pop!();
                }
                op::GET_LOCAL => {
                    let stack_idx = read_u8!() as usize;
                    let local = unsafe { *frame.stack.add(stack_idx) };
                    push!(local);
                }
                op::SET_LOCAL => {
                    let stack_idx = read_u8!() as usize;
                    let local = unsafe { frame.stack.add(stack_idx) };
                    let value = peek!();
                    unsafe { *local = *value };
                }
                op::GET_GLOBAL => {
                    let name = read_object!();
                    match self.globals.get(unsafe { &name.string }) {
                        Some(value) => push!(*value),
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
                    let value = pop!();
                    self.globals.insert(unsafe { name.string }, value);
                }
                op::SET_GLOBAL => {
                    let name = read_object!();
                    let value = peek!();
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
                        (*frame.closure).upvalues.get_unchecked(upvalue_idx)
                    };
                    let value = unsafe { *(*object).location };
                    push!(value);
                }
                op::SET_UPVALUE => {
                    let upvalue_idx = read_u8!() as usize;
                    let object = *unsafe {
                        (*frame.closure).upvalues.get_unchecked(upvalue_idx)
                    };
                    let value = unsafe { (*object).location };
                    unsafe { *value = *peek!() };
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
                    let b = pop!();
                    let a = pop!();
                    match (a, b) {
                        (Value::Number(n1), Value::Number(n2)) => {
                            push!((n1 + n2).into())
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
                                    let string = self.gc.alloc(string);
                                    push!(string.into());
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
                    let value = pop!();
                    push!(!value);
                }
                op::NEGATE => {
                    let value = pop!();
                    match value {
                        Value::Number(number) => push!((-number).into()),
                        _ => bail!(TypeError::UnsupportedOperandPrefix {
                            op: "-".to_string(),
                            rt_type: value.type_().to_string(),
                        }),
                    }
                }
                op::PRINT => {
                    let value = pop!();
                    if writeln!(stdout, "{value}").is_err() {
                        bail!(IoError::WriteError {
                            file: "stdout".to_string()
                        });
                    };
                }
                op::JUMP => {
                    let offset = read_u16!() as usize;
                    frame.ip = unsafe { frame.ip.add(offset) };
                }
                op::JUMP_IF_FALSE => {
                    let offset = read_u16!() as usize;
                    let value = peek!();
                    if !(unsafe { *value }.bool()) {
                        frame.ip = unsafe { frame.ip.add(offset) };
                    }
                }
                op::LOOP => {
                    let offset = read_u16!() as usize;
                    frame.ip = unsafe { frame.ip.sub(offset) };
                }
                op::CALL => {
                    let arg_count = read_u8!();
                    let callee = unsafe { *peek!(arg_count as usize) };
                    match callee {
                        Value::Object(object) => match unsafe {
                            (*object.common).type_
                        } {
                            ObjectType::Closure => {
                                let closure = unsafe { object.closure };
                                let function = unsafe { (*closure).function };
                                if arg_count != unsafe { (*function).arity } {
                                    bail!(TypeError::ArityMismatch {
                                        name: unsafe {
                                            (*(*function).name)
                                                .value
                                                .to_string()
                                        },
                                        exp_args: unsafe { (*function).arity },
                                        got_args: arg_count,
                                    });
                                }

                                if frames.len() >= FRAMES_MAX {
                                    bail!(OverflowError::StackOverflow);
                                }
                                frames.push(frame);
                                frame = CallFrame {
                                    closure,
                                    ip: unsafe {
                                        (*function).chunk.ops.as_ptr()
                                    },
                                    stack: peek!(arg_count as usize),
                                };
                            }
                            _ => {
                                bail!(TypeError::NotCallable {
                                    type_: callee.type_().to_string()
                                })
                            }
                        },
                        Value::Native(native) => {
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
                            push!(value);
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
                            let location = unsafe {
                                frame.stack.add(upvalue_idx as usize)
                            };
                            println!("capture location: {}", unsafe {
                                location.offset_from(stack)
                            });
                            self.capture_upvalue(location)
                        } else {
                            unsafe {
                                *(*frame.closure)
                                    .upvalues
                                    .get_unchecked(upvalue_idx as usize)
                            }
                        };
                        upvalues.push(upvalue);
                    }

                    let closure =
                        self.gc.alloc(ObjectClosure::new(function, upvalues));
                    push!(closure.into());
                }
                op::CLOSE_UPVALUE => {
                    let last = peek!();
                    self.close_upvalues(last);
                    pop!();
                }
                op::RETURN => {
                    let value = pop!();
                    self.close_upvalues(frame.stack);
                    match frames.pop() {
                        Some(prev_frame) => {
                            stack_top = frame.stack;
                            frame = prev_frame;
                            push!(value);
                        }
                        None => break,
                    };
                }
                op::CLASS => {
                    let name = unsafe { read_object!().string };
                    let class = self.gc.alloc(ObjectClass::new(name)).into();
                    push!(class);
                }
                _ => unsafe { hint::unreachable_unchecked() },
            }

            if cfg!(feature = "debug-trace") {
                eprint!("     ");
                let mut stack_ptr = frame.stack;
                while stack_ptr < stack_top {
                    eprint!("[ {} ]", unsafe { *stack_ptr });
                    stack_ptr = unsafe { stack_ptr.add(1) };
                }
                eprintln!();
            }
        }

        debug_assert!(
            frame.stack == stack_top,
            "VM finished executing but stack is not empty"
        );
        Ok(())
    }

    fn capture_upvalue(&mut self, location: *mut Value) -> *mut ObjectUpvalue {
        match self
            .open_upvalues
            .iter()
            .find(|&&upvalue| unsafe { (*upvalue).location } == location)
        {
            Some(&upvalue) => upvalue,
            None => {
                let upvalue = self.gc.alloc(ObjectUpvalue::new(location));
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
        let mut globals =
            HashMap::with_capacity_and_hasher(256, Default::default());
        let mut gc = Gc::default();

        let clock = gc.alloc("clock");
        globals.insert(clock, Native::Clock.into());

        Self { globals, open_upvalues: Vec::with_capacity(256), gc }
    }
}

pub struct CallFrame {
    closure: *mut ObjectClosure,
    ip: *const u8,
    stack: *mut Value,
}
