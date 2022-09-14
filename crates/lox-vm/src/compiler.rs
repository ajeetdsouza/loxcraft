use std::convert::TryInto;

use crate::chunk::Chunk;
use crate::intern::Intern;
use crate::op;
use crate::value::Value;
use lox_common::error::{ErrorS, OverflowError, Result};
use lox_common::types::Span;
use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Program, Stmt, StmtS};

#[derive(Default)]
pub struct Compiler {
    chunk: Chunk,
    locals: Vec<Local>,
    scope_depth: usize,
}

impl Compiler {
    pub fn compile(source: &str, intern: &mut Intern) -> Result<Chunk, Vec<ErrorS>> {
        let mut compiler = Compiler::default();
        let program = lox_syntax::parse(source)?;
        if let Err(e) = compiler.compile_program(&program, intern) {
            return Err(vec![e]);
        }
        Ok(compiler.chunk)
    }

    fn compile_program(&mut self, program: &Program, intern: &mut Intern) -> Result<()> {
        for stmt in &program.stmts {
            self.compile_stmt(stmt, intern)?;
        }
        self.emit_u8(op::RETURN, &(0..0));
        Ok(())
    }

    fn compile_stmt(&mut self, (stmt, span): &StmtS, intern: &mut Intern) -> Result<()> {
        match stmt {
            Stmt::Block(block) => {
                self.begin_scope();
                for stmt in &block.stmts {
                    self.compile_stmt(stmt, intern)?;
                }
                self.end_scope();
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

                self.end_scope();
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
            Stmt::Var(var) => {
                let name = &var.var.name;
                let value = var.value.as_ref().unwrap_or(&(Expr::Literal(ExprLiteral::Nil), 0..0));
                if self.scope_depth == 0 {
                    let (name, _) = intern.insert_str(name);
                    self.compile_expr(value, intern)?;
                    self.emit_u8(op::DEFINE_GLOBAL, span);
                    self.emit_constant(name.into(), span)?;
                } else {
                    self.declare_local(name);
                    self.compile_expr(value, intern)?;
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
            Expr::Literal(literal) => {
                let value = match literal {
                    ExprLiteral::Bool(bool) => (*bool).into(),
                    ExprLiteral::Nil => Value::Nil,
                    ExprLiteral::Number(number) => (*number).into(),
                    ExprLiteral::String(string) => {
                        let (object, _) = intern.insert_str(string);
                        unsafe { (*object).is_marked = true };
                        object.into()
                    }
                };
                self.emit_u8(op::CONSTANT, span);
                self.emit_constant(value, span)?;
            }
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

    fn get_variable(&mut self, name: &str, span: &Span, intern: &mut Intern) -> Result<()> {
        if let Some(local_idx) = self.resolve_local(name) {
            self.emit_u8(op::GET_LOCAL, span);
            self.emit_u8(local_idx, span);
        } else {
            let (name, _) = intern.insert_str(name);
            self.emit_u8(op::GET_GLOBAL, span);
            self.emit_constant(name.into(), span)?;
        }
        Ok(())
    }

    fn set_variable(&mut self, name: &str, span: &Span, intern: &mut Intern) -> Result<()> {
        if let Some(local_idx) = self.resolve_local(name) {
            self.emit_u8(op::SET_LOCAL, span);
            self.emit_u8(local_idx, span);
        } else {
            let (name, _) = intern.insert_str(name);
            self.emit_u8(op::SET_GLOBAL, span);
            self.emit_constant(name.into(), span)?;
        }
        Ok(())
    }

    fn declare_local(&mut self, name: &str) {
        for local in self.locals.iter().rev() {
            if local.depth < self.scope_depth {
                break;
            }
            if local.name == name {
                panic!("Variable with this name already declared in this scope.");
            }
        }

        let local = Local { name: name.to_string(), depth: 0, is_initialized: false };
        self.locals.push(local);
        if self.locals.len() >= u8::MAX as usize {
            panic!("Too many local variables in function.");
        }
    }

    fn define_local(&mut self) {
        self.locals.last_mut().unwrap().is_initialized = true;
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        let (idx, local) = self.locals.iter().enumerate().rfind(|(_, local)| local.name == name)?;
        if local.is_initialized {
            Some(idx.try_into().unwrap())
        } else {
            panic!("cannot define variable in its own initializer");
        }
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
        self.chunk.ops.len() - 2
    }

    /// Takes the index of the jump offset to be patched as input, and patches
    /// it to point to the current instruction.
    fn patch_jump(&mut self, offset_idx: usize, span: &Span) -> Result<()> {
        // The extra -2 is to account for the space taken by the offset.
        let offset = self.chunk.ops.len() - 2 - offset_idx;
        let offset =
            offset.try_into().map_err(|_| (OverflowError::JumpTooLarge.into(), span.clone()))?;
        let offset = u16::to_le_bytes(offset);
        [self.chunk.ops[offset_idx], self.chunk.ops[offset_idx + 1]] = offset;
        Ok(())
    }

    fn start_loop(&self) -> usize {
        self.chunk.ops.len()
    }

    fn emit_loop(&mut self, start_idx: usize, span: &Span) -> Result<()> {
        // The extra +3 is to account for the space taken by the instruction
        // and the offset.
        let offset = self.chunk.ops.len() + 3 - start_idx;
        let offset =
            offset.try_into().map_err(|_| (OverflowError::JumpTooLarge.into(), span.clone()))?;
        let offset = u16::to_le_bytes(offset);

        self.emit_u8(op::LOOP, span);
        self.emit_u8(offset[0], span);
        self.emit_u8(offset[1], span);

        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        // Remove all locals that are no longer in scope.
        while let Some(local) = self.locals.last() {
            if local.depth > self.scope_depth {
                self.locals.pop();
            } else {
                break;
            }
        }
    }

    fn emit_u8(&mut self, byte: u8, span: &Span) {
        self.chunk.write_u8(byte, span);
    }

    fn emit_constant(&mut self, value: Value, span: &Span) -> Result<()> {
        let constant_idx = self.chunk.write_constant(value, span)?;
        self.emit_u8(constant_idx, span);
        Ok(())
    }
}

struct Local {
    name: String,
    depth: usize,
    is_initialized: bool,
}
