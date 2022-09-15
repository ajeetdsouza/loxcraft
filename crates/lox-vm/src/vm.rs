use std::hash::BuildHasherDefault;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{hint, io};

use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use lox_common::error::{ErrorS, IoError, NameError, OverflowError, Result, TypeError};
use rustc_hash::FxHasher;

use crate::compiler::Compiler;
use crate::intern::Intern;
use crate::op;
use crate::value::{Function, Native, Object, ObjectExt, ObjectType, Value};

const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * STACK_MAX_PER_FRAME;
const STACK_MAX_PER_FRAME: usize = u8::MAX as usize + 1;

pub struct VM {
    pub globals: HashMap<*mut Object, Value, BuildHasherDefault<FxHasher>>,
    pub objects: Vec<*mut Object>,
    pub intern: Intern,
}

impl VM {
    pub fn new() -> Self {
        // TODO: tune these later.
        let mut globals = HashMap::with_capacity_and_hasher(256, Default::default());
        let mut intern = Intern::default();

        let (clock, _) = intern.insert_str("clock");
        globals.insert(clock, Native::Clock.into());

        Self { globals, objects: Vec::with_capacity(256), intern }
    }

    pub fn run<W: io::Write>(&mut self, source: &str, stdout: &mut W) -> Result<(), Vec<ErrorS>> {
        let function = Compiler::compile(source, &mut self.intern)?;
        if let Err(e) = self.run_function(&function, stdout) {
            return Err(vec![e]);
        }
        Ok(())
    }

    pub fn run_function<W: io::Write>(&mut self, mut function: &Function, stdout: &mut W) -> Result<()> {
        let mut frames: Vec<CallFrame> = Vec::with_capacity(256);

        // Instruction pointer for the current Chunk.
        // Accessing `ip` without bounds checking is safe, assuming that the
        // compiler always outputs correct code. The program stops execution
        // when it reaches `op::RETURN`.
        let mut ip = function.chunk.ops.as_ptr();
        /// Reads an instruction / byte from the current [`Chunk`].
        macro_rules! read_u8 {
            () => {{
                let byte = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                byte
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

        // Accessing `stack` without bounds checking is safe because:
        // - Each frame can store a theoretical maximum of `STACK_MAX_PER_FRAME` values on the stack.
        // - The frame count can never exceed `MAX_FRAMES`, otherwise we throw a stack overflow error.
        // - Thus, we can statically allocate a stack of size `STACK_MAX = FRAMES_MAX * STACK_MAX_PER_FRAME`
        //   and we are guaranteed to never exceed this.
        let mut stack: [Value; STACK_MAX] = [Default::default(); STACK_MAX];
        let mut stack = stack.as_mut_ptr();
        let mut stack_top = stack;
        /// Pushes a value to the stack.
        macro_rules! push {
            ($value:expr) => {{
                let value = $value;
                unsafe { *stack_top = value };
                stack_top = unsafe { stack_top.add(1) };
            }};
        }
        /// Pops a [`Value`] from the stack.
        macro_rules! pop {
            () => {{
                stack_top = unsafe { stack_top.sub(1) };
                debug_assert!(stack_top >= stack);
                unsafe { *stack_top }
            }};
        }
        /// Peeks at a [`Value`] from the stack.
        macro_rules! peek {
            () => {{ unsafe { stack_top.sub(1) } }};
            ($n:expr) => {{
                #[allow(unused_unsafe)]
                unsafe {
                    stack_top.sub(($n) + 1)
                }
            }};
        }

        /// Reads a [`Value`] from the current [`Chunk`].
        macro_rules! read_value {
            () => {{
                let constant_idx = read_u8!() as usize;
                *unsafe { function.chunk.constants.get_unchecked(constant_idx) }
            }};
        }
        /// Reads an [`Object`] from the current [`Chunk`].
        macro_rules! read_object {
            () => {{
                let constant_idx = read_u8!() as usize;
                let constant = *unsafe { function.chunk.constants.get_unchecked(constant_idx) };
                match constant {
                    Value::Object(object) => object,
                    _ => unsafe { hint::unreachable_unchecked() },
                }
            }};
        }

        macro_rules! bail {
            ($error:expr) => {{
                let idx = unsafe { ip.offset_from(function.chunk.ops.as_ptr()) } as usize;
                let span = &function.chunk.spans[idx];
                return Err(($error.into(), span.clone()));
            }};
        }

        loop {
            if cfg!(feature = "debug-trace") {
                let idx = unsafe { ip.offset_from(function.chunk.ops.as_ptr()) };
                function.chunk.debug_op(idx as usize);
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
                    let local = unsafe { *stack.add(stack_idx) };
                    push!(local);
                }
                op::SET_LOCAL => {
                    let stack_idx = read_u8!() as usize;
                    let local = unsafe { stack.add(stack_idx) };
                    let value = peek!();
                    unsafe { *local = *value };
                }
                op::GET_GLOBAL => {
                    let name = read_object!();
                    match self.globals.get(&name) {
                        Some(value) => push!(*value),
                        None => {
                            bail!(NameError::NotDefined { name: name.to_str().to_string() })
                        }
                    }
                }
                op::DEFINE_GLOBAL => {
                    let name = read_object!();
                    let value = pop!();
                    self.globals.insert(name, value);
                }
                op::SET_GLOBAL => {
                    let name = read_object!();
                    let value = peek!();
                    match self.globals.entry(name) {
                        Entry::Occupied(mut entry) => entry.insert(unsafe { *value }),
                        Entry::Vacant(_) => {
                            bail!(NameError::NotDefined { name: name.to_str().to_string() })
                        }
                    };
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
                        (Value::Number(n1), Value::Number(n2)) => push!((n1 + n2).into()),
                        (Value::Object(o1), Value::Object(o2)) => {
                            match unsafe { (&(*o1).type_, &(*o2).type_) } {
                                (&ObjectType::String(a), &ObjectType::String(b)) => {
                                    let string = [a, b].concat();
                                    let (object, inserted) = self.intern.insert_string(string);
                                    if inserted {
                                        // TODO: Add this back once we have a GC.
                                        // self.objects.push(object);
                                    };
                                    push!(object.into());
                                }
                                _ => bail!(TypeError::UnsupportedOperandInfix {
                                    op: "+".to_string(),
                                    lt_type: a.type_().to_string(),
                                    rt_type: b.type_().to_string(),
                                }),
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
                // TODO: parametrize output using `writeln!`.
                op::PRINT => {
                    let value = pop!();
                    if let Err(_) = writeln!(stdout, "{value}") {
                        bail!(IoError::WriteError { file: "stdout".to_string() });
                    };
                }
                op::JUMP => {
                    let offset = read_u16!() as usize;
                    ip = unsafe { ip.add(offset) };
                }
                op::JUMP_IF_FALSE => {
                    let offset = read_u16!() as usize;
                    let value = peek!();
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
                    let callee = unsafe { *peek!(arg_count as usize) };
                    match callee {
                        Value::Object(object) => match unsafe { &(*object).type_ } {
                            ObjectType::Function(callee) => {
                                if arg_count != callee.arity {
                                    bail!(TypeError::ArityMismatch {
                                        name: callee.name.to_str().to_string(),
                                        exp_args: callee.arity,
                                        got_args: arg_count,
                                    });
                                }

                                frames.push(CallFrame { function, ip, stack });
                                if frames.len() > FRAMES_MAX {
                                    bail!(OverflowError::StackOverflow);
                                }

                                function = callee;
                                ip = function.chunk.ops.as_ptr();
                                stack = peek!(arg_count as usize);
                            }
                            _ => {
                                bail!(TypeError::NotCallable { type_: callee.type_().to_string() })
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
                            bail!(TypeError::NotCallable { type_: callee.type_().to_string() })
                        }
                    }
                }
                op::RETURN => {
                    let value = pop!();
                    match frames.pop() {
                        Some(frame) => {
                            stack_top = stack;

                            function = frame.function;
                            ip = frame.ip;
                            stack = frame.stack;

                            push!(value);
                        }
                        None => break,
                    };
                }
                _ => unsafe { hint::unreachable_unchecked() },
            }

            if cfg!(feature = "debug-trace") {
                eprint!("     ");
                let mut stack_ptr = stack;
                while stack_ptr < stack_top {
                    eprint!("[ {} ]", unsafe { *stack_ptr });
                    stack_ptr = unsafe { stack_ptr.add(1) };
                }
                eprintln!();
            }
        }

        debug_assert!(stack == stack_top);
        Ok(())
    }
}

impl Drop for VM {
    fn drop(&mut self) {
        for &object in &self.objects {
            unsafe {
                let _ = Box::from_raw(object);
            }
        }
    }
}

pub struct CallFrame<'a> {
    function: &'a Function,
    ip: *const u8,
    stack: *mut Value,
}
