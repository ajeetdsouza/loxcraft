use std::convert::TryInto;

use crate::chunk::Chunk;
use crate::intern::Intern;
use crate::op;
use crate::value::Value;
use lox_syntax::ast::{Expr, ExprLiteral, ExprS, OpInfix, OpPrefix, Program, Stmt, StmtS};

#[derive(Default)]
pub struct Compiler {
    chunk: Chunk,
    locals: Vec<Local>,
    scope_depth: usize,
}

impl Compiler {
    pub fn compile(source: &str, intern: &mut Intern) -> Chunk {
        let mut compiler = Compiler::default();
        let (program, errors) = lox_syntax::parse(source);
        assert!(errors.is_empty());
        compiler.compile_program(&program, intern);
        compiler.chunk
    }

    fn compile_program(&mut self, program: &Program, intern: &mut Intern) {
        for stmt in &program.stmts {
            self.compile_stmt(stmt, intern);
        }
        self.emit_u8(op::RETURN);
    }

    fn compile_stmt(&mut self, (stmt, span): &StmtS, intern: &mut Intern) {
        match stmt {
            Stmt::Block(block) => {
                self.begin_scope();
                for stmt in &block.stmts {
                    self.compile_stmt(stmt, intern);
                }
                self.end_scope();
            }
            Stmt::Expr(expr) => {
                self.compile_expr(&expr.value, intern);
                self.emit_u8(op::POP);
            }
            Stmt::For(for_) => {
                self.begin_scope();

                // Evaluate init statement. This may be an expression, a variable
                // assignment, or nothing at all.
                if let Some(init) = &for_.init {
                    self.compile_stmt(init, intern);
                }

                // START:
                let loop_start = self.start_loop();

                // Evaluate the condition, if it exists.
                let mut jump_to_end = None;
                if let Some(cond) = &for_.cond {
                    self.compile_expr(cond, intern);
                    // If the condition is false, go to END.
                    jump_to_end = Some(self.emit_jump(op::JUMP_IF_FALSE));
                    // Discard the condition.
                    self.emit_u8(op::POP);
                }

                // Evaluate the body.
                self.compile_stmt(&for_.body, intern);

                // Evaluate the increment expression, if it exists.
                if let Some(incr) = &for_.incr {
                    self.compile_expr(incr, intern);
                    // Discard the result of the expression.
                    self.emit_u8(op::POP);
                }

                // Go to START.
                self.emit_loop(loop_start);
                // END:
                if let Some(jump_to_end) = jump_to_end {
                    self.patch_jump(jump_to_end);
                    // Discard the condition.
                    self.emit_u8(op::POP);
                }

                self.end_scope();
            }
            Stmt::If(if_) => {
                self.compile_expr(&if_.cond, intern);
                // If the condition is false, go to ELSE.
                let jump_to_else = self.emit_jump(op::JUMP_IF_FALSE);
                // Discard the condition.
                self.emit_u8(op::POP);
                // Evaluate the if branch.
                self.compile_stmt(&if_.then, intern);
                // Go to END.
                let jump_to_end = self.emit_jump(op::JUMP);

                // ELSE:
                self.patch_jump(jump_to_else);
                self.emit_u8(op::POP); // Discard the condition.
                if let Some(else_) = &if_.else_ {
                    self.compile_stmt(&else_, intern);
                }

                // END:
                self.patch_jump(jump_to_end);
            }
            Stmt::Print(print) => {
                self.compile_expr(&print.value, intern);
                self.emit_u8(op::PRINT);
            }
            Stmt::Var(var) => {
                let name = &var.var.name;
                let value = var.value.as_ref().unwrap_or(&(Expr::Literal(ExprLiteral::Nil), 0..0));
                if self.scope_depth == 0 {
                    let (name, _) = intern.insert_str(name);
                    self.compile_expr(value, intern);
                    self.emit_u8(op::DEFINE_GLOBAL);
                    self.emit_constant(name.into());
                } else {
                    self.declare_local(name);
                    self.compile_expr(value, intern);
                    self.define_local(name);
                }
            }
            Stmt::While(while_) => {
                // START:
                let loop_start = self.start_loop();

                // Evaluate condition.
                self.compile_expr(&while_.cond, intern);
                // If the condition is false, go to END.
                let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE);
                // Discard the condition.
                self.emit_u8(op::POP);
                // Evaluate the body of the loop.
                self.compile_stmt(&while_.body, intern);
                // Go to START.
                self.emit_loop(loop_start);

                // END:
                self.patch_jump(jump_to_end);
                // Discard the condition.
                self.emit_u8(op::POP);
            }
            _ => unimplemented!(),
        }
    }

    /// Compute an expression and push it onto the stack.
    fn compile_expr(&mut self, (expr, span): &ExprS, intern: &mut Intern) {
        match expr {
            Expr::Assign(assign) => {
                self.compile_expr(&assign.value, intern);
                self.set_variable(&assign.var.name, intern);
            }
            Expr::Infix(infix) => {
                self.compile_expr(&infix.lt, intern);
                match infix.op {
                    OpInfix::Add => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::ADD);
                    }
                    OpInfix::Subtract => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::SUBTRACT);
                    }
                    OpInfix::Multiply => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::MULTIPLY);
                    }
                    OpInfix::Divide => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::DIVIDE);
                    }
                    OpInfix::Less => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::LESS);
                    }
                    OpInfix::LessEqual => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::LESS_EQUAL);
                    }
                    OpInfix::Greater => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::GREATER);
                    }
                    OpInfix::GreaterEqual => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::GREATER_EQUAL);
                    }
                    OpInfix::Equal => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::EQUAL);
                    }
                    OpInfix::NotEqual => {
                        self.compile_expr(&infix.rt, intern);
                        self.emit_u8(op::NOT_EQUAL);
                    }
                    OpInfix::LogicAnd => {
                        // If the first expression is false, go to END.
                        let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE);
                        // Otherwise, evaluate the right expression.
                        self.emit_u8(op::POP);
                        self.compile_expr(&infix.rt, intern);

                        // END:
                        // Short-circuit to the end.
                        self.patch_jump(jump_to_end);
                    }
                    OpInfix::LogicOr => {
                        // If the first expression is false, go to RIGHT_EXPR.
                        let jump_to_right_expr = self.emit_jump(op::JUMP_IF_FALSE);
                        // Otherwise, go to END.
                        let jump_to_end = self.emit_jump(op::JUMP);

                        // RIGHT_EXPR:
                        self.patch_jump(jump_to_right_expr);
                        // Discard the left value.
                        self.emit_u8(op::POP);
                        // Evaluate the right expression.
                        self.compile_expr(&infix.rt, intern);

                        // END:
                        // Short-circuit to the end.
                        self.patch_jump(jump_to_end);
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
                self.emit_u8(op::CONSTANT);
                self.emit_constant(value);
            }
            Expr::Prefix(prefix) => {
                self.compile_expr(&prefix.rt, intern);
                match prefix.op {
                    OpPrefix::Negate => self.emit_u8(op::NEGATE),
                    OpPrefix::Not => self.emit_u8(op::NOT),
                };
            }
            Expr::Var(var) => self.get_variable(&var.var.name, intern),
            _ => unimplemented!(),
        }
    }

    fn get_variable(&mut self, name: &str, intern: &mut Intern) {
        if let Some(local_idx) = self.resolve_local(name) {
            self.emit_u8(op::GET_LOCAL);
            self.emit_u8(local_idx);
        } else {
            let (name, _) = intern.insert_str(name);
            self.emit_u8(op::GET_GLOBAL);
            self.emit_constant(name.into());
        }
    }

    fn set_variable(&mut self, name: &str, intern: &mut Intern) {
        if let Some(local_idx) = self.resolve_local(name) {
            self.emit_u8(op::SET_LOCAL);
            self.emit_u8(local_idx);
        } else {
            let (name, _) = intern.insert_str(name);
            self.emit_u8(op::SET_GLOBAL);
            self.emit_constant(name.into());
        }
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

    fn define_local(&mut self, name: &str) {
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
    fn emit_jump(&mut self, opcode: u8) -> usize {
        self.emit_u8(opcode);
        self.emit_u8(0xFF);
        self.emit_u8(0xFF);
        self.chunk.ops.len() - 2
    }

    /// Takes the index of the jump offset to be patched as input, and patches
    /// it to point to the current instruction.
    fn patch_jump(&mut self, offset_idx: usize) {
        // The extra -2 is to account for the space taken by the offset.
        let offset = self.chunk.ops.len() - 2 - offset_idx;
        let offset = offset.try_into().unwrap();
        let offset = u16::to_le_bytes(offset);
        [self.chunk.ops[offset_idx], self.chunk.ops[offset_idx + 1]] = offset;
    }

    fn start_loop(&self) -> usize {
        self.chunk.ops.len()
    }

    fn emit_loop(&mut self, start_idx: usize) {
        // The extra +3 is to account for the space taken by the instruction
        // and the offset.
        let offset = self.chunk.ops.len() + 3 - start_idx;
        let offset = offset.try_into().unwrap();
        let offset = u16::to_le_bytes(offset);

        self.emit_u8(op::LOOP);
        self.emit_u8(offset[0]);
        self.emit_u8(offset[1]);
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

    fn emit_u8(&mut self, byte: u8) {
        self.chunk.write_u8(byte);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant_idx = self.chunk.write_constant(value);
        self.emit_u8(constant_idx);
    }
}

struct Local {
    name: String,
    depth: usize,
    is_initialized: bool,
}
