use crate::syntax::ast::{
    Expr, ExprAssign, ExprCall, ExprInfix, ExprLiteral, ExprPrefix, ExprVariable, OpInfix,
    OpPrefix, Program, Stmt, StmtBlock, StmtExpr, StmtFor, StmtFun, StmtIf, StmtPrint, StmtReturn,
    StmtVar, StmtWhile,
};
use crate::vm::op;
use crate::vm::value::{Function, Object, Value};

use anyhow::{bail, Context, Result};

use std::mem;
use std::rc::Rc;

type CompileResult<T = ()> = Result<T>;

pub struct Compiler {
    function: Function,
    type_: FunctionType,
    locals: Vec<Local>,
    scope_depth: usize,
}

impl Compiler {
    pub fn new_script() -> Self {
        Self {
            function: Function::new("", 0),
            type_: FunctionType::Script,
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    fn new_function(name: &str, arity: usize) -> Self {
        Self {
            function: Function::new(name, arity),
            type_: FunctionType::Function,
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    pub fn compile(mut self, program: &Program) -> CompileResult<Function> {
        for stmt in &program.stmts {
            self.compile_stmt(stmt)?;
        }
        Ok(self.function)
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> CompileResult {
        match stmt {
            Stmt::Block(block) => self.compile_stmt_block(block),
            Stmt::Expr(expr) => self.compile_stmt_expr(expr),
            Stmt::For(for_) => self.compile_stmt_for(for_),
            Stmt::Fun(fun) => self.compile_stmt_fun(fun),
            Stmt::If(if_) => self.compile_stmt_if(if_),
            Stmt::Print(print) => self.compile_stmt_print(print),
            Stmt::Return(return_) => self.compile_stmt_return(return_),
            Stmt::Var(var) => self.compile_stmt_var(var),
            Stmt::While(while_) => self.compile_stmt_while(while_),
        }
    }

    fn compile_stmt_block(&mut self, block: &StmtBlock) -> CompileResult {
        self.begin_scope();
        self.compile_stmt_block_internal(block)?;
        self.end_scope();
        Ok(())
    }

    fn compile_stmt_block_internal(&mut self, block: &StmtBlock) -> CompileResult {
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    fn compile_stmt_expr(&mut self, expr: &StmtExpr) -> CompileResult {
        self.compile_expr(&expr.expr)?;
        self.emit_u8(op::POP);
        Ok(())
    }

    fn compile_stmt_for(&mut self, for_: &StmtFor) -> CompileResult {
        self.begin_scope();

        match &for_.init {
            Some(Stmt::Expr(expr)) => self.compile_stmt_expr(expr)?,
            Some(Stmt::Var(var)) => self.compile_stmt_var(var)?,
            Some(stmt) => panic!("unexpected statement type in for loop initializer: {:?}", stmt),
            None => (),
        }

        let loop_start = self.start_loop();
        let mut jump_to_end = None;

        if let Some(cond) = &for_.cond {
            self.compile_expr(cond)?;
            jump_to_end = Some(self.emit_jump(op::JUMP_IF_FALSE));
            self.emit_u8(op::POP);
        }

        self.compile_stmt(&for_.body)?;

        if let Some(incr) = &for_.incr {
            self.compile_expr(incr)?;
            self.emit_u8(op::POP);
        }

        self.emit_loop(loop_start)?;
        if let Some(jump_to_end) = jump_to_end {
            self.patch_jump(jump_to_end)?;
            self.emit_u8(op::POP);
        }

        self.end_scope();
        Ok(())
    }

    fn compile_stmt_fun(&mut self, fun: &StmtFun) -> CompileResult {
        let mut compiler = Compiler::new_function(&fun.name, fun.params.len());
        compiler.scope_depth = self.scope_depth;

        // TODO: find a cleaner way to do this
        mem::swap(&mut self.locals, &mut compiler.locals);
        compiler.begin_scope();
        compiler.add_local(&fun.name)?;

        for param in &fun.params {
            compiler.add_local(param)?;
        }
        compiler.compile_stmt_block_internal(&fun.body)?;
        compiler.emit_u8(op::NIL);
        compiler.emit_u8(op::RETURN);

        compiler.end_scope();
        mem::swap(&mut self.locals, &mut compiler.locals);

        self.emit_constant(Value::Object(Object::Function(compiler.function)))?;
        self.add_variable(&fun.name)
    }

    fn compile_stmt_if(&mut self, if_: &StmtIf) -> CompileResult {
        self.compile_expr(&if_.cond)?;
        let jump_to_else = self.emit_jump(op::JUMP_IF_FALSE);
        self.emit_u8(op::POP);

        self.compile_stmt(&if_.then)?;
        let jump_to_end = self.emit_jump(op::JUMP);

        self.patch_jump(jump_to_else)?;
        self.emit_u8(op::POP);
        if let Some(else_) = &if_.else_ {
            self.compile_stmt(else_)?;
        }

        self.patch_jump(jump_to_end)?;
        Ok(())
    }

    fn compile_stmt_print(&mut self, print: &StmtPrint) -> CompileResult {
        self.compile_expr(&print.expr)?;
        self.emit_u8(op::PRINT);
        Ok(())
    }

    fn compile_stmt_return(&mut self, return_: &StmtReturn) -> CompileResult {
        if self.type_ == FunctionType::Script {
            bail!("cannot return outside function");
        }

        self.compile_expr(&return_.expr)?;
        self.emit_u8(op::RETURN);
        Ok(())
    }

    fn compile_stmt_var(&mut self, var: &StmtVar) -> CompileResult {
        self.compile_expr(&var.value)?;
        self.add_variable(&var.name)
    }

    fn compile_stmt_while(&mut self, while_: &StmtWhile) -> CompileResult {
        let loop_start = self.start_loop();

        self.compile_expr(&while_.cond)?;

        let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE);
        self.emit_u8(op::POP);
        self.compile_stmt(&while_.body)?;
        self.emit_loop(loop_start)?;

        self.patch_jump(jump_to_end)?;
        self.emit_u8(op::POP);

        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> CompileResult {
        match expr {
            Expr::Assign(assign) => self.compile_expr_assign(assign),
            Expr::Call(call) => self.compile_expr_call(call),
            Expr::Literal(literal) => self.compile_expr_literal(literal),
            Expr::Infix(infix) => self.compile_expr_infix(infix),
            Expr::Prefix(prefix) => self.compile_expr_prefix(prefix),
            Expr::Variable(variable) => self.compile_expr_variable(variable),
        }
    }

    fn compile_expr_assign(&mut self, assign: &ExprAssign) -> CompileResult {
        self.compile_expr(&assign.value)?;
        if let Some(idx) = self.resolve_local(&assign.name)? {
            self.emit_u8(op::SET_LOCAL);
            self.emit_u8(idx);
            return Ok(());
        }

        self.emit_u8(op::SET_GLOBAL);
        let name = Value::Object(Object::String(Rc::new(assign.name.to_string())));
        let idx = self.make_constant(name)?;
        self.emit_u8(idx);
        Ok(())
    }

    fn compile_expr_call(&mut self, call: &ExprCall) -> CompileResult {
        self.compile_expr(&call.callee)?;

        let arg_count =
            u8::try_from(call.args.len()).context("too many arguments in function call")?;
        for arg in &call.args {
            self.compile_expr(arg)?;
        }

        self.emit_u8(op::CALL);
        self.emit_u8(arg_count);

        Ok(())
    }

    fn compile_expr_literal(&mut self, expr: &ExprLiteral) -> CompileResult {
        match expr {
            ExprLiteral::Nil => self.emit_u8(op::NIL),
            ExprLiteral::Bool(false) => self.emit_u8(op::FALSE),
            ExprLiteral::Bool(true) => self.emit_u8(op::TRUE),
            ExprLiteral::Number(number) => {
                let value = Value::Number(*number);
                self.emit_constant(value)?;
            }
            ExprLiteral::String(string) => {
                let object = Object::String(Rc::new(string.to_string()));
                let value = Value::Object(object);
                self.emit_constant(value)?;
            }
        };
        Ok(())
    }

    fn compile_expr_infix(&mut self, expr: &ExprInfix) -> CompileResult {
        match expr.op {
            OpInfix::LogicOr => {
                self.compile_expr(&expr.lt)?;
                let jump_to_else = self.emit_jump(op::JUMP_IF_FALSE);
                let jump_to_end = self.emit_jump(op::JUMP);

                self.patch_jump(jump_to_else)?;
                self.emit_u8(op::POP);
                self.compile_expr(&expr.rt)?;

                self.patch_jump(jump_to_end)?;
            }
            OpInfix::LogicAnd => {
                self.compile_expr(&expr.lt)?;
                let jump_to_end = self.emit_jump(op::JUMP_IF_FALSE);
                self.emit_u8(op::POP);
                self.compile_expr(&expr.rt)?;

                self.patch_jump(jump_to_end)?;
            }
            OpInfix::Equal => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::EQUAL);
            }
            OpInfix::NotEqual => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::EQUAL);
                self.emit_u8(op::NOT);
            }
            OpInfix::Greater => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::GREATER);
            }
            OpInfix::GreaterEqual => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::LESS);
                self.emit_u8(op::NOT);
            }
            OpInfix::Less => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::LESS);
            }
            OpInfix::LessEqual => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::GREATER);
                self.emit_u8(op::NOT);
            }
            OpInfix::Add => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::ADD);
            }
            OpInfix::Subtract => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::SUBTRACT)
            }
            OpInfix::Multiply => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::MULTIPLY);
            }
            OpInfix::Divide => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_u8(op::DIVIDE);
            }
        };

        Ok(())
    }

    fn compile_expr_prefix(&mut self, expr: &ExprPrefix) -> CompileResult {
        self.compile_expr(&expr.expr)?;

        match expr.op {
            OpPrefix::Negate => self.emit_u8(op::NEGATE),
            OpPrefix::Not => self.emit_u8(op::NOT),
        };

        Ok(())
    }

    fn compile_expr_variable(&mut self, variable: &ExprVariable) -> CompileResult {
        if let Some(idx) = self.resolve_local(&variable.name)? {
            self.emit_u8(op::GET_LOCAL);
            self.emit_u8(idx);
            return Ok(());
        }

        self.emit_u8(op::GET_GLOBAL);
        let name = Value::Object(Object::String(Rc::new(variable.name.to_string())));
        let idx = self.make_constant(name)?;
        self.emit_u8(idx);
        Ok(())
    }

    fn emit_u8(&mut self, value: u8) {
        self.function.chunk.code.push(value);
    }

    fn emit_constant(&mut self, value: Value) -> CompileResult {
        self.emit_u8(op::CONSTANT);
        let idx = self.make_constant(value)?;
        self.emit_u8(idx);
        Ok(())
    }

    fn make_constant(&mut self, value: Value) -> CompileResult<u8> {
        let idx = u8::try_from(self.function.chunk.constants.len())
            .ok()
            .context("cannot define more than 256 constants within a chunk")?;
        self.function.chunk.constants.push(value);
        Ok(idx)
    }

    fn emit_jump(&mut self, op: u8) -> usize {
        self.emit_u8(op);
        let jump = self.function.chunk.code.len();

        self.emit_u8(0xFF);
        self.emit_u8(0xFF);

        jump
    }

    fn patch_jump(&mut self, offset: usize) -> CompileResult {
        // -2 to adjust for the bytecode for the jump offset itself.
        let jump = self.function.chunk.code.len() - offset - 2;
        let jump = u16::try_from(jump).context("jump offset too large")?;

        self.function.chunk.code[offset] = (jump >> 8) as u8;
        self.function.chunk.code[offset + 1] = jump as u8;

        Ok(())
    }

    fn start_loop(&self) -> usize {
        self.function.chunk.code.len()
    }

    fn emit_loop(&mut self, loop_start: usize) -> CompileResult {
        self.emit_u8(op::LOOP);

        let offset = self.function.chunk.code.len() - loop_start + 2;
        let offset = u16::try_from(offset).context("loop offset too large")?;

        self.emit_u8((offset >> 8) as u8);
        self.emit_u8(offset as u8);

        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        debug_assert!(self.scope_depth > 0);
        self.scope_depth -= 1;

        while self.locals.last().map(|local| local.depth > self.scope_depth).unwrap_or(false) {
            self.locals.pop();
            self.emit_u8(op::POP);
        }
    }

    fn add_variable(&mut self, name: &str) -> CompileResult {
        if self.scope_depth > 0 {
            for local in self.locals.iter().rev() {
                if local.depth < self.scope_depth {
                    break;
                }
                if local.name == name {
                    bail!("'{}' has already been defined in this scope", name);
                }
            }
            return self.add_local(name);
        }

        self.emit_u8(op::DEFINE_GLOBAL);
        let name = Value::Object(Object::String(Rc::new(name.to_string())));
        let idx = self.make_constant(name)?;
        self.emit_u8(idx);

        Ok(())
    }

    fn add_local(&mut self, name: &str) -> CompileResult {
        if self.locals.len() >= 256 {
            bail!("cannot define more than 256 local variables within a chunk");
        }
        self.locals.push(Local { name: name.to_string(), depth: self.scope_depth });
        Ok(())
    }

    fn resolve_local(&self, name: &str) -> Result<Option<u8>> {
        for (idx, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                let idx = idx.try_into().context("more than 256 local variables were defined")?;
                return Ok(Some(idx));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum FunctionType {
    Function,
    Script,
}

#[derive(Clone, Debug)]
struct Local {
    name: String,
    depth: usize,
}
