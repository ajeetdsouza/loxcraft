use std::convert::TryInto;
use std::mem;

use arrayvec::ArrayVec;
use lox_common::error::{ErrorS, NameError, OverflowError, Result, SyntaxError};
use lox_common::types::Span;
use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Stmt, StmtS};

use crate::gc::Gc;
use crate::object::ObjectFunction;
use crate::op;
use crate::value::Value;

#[derive(Debug)]
pub struct Compiler {
    ctx: CompilerCtx,
}

impl Compiler {
    /// Creates a compiler for a new script.
    pub fn new(gc: &mut Gc) -> Self {
        let name = gc.alloc("");
        Self {
            ctx: CompilerCtx {
                function: gc.alloc(ObjectFunction::new(name, 0)),
                type_: FunctionType::Script,
                locals: ArrayVec::new(),
                upvalues: ArrayVec::new(),
                parent: None,
                scope_depth: 0,
            },
        }
    }

    pub fn compile(source: &str, gc: &mut Gc) -> Result<*mut ObjectFunction, Vec<ErrorS>> {
        let mut compiler = Self::new(gc);
        let program = lox_syntax::parse(source)?;
        for stmt in &program.stmts {
            compiler.compile_stmt(stmt, gc).map_err(|e| vec![e])?;
        }
        compiler.emit_u8(op::NIL, &(0..0));
        compiler.emit_u8(op::RETURN, &(0..0));
        Ok(compiler.ctx.function)
    }

    fn compile_stmt(&mut self, (stmt, span): &StmtS, gc: &mut Gc) -> Result<()> {
        match stmt {
            Stmt::Block(block) => {
                self.begin_scope();
                for stmt in &block.stmts {
                    self.compile_stmt(stmt, gc)?;
                }
                self.end_scope(span);
            }
            Stmt::Class(class) => {
                let name = gc.alloc(&class.name).into();

                self.emit_u8(op::CLASS, span);
                self.emit_constant(name, span)?;

                if self.is_global() {
                    self.emit_u8(op::DEFINE_GLOBAL, span);
                    self.emit_constant(name, span)?;
                } else {
                    self.declare_local(&class.name, span)?;
                    self.define_local();
                }
            }
            Stmt::Expr(expr) => {
                self.compile_expr(&expr.value, gc)?;
                self.emit_u8(op::POP, span);
            }
            Stmt::For(for_) => {
                self.begin_scope();

                // Evaluate init statement. This may be an expression, a
                // variable assignment, or nothing at all.
                if let Some(init) = &for_.init {
                    self.compile_stmt(init, gc)?;
                }

                // START:
                let loop_start = self.start_loop();

                // Evaluate the condition, if it exists.
                let mut jump_to_end = None;
                if let Some(cond) = &for_.cond {
                    self.compile_expr(cond, gc)?;
                    // If the condition is false, go to END.
                    jump_to_end = Some(self.emit_jump(op::JUMP_IF_FALSE, span));
                    // Discard the condition.
                    self.emit_u8(op::POP, span);
                }

                // Evaluate the body.
                self.compile_stmt(&for_.body, gc)?;

                // Evaluate the increment expression, if it exists.
                if let Some(incr) = &for_.incr {
                    self.compile_expr(incr, gc)?;
                    // Discard the result of the expression.
                    self.emit_u8(op::POP, span);
                }

                // Go to START.
                self.emit_loop(loop_start, span)?;
                // END:
                if let Some(jump_to_end) = jump_to_end {
                    self.patch_jump(jump_to_end, span)?;
                    // Discard the condition.
                    self.emit_u8(op::POP, span);
                }

                self.end_scope(span);
            }
            Stmt::Fun(fun) => {
                let name = gc.alloc(&fun.name);
                let arity = fun
                    .params
                    .len()
                    .try_into()
                    .map_err(|_| (OverflowError::TooManyParams.into(), span.clone()))?;

                let ctx = CompilerCtx {
                    function: gc.alloc(ObjectFunction::new(name, arity)),
                    type_: FunctionType::Function,
                    locals: ArrayVec::new(),
                    upvalues: ArrayVec::new(),
                    parent: None,
                    scope_depth: self.ctx.scope_depth + 1,
                };
                self.begin_ctx(ctx);

                self.declare_local(&fun.name, span)?;
                self.define_local();
                for param in &fun.params {
                    self.declare_local(param, span)?;
                    self.define_local();
                }

                for stmt in &fun.body.stmts {
                    self.compile_stmt(stmt, gc)?;
                }

                // Implicit return at the end of the function.
                if unsafe { (*self.ctx.function).chunk.ops.last() } != Some(&op::RETURN) {
                    self.emit_u8(op::NIL, span);
                    self.emit_u8(op::RETURN, span);
                }

                let (function, upvalues) = self.end_ctx();
                let value = function.into();
                self.emit_u8(op::CLOSURE, span);
                self.emit_constant(value, span)?;

                for upvalue in &upvalues {
                    self.emit_u8(upvalue.is_local.into(), span);
                    self.emit_u8(upvalue.idx, span);
                }

                if self.is_global() {
                    self.emit_u8(op::DEFINE_GLOBAL, span);
                    self.emit_constant(name.into(), span)?;
                } else {
                    self.declare_local(&fun.name, span)?;
                    self.define_local();
                }
            }
            Stmt::If(if_) => {
                self.compile_expr(&if_.cond, gc)?;
                // If the condition is false, go to ELSE.
                let jump_to_else = self.emit_jump(op::JUMP_IF_FALSE, span);
                // Discard the condition.
                self.emit_u8(op::POP, span);
                // Evaluate the if branch.
                self.compile_stmt(&if_.then, gc)?;
                // Go to END.
                let jump_to_end = self.emit_jump(op::JUMP, span);

                // ELSE:
                self.patch_jump(jump_to_else, span)?;
                self.emit_u8(op::POP, span); // Discard the condition.
                if let Some(else_) = &if_.else_ {
                    self.compile_stmt(else_, gc)?;
                }

                // END:
                self.patch_jump(jump_to_end, span)?;
            }
            Stmt::Print(print) => {
                self.compile_expr(&print.value, gc)?;
                self.emit_u8(op::PRINT, span);
            }
            Stmt::Return(return_) => {
                if self.ctx.type_ == FunctionType::Script {
                    return Err((SyntaxError::ReturnOutsideFunction.into(), span.clone()));
                }

                match &return_.value {
                    Some(value) => self.compile_expr(value, gc)?,
                    None => self.emit_u8(op::NIL, span),
                };
                self.emit_u8(op::RETURN, span);
            }
            Stmt::Var(var) => {
                let name = &var.var.name;
                if self.is_global() {
                    let name = gc.alloc(name);
                    match &var.value {
                        Some(value) => self.compile_expr(value, gc)?,
                        None => self.emit_u8(op::NIL, span),
                    }
                    self.emit_u8(op::DEFINE_GLOBAL, span);
                    self.emit_constant(name.into(), span)?;
                } else {
                    self.declare_local(name, span)?;
                    match &var.value {
                        Some(value) => self.compile_expr(value, gc)?,
                        None => self.emit_u8(op::NIL, span),
                    }
                    self.define_local();
                }
            }
            Stmt::While(while_) => {
                // START:
                let loop_start = self.start_loop();

                // Evaluate condition.
                self.compile_expr(&while_.cond, gc)?;
                // If the condition is false, go to END.
                let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE, span);
                // Discard the condition.
                self.emit_u8(op::POP, span);
                // Evaluate the body of the loop.
                self.compile_stmt(&while_.body, gc)?;
                // Go to START.
                self.emit_loop(loop_start, span)?;

                // END:
                self.patch_jump(jump_to_end, span)?;
                // Discard the condition.
                self.emit_u8(op::POP, span);
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    /// Compute an expression and push it onto the stack.
    fn compile_expr(&mut self, (expr, span): &ExprS, gc: &mut Gc) -> Result<()> {
        match expr {
            Expr::Assign(assign) => {
                self.compile_expr(&assign.value, gc)?;
                self.set_variable(&assign.var.name, span, gc)?;
            }
            Expr::Call(call) => {
                let arg_count = call
                    .args
                    .len()
                    .try_into()
                    .map_err(|_| (OverflowError::TooManyArgs.into(), span.clone()))?;

                self.compile_expr(&call.callee, gc)?;
                for arg in &call.args {
                    self.compile_expr(arg, gc)?;
                }

                self.emit_u8(op::CALL, span);
                self.emit_u8(arg_count, span);
            }
            Expr::Get(get) => {
                self.compile_expr(&get.object, gc)?;

                let name = gc.alloc(&get.name).into();
                self.emit_u8(op::GET_PROPERTY, span);
                self.emit_constant(name, span)?;
            }
            Expr::Infix(infix) => {
                self.compile_expr(&infix.lt, gc)?;
                match infix.op {
                    OpInfix::Add => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::ADD, span);
                    }
                    OpInfix::Subtract => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::SUBTRACT, span);
                    }
                    OpInfix::Multiply => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::MULTIPLY, span);
                    }
                    OpInfix::Divide => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::DIVIDE, span);
                    }
                    OpInfix::Less => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::LESS, span);
                    }
                    OpInfix::LessEqual => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::LESS_EQUAL, span);
                    }
                    OpInfix::Greater => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::GREATER, span);
                    }
                    OpInfix::GreaterEqual => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::GREATER_EQUAL, span);
                    }
                    OpInfix::Equal => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::EQUAL, span);
                    }
                    OpInfix::NotEqual => {
                        self.compile_expr(&infix.rt, gc)?;
                        self.emit_u8(op::NOT_EQUAL, span);
                    }
                    OpInfix::LogicAnd => {
                        // If the first expression is false, go to END.
                        let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE, span);
                        // Otherwise, evaluate the right expression.
                        self.emit_u8(op::POP, span);
                        self.compile_expr(&infix.rt, gc)?;

                        // END:
                        // Short-circuit to the end.
                        self.patch_jump(jump_to_end, span)?;
                    }
                    OpInfix::LogicOr => {
                        // If the first expression is false, go to RIGHT_EXPR.
                        let jump_to_right_expr = self.emit_jump(op::JUMP_IF_FALSE, span);
                        // Otherwise, go to END.
                        let jump_to_end = self.emit_jump(op::JUMP, span);

                        // RIGHT_EXPR:
                        self.patch_jump(jump_to_right_expr, span)?;
                        // Discard the left value.
                        self.emit_u8(op::POP, span);
                        // Evaluate the right expression.
                        self.compile_expr(&infix.rt, gc)?;

                        // END:
                        // Short-circuit to the end.
                        self.patch_jump(jump_to_end, span)?;
                    }
                };
            }
            Expr::Literal(literal) => match literal {
                ExprLiteral::Bool(true) => self.emit_u8(op::TRUE, span),
                ExprLiteral::Bool(false) => self.emit_u8(op::FALSE, span),
                ExprLiteral::Nil => self.emit_u8(op::NIL, span),
                ExprLiteral::Number(number) => {
                    let value = (*number).into();
                    self.emit_u8(op::CONSTANT, span);
                    self.emit_constant(value, span)?;
                }
                ExprLiteral::String(string) => {
                    let string = gc.alloc(string);
                    unsafe { (*string).is_marked = true };
                    let value = string.into();
                    self.emit_u8(op::CONSTANT, span);
                    self.emit_constant(value, span)?;
                }
            },
            Expr::Prefix(prefix) => {
                self.compile_expr(&prefix.rt, gc)?;
                match prefix.op {
                    OpPrefix::Negate => self.emit_u8(op::NEGATE, span),
                    OpPrefix::Not => self.emit_u8(op::NOT, span),
                };
            }
            Expr::Set(set) => {
                self.compile_expr(&set.value, gc)?;
                self.compile_expr(&set.object, gc)?;

                let name = gc.alloc(&set.name).into();
                self.emit_u8(op::SET_PROPERTY, span);
                self.emit_constant(name, span)?;
            }
            Expr::Var(var) => self.get_variable(&var.var.name, span, gc)?,
            _ => unimplemented!(),
        }
        Ok(())
    }

    /// Pushes the current ctx to parent and assigns it to the given ctx.
    fn begin_ctx(&mut self, ctx: CompilerCtx) {
        let ctx = mem::replace(&mut self.ctx, ctx);
        self.ctx.parent = Some(Box::new(ctx));
    }

    /// Pops the current ctx and extracts a [`Function`] from it.
    fn end_ctx(&mut self) -> (*mut ObjectFunction, ArrayVec<Upvalue, 256>) {
        let parent = self.ctx.parent.take().expect("tried to end context in a script");
        let ctx = mem::replace(&mut self.ctx, *parent);
        (ctx.function, ctx.upvalues)
    }

    fn get_variable(&mut self, name: &str, span: &Span, gc: &mut Gc) -> Result<()> {
        if let Some(local_idx) = self.ctx.resolve_local(name, false, span)? {
            self.emit_u8(op::GET_LOCAL, span);
            self.emit_u8(local_idx, span);
        } else if let Some(upvalue_idx) = self.ctx.resolve_upvalue(name, span)? {
            self.emit_u8(op::GET_UPVALUE, span);
            self.emit_u8(upvalue_idx, span);
        } else {
            let name = gc.alloc(name);
            self.emit_u8(op::GET_GLOBAL, span);
            self.emit_constant(name.into(), span)?;
        }
        Ok(())
    }

    fn set_variable(&mut self, name: &str, span: &Span, gc: &mut Gc) -> Result<()> {
        if let Some(local_idx) = self.ctx.resolve_local(name, false, span)? {
            self.emit_u8(op::SET_LOCAL, span);
            self.emit_u8(local_idx, span);
        } else if let Some(upvalue_idx) = self.ctx.resolve_upvalue(name, span)? {
            self.emit_u8(op::SET_UPVALUE, span);
            self.emit_u8(upvalue_idx, span);
        } else {
            let name = gc.alloc(name);
            self.emit_u8(op::SET_GLOBAL, span);
            self.emit_constant(name.into(), span)?;
        }
        Ok(())
    }

    fn declare_local(&mut self, name: &str, span: &Span) -> Result<()> {
        for local in self.ctx.locals.iter().rev() {
            if local.depth < self.ctx.scope_depth {
                break;
            }
            if local.name == name {
                return Err((
                    NameError::AlreadyDefined { name: name.to_string() }.into(),
                    span.clone(),
                ));
            }
        }

        let local = Local {
            name: name.to_string(),
            depth: self.ctx.scope_depth,
            is_initialized: false,
            is_captured: false,
        };
        self.ctx
            .locals
            .try_push(local)
            .map_err(|_| (OverflowError::TooManyLocals.into(), span.clone()))
    }

    fn define_local(&mut self) {
        self.ctx.locals.last_mut().unwrap().is_initialized = true;
    }

    /// A jump takes 1 byte for the instruction followed by 2 bytes for the
    /// offset. The offset is initialized as a dummy value, and is later patched
    /// to the correct value.
    ///
    /// It returns the index of the offset which is to be patched.
    fn emit_jump(&mut self, opcode: u8, span: &Span) -> usize {
        self.emit_u8(opcode, span);
        self.emit_u8(0xFF, span);
        self.emit_u8(0xFF, span);
        unsafe { (*self.ctx.function).chunk.ops.len() - 2 }
    }

    /// Takes the index of the jump offset to be patched as input, and patches
    /// it to point to the current instruction.
    fn patch_jump(&mut self, offset_idx: usize, span: &Span) -> Result<()> {
        // The extra -2 is to account for the space taken by the offset.
        let offset = unsafe { (*self.ctx.function).chunk.ops.len() - 2 - offset_idx };
        let offset =
            offset.try_into().map_err(|_| (OverflowError::JumpTooLarge.into(), span.clone()))?;
        let offset = u16::to_le_bytes(offset);
        unsafe {
            [
                (*self.ctx.function).chunk.ops[offset_idx],
                (*self.ctx.function).chunk.ops[offset_idx + 1],
            ] = offset
        };
        Ok(())
    }

    fn start_loop(&self) -> usize {
        unsafe { (*self.ctx.function).chunk.ops.len() }
    }

    fn emit_loop(&mut self, start_idx: usize, span: &Span) -> Result<()> {
        // The extra +3 is to account for the space taken by the instruction and
        // the offset.
        let offset = unsafe { (*self.ctx.function).chunk.ops.len() } + 3 - start_idx;
        let offset =
            offset.try_into().map_err(|_| (OverflowError::JumpTooLarge.into(), span.clone()))?;
        let offset = u16::to_le_bytes(offset);

        self.emit_u8(op::LOOP, span);
        self.emit_u8(offset[0], span);
        self.emit_u8(offset[1], span);

        Ok(())
    }

    fn begin_scope(&mut self) {
        self.ctx.scope_depth += 1;
    }

    fn end_scope(&mut self, span: &Span) {
        self.ctx.scope_depth -= 1;

        // Remove all locals that are no longer in scope.
        while let Some(local) = self.ctx.locals.last() {
            if local.depth > self.ctx.scope_depth {
                if local.is_captured {
                    self.emit_u8(op::CLOSE_UPVALUE, span);
                } else {
                    self.emit_u8(op::POP, span);
                }
                self.ctx.locals.pop();
            } else {
                break;
            }
        }
    }

    fn emit_u8(&mut self, byte: u8, span: &Span) {
        unsafe { (*self.ctx.function).chunk.write_u8(byte, span) };
    }

    fn emit_constant(&mut self, value: Value, span: &Span) -> Result<()> {
        let constant_idx = unsafe { (*self.ctx.function).chunk.write_constant(value, span)? };
        self.emit_u8(constant_idx, span);
        Ok(())
    }

    fn is_global(&self) -> bool {
        self.ctx.scope_depth == 0
    }
}

#[derive(Debug)]
pub struct CompilerCtx {
    function: *mut ObjectFunction,
    type_: FunctionType,
    locals: ArrayVec<Local, 256>,
    // TODO: how is this 256 in the book?
    upvalues: ArrayVec<Upvalue, 256>,
    parent: Option<Box<CompilerCtx>>,
    scope_depth: usize,
}

impl CompilerCtx {
    fn resolve_local(&mut self, name: &str, capture: bool, span: &Span) -> Result<Option<u8>> {
        match self.locals.iter_mut().enumerate().rfind(|(_, local)| local.name == name) {
            Some((idx, local)) => {
                if local.is_initialized {
                    if capture {
                        local.is_captured = true;
                    }
                    Ok(Some(idx.try_into().unwrap()))
                } else {
                    Err((
                        NameError::AccessInsideInitializer { name: name.to_string() }.into(),
                        span.clone(),
                    ))
                }
            }
            None => Ok(None),
        }
    }

    fn resolve_upvalue(&mut self, name: &str, span: &Span) -> Result<Option<u8>> {
        let local_idx = match &mut self.parent {
            Some(parent) => parent.resolve_local(name, true, span)?,
            None => return Ok(None),
        };

        if let Some(local_idx) = local_idx {
            let upvalue_idx = self.add_upvalue(local_idx, true, span)?;
            return Ok(Some(upvalue_idx));
        };

        let upvalue_idx = match &mut self.parent {
            Some(parent) => parent.resolve_upvalue(name, span)?,
            None => return Ok(None),
        };

        if let Some(upvalue_idx) = upvalue_idx {
            let upvalue_idx = self.add_upvalue(upvalue_idx, false, span)?;
            return Ok(Some(upvalue_idx));
        };

        Ok(None)
    }

    fn add_upvalue(&mut self, idx: u8, is_local: bool, span: &Span) -> Result<u8> {
        let upvalue = Upvalue { idx, is_local };
        let upvalue_idx = match self.upvalues.iter().position(|u| u == &upvalue) {
            Some(upvalue_idx) => upvalue_idx,
            None => {
                self.upvalues
                    .try_push(upvalue)
                    .map_err(|_| (OverflowError::TooManyUpvalues.into(), span.clone()))?;
                let upvalues = self.upvalues.len();

                unsafe { (*self.function).upvalue_count = upvalues.try_into().unwrap() };
                upvalues - 1
            }
        };

        Ok(upvalue_idx.try_into().unwrap())
    }
}

#[derive(Debug, Default)]
struct Local {
    /// The name of the variable.
    name: String,
    /// The scope depth of the variable, i.e. the number of nested scopes that
    /// surround it. This starts at 1, because global scopes don't have local
    /// variables.
    depth: usize,
    is_initialized: bool,
    is_captured: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct Upvalue {
    idx: u8,
    is_local: bool,
}

#[derive(Debug, Eq, PartialEq)]
enum FunctionType {
    /// A function that has been defined in code.
    Function,
    /// The global-level function that is called when the program starts.
    Script,
}
