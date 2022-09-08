use crate::chunk::Chunk;
use crate::intern::Intern;
use crate::op;
use crate::value::{Object, ObjectType, Value};
use std::hint;

const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * STACK_MAX_PER_FRAME;
const STACK_MAX_PER_FRAME: usize = u8::MAX as usize + 1;

pub struct VM {
    pub objects: Vec<*mut Object>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            objects: Vec::with_capacity(256), // TODO: tune this later.
        }
    }

    pub fn run(&mut self, chunk: &Chunk, intern: &mut Intern) {
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

        /// Reads a constant from the current [`Chunk`].
        macro_rules! read_constant {
            () => {{
                let constant_idx = read_u8!() as usize;
                *unsafe { chunk.constants.get_unchecked(constant_idx) }
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
        let mut stack_top = stack.as_mut_ptr();

        /// Pushes a value to the stack.
        macro_rules! push {
            ($value:expr) => {{
                let value = $value;
                unsafe { *stack_top = value };
                stack_top = unsafe { stack_top.add(1) };
            }};
        }

        /// Pops a value from the stack.
        macro_rules! pop {
            () => {{
                stack_top = unsafe { stack_top.sub(1) };
                unsafe { *stack_top }
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
                    _ => panic!("unsupported operand for type: {}", stringify!($op)),
                };
            }};
        }

        loop {
            match read_u8!() {
                op::CONSTANT => {
                    let constant = read_constant!();
                    push!(constant);
                }
                op::NIL => push!(Value::Nil),
                op::TRUE => push!(true.into()),
                op::FALSE => push!(false.into()),
                op::POP => {
                    pop!();
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
                                (ObjectType::String(a), ObjectType::String(b)) => {
                                    let mut string = String::with_capacity(a.len() + b.len());
                                    string.push_str(a);
                                    string.push_str(b);
                                    let (object, inserted) = intern.insert_string(string);
                                    if inserted {
                                        self.objects.push(object);
                                    };
                                    push!(object.into());
                                }
                            };
                        }
                        _ => panic!(),
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
                        _ => panic!(),
                    }
                }
                // TODO: parametrize output using `writeln!`.
                op::PRINT => {
                    let value = pop!();
                    println!("{}", value);
                }
                op::RETURN => break,
                _ => unsafe { hint::unreachable_unchecked() },
            }
        }
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
