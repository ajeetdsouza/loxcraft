mod allocator;
mod chunk;
mod compiler;
mod gc;
mod object;
mod op;
mod util;
mod value;
pub mod vecmap;

use std::hash::BuildHasherDefault;
use std::io::Write;
use std::{mem, ptr};

use arrayvec::ArrayVec;
pub use compiler::Compiler;
pub use gc::Gc;
use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use rustc_hash::FxHasher;

use crate::error::{
    AttributeError, Error, ErrorS, IoError, NameError, OverflowError, Result, TypeError,
};
use crate::syntax::ast::{OpInfix, OpPrefix};
use crate::vm::allocator::GLOBAL;
use crate::vm::gc::GcAlloc;
use crate::vm::object::{
    Native, ObjectBoundMethod, ObjectClass, ObjectClosure, ObjectFunction, ObjectInstance,
    ObjectNative, ObjectString, ObjectType, ObjectUpvalue,
};
use crate::vm::value::Value;

macro_rules! binary_op_number {
    ($self:ident, $ip:expr, $op:tt, $op_infix:expr) => {{
        let b = $self.pop();
        let a_ptr = $self.peek(0);
        let a = unsafe { *a_ptr };
        if a.is_number() && b.is_number() {
            unsafe { *a_ptr = Value::from(a.as_number() $op b.as_number()) };
            $self.dispatch($ip)
        } else {
            $self.err(*$ip, TypeError::UnsupportedOperandInfix {
                op: $op_infix,
                lt_type: a.type_().to_string(),
                rt_type: b.type_().to_string(),
            })
        }
    }};
}

const GC_HEAP_GROW_FACTOR: usize = 2;
const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * STACK_MAX_PER_FRAME;
const STACK_MAX_PER_FRAME: usize = u8::MAX as usize + 1;

pub struct VM<'a> {
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

    stdout: &'a mut dyn Write,
}

impl<'a> VM<'a> {
    pub fn new(stdout: &'a mut dyn Write) -> Self {
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
            stdout,
        }
    }

    pub fn run(&mut self, source: &str) -> Result<(), Vec<ErrorS>> {
        let offset = self.source.len();

        self.source.reserve(source.len() + 1);
        self.source.push_str(source);
        self.source.push('\n');

        let function = Compiler::compile(source, offset, &mut self.gc)?;
        self.run_function(function).map_err(|e| vec![e])
    }

    fn run_function(&mut self, function: *mut ObjectFunction) -> Result<()> {
        self.stack_top = self.stack.as_mut_ptr();

        self.frames.clear();
        self.frame = CallFrame {
            closure: self.gc.alloc(ObjectClosure::new(function, Vec::new())),
            ip: unsafe { (*function).chunk.ops.as_ptr() },
            stack: self.stack_top,
        };
        let mut ip = self.frame.ip;

        self.dispatch(&mut ip)
    }

    fn dispatch(&mut self, ip: &mut *const u8) -> Result<()> {
        if cfg!(feature = "vm-trace") {
            let function = unsafe { (*self.frame.closure).function };
            let idx = unsafe { ip.offset_from((*function).chunk.ops.as_ptr()) };
            unsafe { (*function).chunk.debug_op(idx as usize) };
        }

        match Self::read_u8(ip) {
            op::CONSTANT => become Self::op_constant(self, ip),
            op::NIL => become Self::op_nil(self, ip),
            op::TRUE => become Self::op_true(self, ip),
            op::FALSE => become Self::op_false(self, ip),
            op::POP => become Self::op_pop(self, ip),
            op::GET_LOCAL => become Self::op_get_local(self, ip),
            op::SET_LOCAL => become Self::op_set_local(self, ip),
            op::GET_GLOBAL => become Self::op_get_global(self, ip),
            op::DEFINE_GLOBAL => become Self::op_define_global(self, ip),
            op::SET_GLOBAL => become Self::op_set_global(self, ip),
            op::GET_UPVALUE => become Self::op_get_upvalue(self, ip),
            op::SET_UPVALUE => become Self::op_set_upvalue(self, ip),
            op::GET_PROPERTY => become Self::op_get_property(self, ip),
            op::SET_PROPERTY => become Self::op_set_property(self, ip),
            op::GET_SUPER => become Self::op_get_super(self, ip),
            op::EQUAL => become Self::op_equal(self, ip),
            op::NOT_EQUAL => become Self::op_not_equal(self, ip),
            op::GREATER => become Self::op_greater(self, ip),
            op::GREATER_EQUAL => become Self::op_greater_equal(self, ip),
            op::LESS => become Self::op_less(self, ip),
            op::LESS_EQUAL => become Self::op_less_equal(self, ip),
            op::ADD => become Self::op_add(self, ip),
            op::SUBTRACT => become Self::op_subtract(self, ip),
            op::MULTIPLY => become Self::op_multiply(self, ip),
            op::DIVIDE => become Self::op_divide(self, ip),
            op::NOT => become Self::op_not(self, ip),
            op::NEGATE => become Self::op_negate(self, ip),
            op::PRINT => become Self::op_print(self, ip),
            op::JUMP => become Self::op_jump(self, ip),
            op::JUMP_IF_FALSE => become Self::op_jump_if_false(self, ip),
            op::LOOP => become Self::op_loop(self, ip),
            op::CALL => become Self::op_call(self, ip),
            op::INVOKE => become Self::op_invoke(self, ip),
            op::SUPER_INVOKE => become Self::op_super_invoke(self, ip),
            op::CLOSURE => become Self::op_closure(self, ip),
            op::CLOSE_UPVALUE => become Self::op_close_upvalue(self, ip),
            op::RETURN => become Self::op_return(self, ip),
            op::CLASS => become Self::op_class(self, ip),
            op::INHERIT => become Self::op_inherit(self, ip),
            op::METHOD => become Self::op_method(self, ip),
            _ => util::unreachable(),
        }
    }

    fn op_constant(&mut self, ip: &mut *const u8) -> Result<()> {
        let constant = self.read_value(ip);
        self.push(constant);
        become self.dispatch(ip)
    }

    fn op_nil(&mut self, ip: &mut *const u8) -> Result<()> {
        self.push(Value::NIL);
        become self.dispatch(ip)
    }

    fn op_true(&mut self, ip: &mut *const u8) -> Result<()> {
        self.push(Value::TRUE);
        become self.dispatch(ip)
    }

    fn op_false(&mut self, ip: &mut *const u8) -> Result<()> {
        self.push(Value::FALSE);
        become self.dispatch(ip)
    }

    fn op_pop(&mut self, ip: &mut *const u8) -> Result<()> {
        self.pop();
        become self.dispatch(ip)
    }

    fn op_get_local(&mut self, ip: &mut *const u8) -> Result<()> {
        let stack_idx = Self::read_u8(ip) as usize;
        let local = unsafe { *self.frame.stack.add(stack_idx) };
        self.push(local);
        become self.dispatch(ip)
    }

    fn op_set_local(&mut self, ip: &mut *const u8) -> Result<()> {
        let stack_idx = Self::read_u8(ip) as usize;
        let local = unsafe { self.frame.stack.add(stack_idx) };
        let value = self.peek(0);
        unsafe { *local = *value };
        become self.dispatch(ip)
    }

    fn op_get_global(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        match self.globals.get(&name) {
            Some(&value) => {
                self.push(value);
                become self.dispatch(ip)
            }
            None => {
                self.err(*ip, NameError::NotDefined { name: unsafe { (*name).value.to_string() } })
            }
        }
    }

    fn op_define_global(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let value = self.pop();
        self.globals.insert(name, value);
        become self.dispatch(ip)
    }

    fn op_set_global(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let value = unsafe { *self.peek(0) };
        match self.globals.entry(name) {
            Entry::Occupied(mut entry) => {
                entry.insert(value);
                become self.dispatch(ip)
            }
            Entry::Vacant(_) => {
                self.err(*ip, NameError::NotDefined { name: unsafe { (*name).value.to_string() } })
            }
        }
    }

    fn op_get_upvalue(&mut self, ip: &mut *const u8) -> Result<()> {
        let upvalue_idx = Self::read_u8(ip) as usize;
        let object = *unsafe { (*self.frame.closure).upvalues.get_unchecked(upvalue_idx) };
        let value = unsafe { *(*object).location };
        self.push(value);
        become self.dispatch(ip)
    }

    fn op_set_upvalue(&mut self, ip: &mut *const u8) -> Result<()> {
        let upvalue_idx = Self::read_u8(ip) as usize;
        let object = *unsafe { (*self.frame.closure).upvalues.get_unchecked(upvalue_idx) };
        let value = unsafe { (*object).location };
        unsafe { *value = *self.peek(0) };
        become self.dispatch(ip)
    }

    fn op_get_property(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let instance = {
            let value = unsafe { *self.peek(0) };
            let object = value.as_object();

            if value.is_object() && object.type_() == ObjectType::Instance {
                unsafe { object.instance }
            } else {
                return self.err(
                    *ip,
                    AttributeError::NoSuchAttribute {
                        type_: value.type_().to_string(),
                        name: unsafe { (*name).value.to_string() },
                    },
                );
            }
        };

        match unsafe { (*instance).fields.get(&name) } {
            Some(&field) => {
                unsafe { *self.peek(0) = field };
            }
            None => match unsafe { (*(*instance).class).methods.get(&name) } {
                Some(&method) => {
                    let bound_method = self.alloc(ObjectBoundMethod::new(instance, method));
                    unsafe { *self.peek(0) = Value::from(bound_method) };
                }
                None => {
                    return self.err(
                        *ip,
                        AttributeError::NoSuchAttribute {
                            type_: unsafe { (*(*(*instance).class).name).value.to_string() },
                            name: unsafe { (*name).value.to_string() },
                        },
                    );
                }
            },
        }

        become self.dispatch(ip)
    }

    fn op_set_property(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let instance = {
            let value = self.pop();
            let object = value.as_object();

            if value.is_object() && object.type_() == ObjectType::Instance {
                unsafe { object.instance }
            } else {
                return self.err(
                    *ip,
                    AttributeError::NoSuchAttribute {
                        type_: value.type_().to_string(),
                        name: unsafe { (*name).value.to_string() },
                    },
                );
            }
        };
        let value = unsafe { *self.peek(0) };
        unsafe { (*instance).fields.insert(name, value) };
        become self.dispatch(ip)
    }

    fn op_get_super(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let super_ = unsafe { self.pop().as_object().class };
        match unsafe { (*super_).methods.get(&name) } {
            Some(&method) => {
                let instance = unsafe { (*self.peek(0)).as_object().instance };
                let bound_method = self.alloc(ObjectBoundMethod::new(instance, method));
                unsafe { *self.peek(0) = Value::from(bound_method) };
            }
            None => {
                return self.err(
                    *ip,
                    AttributeError::NoSuchAttribute {
                        type_: unsafe { (*(*super_).name).value.to_string() },
                        name: unsafe { (*name).value.to_string() },
                    },
                );
            }
        }
        become self.dispatch(ip)
    }

    fn op_equal(&mut self, ip: &mut *const u8) -> Result<()> {
        let b = self.pop();
        let a_ptr = self.peek(0);
        unsafe { *a_ptr = Value::from(*a_ptr == b) };
        become self.dispatch(ip)
    }

    fn op_not_equal(&mut self, ip: &mut *const u8) -> Result<()> {
        let b = self.pop();
        let a_ptr = self.peek(0);
        unsafe { *a_ptr = Value::from(*a_ptr != b) };
        become self.dispatch(ip)
    }

    fn op_greater(&mut self, ip: &mut *const u8) -> Result<()> {
        binary_op_number!(self, ip, >, OpInfix::Greater)
    }

    fn op_greater_equal(&mut self, ip: &mut *const u8) -> Result<()> {
        binary_op_number!(self, ip, >=, OpInfix::GreaterEqual)
    }

    fn op_less(&mut self, ip: &mut *const u8) -> Result<()> {
        binary_op_number!(self, ip, <, OpInfix::Less)
    }

    fn op_less_equal(&mut self, ip: &mut *const u8) -> Result<()> {
        binary_op_number!(self, ip, <=, OpInfix::LessEqual)
    }

    fn op_add(&mut self, ip: &mut *const u8) -> Result<()> {
        let b = self.pop();
        let a_ptr = self.peek(0);
        let a = unsafe { *a_ptr };

        if a.is_number() && b.is_number() {
            unsafe { *a_ptr = Value::from(a.as_number() + b.as_number()) };
            become self.dispatch(ip);
        }

        if a.is_object() && b.is_object() {
            let a_obj = a.as_object();
            let b_obj = b.as_object();

            if a_obj.type_() == ObjectType::String && b_obj.type_() == ObjectType::String {
                let result = unsafe { [(*a_obj.string).value, (*b_obj.string).value] }.concat();
                let result = Value::from(self.alloc(result));
                unsafe { *a_ptr = result };
                become self.dispatch(ip);
            }
        }

        self.err(
            *ip,
            TypeError::UnsupportedOperandInfix {
                op: OpInfix::Add,
                lt_type: a.type_().to_string(),
                rt_type: b.type_().to_string(),
            },
        )
    }

    fn op_subtract(&mut self, ip: &mut *const u8) -> Result<()> {
        binary_op_number!(self, ip, -, OpInfix::Subtract)
    }

    fn op_multiply(&mut self, ip: &mut *const u8) -> Result<()> {
        binary_op_number!(self, ip, *, OpInfix::Multiply)
    }

    fn op_divide(&mut self, ip: &mut *const u8) -> Result<()> {
        binary_op_number!(self, ip, /, OpInfix::Divide)
    }

    fn op_not(&mut self, ip: &mut *const u8) -> Result<()> {
        let a_ptr = self.peek(0);
        unsafe { *a_ptr = !*a_ptr };
        become self.dispatch(ip)
    }

    fn op_negate(&mut self, ip: &mut *const u8) -> Result<()> {
        let a_ptr = self.peek(0);
        let value = unsafe { *a_ptr };
        if value.is_number() {
            unsafe { *a_ptr = Value::from(-value.as_number()) };
            become self.dispatch(ip)
        } else {
            self.err(
                *ip,
                TypeError::UnsupportedOperandPrefix {
                    op: OpPrefix::Negate,
                    rt_type: value.type_().to_string(),
                },
            )
        }
    }

    fn op_print(&mut self, ip: &mut *const u8) -> Result<()> {
        let value = self.pop();
        writeln!(self.stdout, "{value}")
            .or_else(|_| self.err(*ip, IoError::WriteError { file: "stdout".to_string() }))?;
        become self.dispatch(ip)
    }

    fn op_jump(&mut self, ip: &mut *const u8) -> Result<()> {
        let offset = Self::read_u16(ip) as usize;
        *ip = unsafe { ip.add(offset) };
        become self.dispatch(ip)
    }

    fn op_jump_if_false(&mut self, ip: &mut *const u8) -> Result<()> {
        let offset = Self::read_u16(ip) as usize;
        let value = self.peek(0);
        if !(unsafe { *value }.to_bool()) {
            *ip = unsafe { ip.add(offset) };
        }
        become self.dispatch(ip)
    }

    fn op_loop(&mut self, ip: &mut *const u8) -> Result<()> {
        let offset = Self::read_u16(ip) as usize;
        *ip = unsafe { ip.sub(offset) };
        become self.dispatch(ip)
    }

    fn op_call(&mut self, ip: &mut *const u8) -> Result<()> {
        let arg_count = Self::read_u8(ip) as usize;
        let callee = unsafe { *self.peek(arg_count) };
        self.call_value(ip, callee, arg_count)?;
        become self.dispatch(ip)
    }

    fn op_invoke(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let arg_count = Self::read_u8(ip) as usize;
        let instance = unsafe { (*self.peek(arg_count)).as_object().instance };

        match unsafe { (*instance).fields.get(&name) } {
            Some(&value) => {
                self.call_value(ip, value, arg_count)?;
                become self.dispatch(ip)
            }
            None => match unsafe { (*(*instance).class).methods.get(&name) } {
                Some(&method) => {
                    self.call_closure(ip, method, arg_count)?;
                    become self.dispatch(ip)
                }
                None => self.err(
                    *ip,
                    AttributeError::NoSuchAttribute {
                        type_: unsafe { (*(*(*instance).class).name).value.to_string() },
                        name: unsafe { (*name).value.to_string() },
                    },
                ),
            },
        }
    }

    fn op_super_invoke(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let arg_count = Self::read_u8(ip) as usize;
        let super_ = unsafe { self.pop().as_object().class };

        match unsafe { (*super_).methods.get(&name) } {
            Some(&method) => {
                self.call_closure(ip, method, arg_count)?;
                become self.dispatch(ip)
            }
            None => self.err(
                *ip,
                AttributeError::NoSuchAttribute {
                    type_: unsafe { (*(*super_).name).value.to_string() },
                    name: unsafe { (*name).value.to_string() },
                },
            ),
        }
    }

    fn op_closure(&mut self, ip: &mut *const u8) -> Result<()> {
        let function = unsafe { self.read_value(ip).as_object().function };

        let upvalue_count = unsafe { (*function).upvalue_count } as usize;
        let mut upvalues = Vec::with_capacity(upvalue_count);

        for _ in 0..upvalue_count {
            let is_local = Self::read_u8(ip);
            let upvalue_idx = Self::read_u8(ip) as usize;

            let upvalue = if is_local != 0 {
                let location = unsafe { self.frame.stack.add(upvalue_idx) };
                self.capture_upvalue(location)
            } else {
                *unsafe { (*self.frame.closure).upvalues.get_unchecked(upvalue_idx) }
            };
            upvalues.push(upvalue);
        }

        let closure = Value::from(self.alloc(ObjectClosure::new(function, upvalues)));
        self.push(closure);
        become self.dispatch(ip)
    }

    fn op_close_upvalue(&mut self, ip: &mut *const u8) -> Result<()> {
        let last = self.peek(0);
        self.close_upvalues(last);
        self.pop();
        become self.dispatch(ip)
    }

    fn op_return(&mut self, ip: &mut *const u8) -> Result<()> {
        let value = self.pop();
        self.close_upvalues(self.frame.stack);
        self.stack_top = self.frame.stack;

        match self.frames.pop() {
            Some(frame) => {
                self.frame = frame;
                *ip = self.frame.ip;
                self.push(value);
                become self.dispatch(ip)
            }
            None => {
                unlikely();
                Ok(())
            }
        }
    }

    fn op_class(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let class = Value::from(self.alloc(ObjectClass::new(name)));
        self.push(class);
        become self.dispatch(ip)
    }

    fn op_inherit(&mut self, ip: &mut *const u8) -> Result<()> {
        let class = unsafe { self.pop().as_object().class };
        let super_ = {
            let value = unsafe { *self.peek(0) };
            let object = value.as_object();

            if value.is_object() && object.type_() == ObjectType::Class {
                unsafe { object.class }
            } else {
                return self.err(
                    *ip,
                    TypeError::SuperclassInvalidType { type_: value.type_().to_string() },
                );
            }
        };

        unsafe { (*class).methods.clone_from(&(*super_).methods) };
        become self.dispatch(ip)
    }

    fn op_method(&mut self, ip: &mut *const u8) -> Result<()> {
        let name = unsafe { self.read_value(ip).as_object().string };
        let method = unsafe { self.pop().as_object().closure };
        let class = unsafe { (*self.peek(0)).as_object().class };
        unsafe { (*class).methods.insert(name, method) };
        become self.dispatch(ip)
    }

    fn alloc<T>(&mut self, object: impl GcAlloc<T>) -> T {
        if !cfg!(feature = "gc-off")
            && (cfg!(feature = "gc-stress") || GLOBAL.allocated_bytes() > self.next_gc)
        {
            self.gc();
        }
        self.gc.alloc(object)
    }

    #[cold]
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

    fn call_value(&mut self, ip: &mut *const u8, value: Value, arg_count: usize) -> Result<()> {
        if value.is_object() {
            let object = value.as_object();
            match object.type_() {
                ObjectType::BoundMethod => {
                    self.call_bound_method(ip, unsafe { object.bound_method }, arg_count)
                }
                ObjectType::Class => self.call_class(ip, unsafe { object.class }, arg_count),
                ObjectType::Closure => self.call_closure(ip, unsafe { object.closure }, arg_count),
                ObjectType::Native => self.call_native(ip, unsafe { object.native }, arg_count),
                _ => self.err(*ip, TypeError::NotCallable { type_: value.type_().to_string() }),
            }
        } else {
            self.err(*ip, TypeError::NotCallable { type_: value.type_().to_string() })
        }
    }

    fn call_bound_method(
        &mut self,
        ip: &mut *const u8,
        method: *mut ObjectBoundMethod,
        arg_count: usize,
    ) -> Result<()> {
        unsafe { *self.peek(arg_count) = Value::from((*method).this) };
        self.call_closure(ip, unsafe { (*method).closure }, arg_count)
    }

    fn call_class(
        &mut self,
        ip: &mut *const u8,
        class: *mut ObjectClass,
        arg_count: usize,
    ) -> Result<()> {
        let instance = self.alloc(ObjectInstance::new(class));
        unsafe { *self.peek(arg_count) = Value::from(instance) };

        match unsafe { (*class).methods.get(&self.init_string) } {
            Some(&init) => self.call_closure(ip, init, arg_count),
            None if arg_count != 0 => self.err(
                *ip,
                TypeError::ArityMismatch {
                    name: unsafe { (*self.init_string).value.to_string() },
                    exp_args: 0,
                    got_args: arg_count,
                },
            ),
            None => Ok(()),
        }
    }

    fn call_closure(
        &mut self,
        ip: &mut *const u8,
        closure: *mut ObjectClosure,
        arg_count: usize,
    ) -> Result<()> {
        if self.frames.len() >= self.frames.capacity() {
            return self.err(*ip, OverflowError::StackOverflow);
        }

        let function = unsafe { (*closure).function };
        let arity = unsafe { (*function).arity } as usize;
        if arg_count != arity {
            return self.err(
                *ip,
                TypeError::ArityMismatch {
                    name: unsafe { (*(*function).name).value }.to_string(),
                    exp_args: arity,
                    got_args: arg_count,
                },
            );
        }

        // Save current ip into the frame being pushed, then update ip
        // to point at the new function's bytecode.
        self.frame.ip = *ip;
        let frame = CallFrame {
            closure,
            ip: unsafe { (*function).chunk.ops.as_ptr() },
            stack: self.peek(arg_count),
        };
        unsafe { self.frames.push_unchecked(mem::replace(&mut self.frame, frame)) };
        *ip = self.frame.ip;

        Ok(())
    }

    fn call_native(
        &mut self,
        ip: &mut *const u8,
        native: *mut ObjectNative,
        arg_count: usize,
    ) -> Result<()> {
        self.pop();
        let value = match unsafe { (*native).native } {
            Native::Clock => {
                if arg_count != 0 {
                    return self.err(
                        *ip,
                        TypeError::ArityMismatch {
                            name: "clock".to_string(),
                            exp_args: 0,
                            got_args: arg_count,
                        },
                    );
                }
                Value::from(util::now())
            }
        };
        self.push(value);
        Ok(())
    }

    /// Reads an instruction / byte from the current [`Chunk`].
    fn read_u8(ip: &mut *const u8) -> u8 {
        let byte = unsafe { **ip };
        *ip = unsafe { ip.add(1) };
        byte
    }

    /// Reads a 16-bit value from the current [`Chunk`].
    fn read_u16(ip: &mut *const u8) -> u16 {
        let value = unsafe { (*ip as *const u16).read_unaligned() };
        *ip = unsafe { ip.add(2) };
        u16::from_le(value)
    }

    /// Reads a [`Value`] from the current [`Chunk`].
    fn read_value(&self, ip: &mut *const u8) -> Value {
        let constant_idx = Self::read_u8(ip) as usize;
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
        self.stack_top = self.peek(0);
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
    #[inline(never)]
    fn err(&self, ip: *const u8, err: impl Into<Error>) -> Result<()> {
        let function = unsafe { (*self.frame.closure).function };
        let idx = unsafe { ip.offset_from((*function).chunk.ops.as_ptr()) } as usize;
        let span = unsafe { (&(*function).chunk.spans)[idx - 1].clone() };
        Err((err.into(), span))
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

#[cold]
#[inline(always)]
fn unlikely() {}
