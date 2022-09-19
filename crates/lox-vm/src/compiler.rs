use std::convert::TryInto;
use std::mem;

use lox_common::error::{ErrorS, NameError, OverflowError, Result, SyntaxError};
use lox_common::types::Span;
use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Stmt, StmtS};

use crate::chunk::Chunk;
use crate::intern::Intern;
use crate::op;
use crate::value::{Function, Object, Value};

#[derive(Debug)]
pub struct Compiler {
    ctx: CompilerCtx,
}

impl Compiler {
    /// Creates a compiler for a new script.
    pub fn new(intern: &mut Intern) -> Self {
        let (name, _) = intern.insert_str("");
        Self {
            ctx: CompilerCtx {
                function: Function { name, arity: 0, upvalues: 0, chunk: Chunk::default() },
                type_: FunctionType::Script,
                locals: Vec::new(),
                upvalues: Vec::new(),
                parent: None,
                scope_depth: 0,
            },
        }
    }

    pub fn compile(source: &str, intern: &mut Intern) -> Result<Function, Vec<ErrorS>> {
        let mut compiler = Self::new(intern);
        let program = lox_syntax::parse(source)?;
        for stmt in &program.stmts {
            compiler.compile_stmt(stmt, intern).map_err(|e| vec![e])?;
        }
        compiler.emit_u8(op::NIL, &(0..0));
        compiler.emit_u8(op::RETURN, &(0..0));
        Ok(compiler.ctx.function)
    }

    fn compile_stmt(&mut self, (stmt, span): &StmtS, intern: &mut Intern) -> Result<()> {
        match stmt {
            Stmt::Block(block) => {
                self.begin_scope();
                for stmt in &block.stmts {
                    self.compile_stmt(stmt, intern)?;
                }
                self.end_scope(span);
            }
            Stmt::Expr(expr) => {
                self.compile_expr(&expr.value, intern)?;
                self.emit_u8(op::POP, span);
            }
            Stmt::For(for_) => {
                self.begin_scope();

                // Evaluate init statement. This may be an expression, a variable
                // assignment, or nothing at all.
                if let Some(init) = &for_.init {
                    self.compile_stmt(init, intern)?;
                }

                // START:
                let loop_start = self.start_loop();

                // Evaluate the condition, if it exists.
                let mut jump_to_end = None;
                if let Some(cond) = &for_.cond {
                    self.compile_expr(cond, intern)?;
                    // If the condition is false, go to END.
                    jump_to_end = Some(self.emit_jump(op::JUMP_IF_FALSE, span));
                    // Discard the condition.
                    self.emit_u8(op::POP, span);
                }

                // Evaluate the body.
                self.compile_stmt(&for_.body, intern)?;

                // Evaluate the increment expression, if it exists.
                if let Some(incr) = &for_.incr {
                    self.compile_expr(incr, intern)?;
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
                let (name, _) = intern.insert_str(&fun.name);
                let arity =
                    fun.params.len().try_into().map_err(|_| (OverflowError::TooManyParams.into(), span.clone()))?;

                let ctx = CompilerCtx {
                    function: Function { name, arity, upvalues: 0, chunk: Chunk::default() },
                    type_: FunctionType::Function,
                    locals: Vec::new(),
                    upvalues: Vec::new(),
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
                    self.compile_stmt(stmt, intern)?;
                }

                // Implicit return at the end of the function.
                if self.ctx.function.chunk.ops.last() != Some(&op::RETURN) {
                    self.emit_u8(op::NIL, span);
                    self.emit_u8(op::RETURN, span);
                }

                let (function, upvalues) = self.end_ctx();
                let object: Object = function.into();
                let value: Value = Box::into_raw(Box::new(object)).into();
                self.emit_u8(op::CLOSURE, span);
                self.emit_constant(value, span)?;

                for upvalue in &upvalues {
                    self.ctx.function.chunk.write_u8(upvalue.is_local as u8, span);
                    self.ctx.function.chunk.write_u8(upvalue.idx, span);
                }

                if self.ctx.scope_depth == 0 {
                    self.emit_u8(op::DEFINE_GLOBAL, span);
                    self.emit_constant(name.into(), span)?;
                } else {
                    self.declare_local(&fun.name, span)?;
                    self.define_local();
                }
            }
            Stmt::If(if_) => {
                self.compile_expr(&if_.cond, intern)?;
                // If the condition is false, go to ELSE.
                let jump_to_else = self.emit_jump(op::JUMP_IF_FALSE, span);
                // Discard the condition.
                self.emit_u8(op::POP, span);
                // Evaluate the if branch.
                self.compile_stmt(&if_.then, intern)?;
                // Go to END.
                let jump_to_end = self.emit_jump(op::JUMP, span);

                // ELSE:
                self.patch_jump(jump_to_else, span)?;
                self.emit_u8(op::POP, span); // Discard the condition.
                if let Some(else_) = &if_.else_ {
                    self.compile_stmt(&else_, intern)?;
                }

                // END:
                self.patch_jump(jump_to_end, span)?;
            }
            Stmt::Print(print) => {
                self.compile_expr(&print.value, intern)?;
                self.emit_u8(op::PRINT, span);
            }
            Stmt::Return(return_) => {
                if self.ctx.type_ == FunctionType::Script {
                    return Err((SyntaxError::ReturnOutsideFunction.into(), span.clone()));
                }

                match &return_.value {
                    Some(value) => self.compile_expr(value, intern)?,
                    None => self.emit_u8(op::NIL, span),
                };
                self.emit_u8(op::RETURN, span);
            }
            Stmt::Var(var) => {
                let name = &var.var.name;
                if self.ctx.scope_depth == 0 {
                    let (name, _) = intern.insert_str(name);
                    match &var.value {
                        Some(value) => self.compile_expr(value, intern)?,
                        None => self.emit_u8(op::NIL, span),
                    }
                    self.emit_u8(op::DEFINE_GLOBAL, span);
                    self.emit_constant(name.into(), span)?;
                } else {
                    self.declare_local(name, span)?;
                    match &var.value {
                        Some(value) => self.compile_expr(value, intern)?,
                        None => self.emit_u8(op::NIL, span),
                    }
                    self.define_local();
                }
            }
            Stmt::While(while_) => {
                // START:
                let loop_start = self.start_loop();

                // Evaluate condition.
                self.compile_expr(&while_.cond, intern)?;
                // If the condition is false, go to END.
                let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE, span);
                // Discard the condition.
                self.emit_u8(op::POP, span);
                // Evaluate the body of the loop.
                self.compile_stmt(&while_.body, intern)?;
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
    fn compile_expr(&mut self, (expr, span): &ExprS, intern: &mut Intern) -> Result<()> {
        match expr {
            Expr::Assign(assign) => {
                self.compile_expr(&assign.value, intern)?;
                self.set_variable(&assign.var.name, span, intern)?;
            }
            Expr::Call(call) => {
                let arg_count =
                    call.args.len().try_into().map_err(|_| (OverflowError::TooManyArgs.into(), span.clone()))?;

                self.compile_expr(&call.callee, intern)?;
                for arg in &call.args {
                    self.compile_expr(arg, intern)?;
                }

                self.emit_u8(op::CALL, span);
                self.emit_u8(arg_count, span);
            }
            Expr::Infix(infix) => {
                self.compile_expr(&infix.lt, intern)?;
                match infix.op {
                    OpInfix::Add => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::ADD, span);
                    }
                    OpInfix::Subtract => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::SUBTRACT, span);
                    }
                    OpInfix::Multiply => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::MULTIPLY, span);
                    }
                    OpInfix::Divide => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::DIVIDE, span);
                    }
                    OpInfix::Less => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::LESS, span);
                    }
                    OpInfix::LessEqual => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::LESS_EQUAL, span);
                    }
                    OpInfix::Greater => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::GREATER, span);
                    }
                    OpInfix::GreaterEqual => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::GREATER_EQUAL, span);
                    }
                    OpInfix::Equal => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::EQUAL, span);
                    }
                    OpInfix::NotEqual => {
                        self.compile_expr(&infix.rt, intern)?;
                        self.emit_u8(op::NOT_EQUAL, span);
                    }
                    OpInfix::LogicAnd => {
                        // If the first expression is false, go to END.
                        let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE, span);
                        // Otherwise, evaluate the right expression.
                        self.emit_u8(op::POP, span);
                        self.compile_expr(&infix.rt, intern)?;

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
                        self.compile_expr(&infix.rt, intern)?;

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
                    let (object, _) = intern.insert_str(string);
                    unsafe { (*object).is_marked = true };
                    let value = object.into();
                    self.emit_u8(op::CONSTANT, span);
                    self.emit_constant(value, span)?;
                }
            },
            Expr::Prefix(prefix) => {
                self.compile_expr(&prefix.rt, intern)?;
                match prefix.op {
                    OpPrefix::Negate => self.emit_u8(op::NEGATE, span),
                    OpPrefix::Not => self.emit_u8(op::NOT, span),
                };
            }
            Expr::Var(var) => self.get_variable(&var.var.name, span, intern)?,
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
    fn end_ctx(&mut self) -> (Function, Vec<Upvalue>) {
        let parent = self.ctx.parent.take().expect("tried to end context in a script");
        let ctx = mem::replace(&mut self.ctx, *parent);
        (ctx.function, ctx.upvalues)
    }

    fn get_variable(&mut self, name: &str, span: &Span, intern: &mut Intern) -> Result<()> {
        if let Some(local_idx) = self.ctx.resolve_local(name, false, span)? {
            println!("found local: {name}");
            self.emit_u8(op::GET_LOCAL, span);
            self.emit_u8(local_idx, span);
        } else if let Some(upvalue_idx) = self.ctx.resolve_upvalue(name, span)? {
            println!("found upvalue: {name}");
            self.emit_u8(op::GET_UPVALUE, span);
            self.emit_u8(upvalue_idx, span);
        } else {
            println!("found global: {name}");
            let (name, _) = intern.insert_str(name);
            self.emit_u8(op::GET_GLOBAL, span);
            self.emit_constant(name.into(), span)?;
        }
        Ok(())
    }

    fn set_variable(&mut self, name: &str, span: &Span, intern: &mut Intern) -> Result<()> {
        if let Some(local_idx) = self.ctx.resolve_local(name, false, span)? {
            self.emit_u8(op::SET_LOCAL, span);
            self.emit_u8(local_idx, span);
        } else if let Some(upvalue_idx) = self.ctx.resolve_upvalue(name, span)? {
            self.emit_u8(op::SET_UPVALUE, span);
            self.emit_u8(upvalue_idx, span);
        } else {
            let (name, _) = intern.insert_str(name);
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
                return Err((NameError::AlreadyDefined { name: name.to_string() }.into(), span.clone()));
            }
        }

        let local =
            Local { name: name.to_string(), depth: self.ctx.scope_depth, is_initialized: false, is_captured: false };
        self.ctx.locals.push(local);

        if self.ctx.locals.len() > u8::MAX as usize {
            return Err((OverflowError::TooManyLocals.into(), span.clone()));
        }
        Ok(())
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
        self.ctx.function.chunk.ops.len() - 2
    }

    /// Takes the index of the jump offset to be patched as input, and patches
    /// it to point to the current instruction.
    fn patch_jump(&mut self, offset_idx: usize, span: &Span) -> Result<()> {
        // The extra -2 is to account for the space taken by the offset.
        let offset = self.ctx.function.chunk.ops.len() - 2 - offset_idx;
        let offset = offset.try_into().map_err(|_| (OverflowError::JumpTooLarge.into(), span.clone()))?;
        let offset = u16::to_le_bytes(offset);
        [self.ctx.function.chunk.ops[offset_idx], self.ctx.function.chunk.ops[offset_idx + 1]] = offset;
        Ok(())
    }

    fn start_loop(&self) -> usize {
        self.ctx.function.chunk.ops.len()
    }

    fn emit_loop(&mut self, start_idx: usize, span: &Span) -> Result<()> {
        // The extra +3 is to account for the space taken by the instruction
        // and the offset.
        let offset = self.ctx.function.chunk.ops.len() + 3 - start_idx;
        let offset = offset.try_into().map_err(|_| (OverflowError::JumpTooLarge.into(), span.clone()))?;
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
        self.ctx.function.chunk.write_u8(byte, span);
    }

    fn emit_constant(&mut self, value: Value, span: &Span) -> Result<()> {
        let constant_idx = self.ctx.function.chunk.write_constant(value, span)?;
        self.emit_u8(constant_idx, span);
        Ok(())
    }
}

#[derive(Debug)]
pub struct CompilerCtx {
    function: Function,
    type_: FunctionType,
    locals: Vec<Local>,
    upvalues: Vec<Upvalue>,
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
                    Ok(Some(idx as u8))
                } else {
                    Err((NameError::AccessInsideInitializer { name: name.to_string() }.into(), span.clone()))
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
                self.upvalues.push(upvalue);
                self.function.upvalues = self
                    .function
                    .upvalues
                    .checked_add(1)
                    .ok_or_else(|| (OverflowError::TooManyUpvalues.into(), span.clone()))?;
                self.upvalues.len() - 1
            }
        };

        Ok(upvalue_idx as u8)
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
