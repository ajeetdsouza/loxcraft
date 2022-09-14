use crate::chunk::Chunk;
use crate::compiler::Compiler;
use crate::intern::Intern;
use crate::op;
use crate::value::{Object, ObjectType, Value};
use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use lox_common::error::{ErrorS, IoError, NameError, Result, TypeError};
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;
use std::hint;
use std::io::{self, Write};

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
        Self {
            // TODO: tune these later.
            globals: HashMap::with_capacity_and_hasher(256, Default::default()),
            objects: Vec::with_capacity(256),
            intern: Intern::default(),
        }
    }

    pub fn run(&mut self, source: &str) -> Result<(), Vec<ErrorS>> {
        let chunk = Compiler::compile(source, &mut self.intern)?;
        if let Err(e) = self.run_chunk(&chunk) {
            return Err(vec![e]);
        }
        Ok(())
    }

    pub fn run_chunk(&mut self, chunk: &Chunk) -> Result<()> {
        // Output stream. This is used for printing as well as debug logs.
        let out = &mut io::stdout().lock();

        // Instruction pointer for the current Chunk.
        // Accessing `ip` without bounds checking is safe, assuming that the
        // compiler always outputs correct code. The program stops execution
        // when it reaches `op::RETURN`.
        let mut ip = chunk.ops.as_ptr();

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

        /// Reads a [`Value`] from the current [`Chunk`].
        macro_rules! read_value {
            () => {{
                let constant_idx = read_u8!() as usize;
                *unsafe { chunk.constants.get_unchecked(constant_idx) }
            }};
        }

        /// Reads an [`Object`] from the current [`Chunk`].
        macro_rules! read_object {
            () => {{
                let constant_idx = read_u8!() as usize;
                let constant = *unsafe { chunk.constants.get_unchecked(constant_idx) };
                match constant {
                    Value::Object(object) => object,
                    _ => unsafe { hint::unreachable_unchecked() },
                }
            }};
        }

        // Accessing `stack` without bounds checking is safe because:
        // - Each frame can store a theoretical maximum of `STACK_MAX_PER_FRAME`
        //   values on the stack.
        // - The frame count can never exceed `MAX_FRAMES`, otherwise we throw a
        //   stack overflow error.
        // - Thus, we can statically allocate a stack of size
        //   `STACK_MAX = FRAMES_MAX * STACK_MAX_PER_FRAME` and we are
        //   guaranteed to never exceed this.
        let mut stack = [Value::default(); STACK_MAX];
        let stack = stack.as_mut_ptr();
        let mut stack_top = stack;

        macro_rules! bail {
            ($error:expr) => {{
                let idx = unsafe { ip.offset_from(chunk.ops.as_ptr()) } as usize;
                let span = &chunk.spans[idx];
                return Err(($error.into(), span.clone()));
            }};
        }

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
                unsafe { *stack_top }
            }};
        }

        /// Peeks at a [`Value`] from the stack.
        macro_rules! peek {
            () => {{
                unsafe { stack_top.sub(1) }
            }};
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

        loop {
            if cfg!(feature = "debug-trace") {
                let idx = unsafe { ip.offset_from(chunk.ops.as_ptr()) };
                chunk.debug_op(idx as usize);
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
                            bail!(NameError::NotDefined {
                                name: self.intern.get_string(name).unwrap()
                            })
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
                            bail!(NameError::NotDefined {
                                name: self.intern.get_string(name).unwrap()
                            })
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
                        (Value::Number(a), Value::Number(b)) => push!((a + b).into()),
                        (Value::Object(a), Value::Object(b)) => {
                            match unsafe { (&(*a).type_, &(*b).type_) } {
                                (&ObjectType::String(a), &ObjectType::String(b)) => {
                                    let string = [a, b].concat();
                                    let (object, inserted) = self.intern.insert_string(string);
                                    if inserted {
                                        // TODO: Add this back once we have a GC.
                                        // self.objects.push(object);
                                    };
                                    push!(object.into());
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
                            op: "+".to_string(),
                            rt_type: value.type_().to_string(),
                        }),
                    }
                }
                // TODO: parametrize output using `writeln!`.
                op::PRINT => {
                    let value = pop!();
                    if let Err(_) = writeln!(out, "{value}") {
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
                op::RETURN => break,
                _ => unsafe { hint::unreachable_unchecked() },
            }

            if cfg!(feature = "debug-trace") {
                write!(out, "     ").unwrap();
                let mut stack_ptr = stack;
                while stack_ptr < stack_top {
                    write!(out, "[ {} ]", unsafe { *stack_ptr }).unwrap();
                    stack_ptr = unsafe { stack_ptr.add(1) };
                }
                writeln!(out).unwrap();
            }
        }
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
