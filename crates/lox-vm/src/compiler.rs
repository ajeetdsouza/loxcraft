use crate::op::{ArgCount, ConstantIdx, JumpOffset, Op, StackIdx};
use crate::value::{Function, Value};
use lox_syntax::ast::{
    Expr, ExprAssign, ExprCall, ExprInfix, ExprLiteral, ExprPrefix, ExprVariable, OpInfix,
    OpPrefix, Program, Stmt, StmtBlock, StmtExpr, StmtFor, StmtFun, StmtIf, StmtPrint, StmtReturn,
    StmtVar, StmtWhile,
};

use anyhow::{bail, Context, Result};

use std::mem;
use std::rc::Rc;

type CompileResult<T = ()> = Result<T>;

#[derive(Debug)]
pub struct Compiler {
    ctx: CompilerCtx,
    scope_depth: usize,
}

impl Compiler {
    /// Creates a compiler for a new script.
    pub fn new() -> Self {
        Self {
            ctx: CompilerCtx {
                function: Function::new("", 0),
                type_: FunctionType::Script,
                locals: Vec::new(),
                parent: None,
            },
            scope_depth: 0,
        }
    }

    pub fn compile(mut self, program: &Program) -> CompileResult<Function> {
        for stmt in &program.stmts {
            self.compile_stmt(stmt)?;
        }
        Ok(self.ctx.function)
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
        self.compile_expr(&expr.value)?;
        self.emit_op(Op::Pop);
        Ok(())
    }

    fn compile_stmt_for(&mut self, for_: &StmtFor) -> CompileResult {
        self.begin_scope();

        // Evaluate init statement. This may be an expression, a variable
        // assignment, or nothing at all.
        match &for_.init {
            Some(Stmt::Expr(expr)) => self.compile_stmt_expr(expr)?,
            Some(Stmt::Var(var)) => self.compile_stmt_var(var)?,
            Some(stmt) => panic!("unexpected statement type in for loop initializer: {:?}", stmt),
            None => (),
        }

        // START:
        let loop_start = self.start_loop();

        // Evaluate the condition, if it exists.
        let mut jump_to_end = None;
        if let Some(cond) = &for_.cond {
            self.compile_expr(cond)?;
            // If the condition is false, go to END.
            jump_to_end = Some(self.emit_jump(Op::JumpIfFalse));
            // Discard the condition.
            self.emit_op(Op::Pop);
        }

        // Evaluate the body.
        self.compile_stmt(&for_.body)?;

        // Evaluate the increment expression, if it exists.
        if let Some(incr) = &for_.incr {
            self.compile_expr(incr)?;
            // Discard the result of the expression.
            self.emit_op(Op::Pop);
        }

        // Go to START.
        self.emit_loop(loop_start)?;
        // END:
        if let Some(jump_to_end) = jump_to_end {
            self.patch_jump(jump_to_end)?;
            // Discard the condition.
            self.emit_op(Op::Pop);
        }

        self.end_scope();
        Ok(())
    }

    fn compile_stmt_fun(&mut self, fun: &StmtFun) -> CompileResult {
        let ctx = CompilerCtx {
            function: Function::new(&fun.name, fun.params.len()),
            type_: FunctionType::Function,
            locals: Vec::new(),
            parent: None,
        };
        self.begin_ctx(ctx);
        self.create_local(&fun.name)?;
        for param in &fun.params {
            self.create_local(param)?;
        }
        self.compile_stmt_block(&fun.body)?;
        // Implicit return at the end of the function.
        self.compile_stmt_return(&StmtReturn { value: None })?;
        let function = self.end_ctx();

        let constant_idx = self.create_constant(Value::Function(Rc::new(function)))?;
        self.emit_op(Op::Closure(constant_idx));
        self.create_variable(&fun.name)?;

        Ok(())
    }

    fn compile_stmt_if(&mut self, if_: &StmtIf) -> CompileResult {
        self.compile_expr(&if_.cond)?;
        // If the condition is false, go to ELSE.
        let jump_to_else = self.emit_jump(Op::JumpIfFalse);
        // Discard the condition.
        self.emit_op(Op::Pop);
        // Evaluate the if branch.
        self.compile_stmt(&if_.then)?;
        // Go to END.
        let jump_to_end = self.emit_jump(Op::Jump);

        // ELSE:
        self.patch_jump(jump_to_else)?;
        self.emit_op(Op::Pop); // Discard the condition.
        if let Some(else_) = &if_.else_ {
            self.compile_stmt(else_)?;
        }

        // END:
        self.patch_jump(jump_to_end)?;

        Ok(())
    }

    fn compile_stmt_print(&mut self, print: &StmtPrint) -> CompileResult {
        self.compile_expr(&print.value)?;
        self.emit_op(Op::Print);
        Ok(())
    }

    fn compile_stmt_return(&mut self, return_: &StmtReturn) -> CompileResult {
        if self.ctx.type_ == FunctionType::Script {
            bail!("cannot return outside function");
        }
        self.compile_expr(return_.value.as_ref().unwrap_or(&Expr::Literal(ExprLiteral::Nil)))?;
        self.emit_op(Op::Return);
        Ok(())
    }

    fn compile_stmt_var(&mut self, var: &StmtVar) -> CompileResult {
        // Push the value to the stack. This will either be popped and added to
        // the globals map, or it will be used directly from the stack as a
        // local variable.
        self.compile_expr(var.value.as_ref().unwrap_or(&Expr::Literal(ExprLiteral::Nil)))?;
        self.create_variable(&var.name)
    }

    fn compile_stmt_while(&mut self, while_: &StmtWhile) -> CompileResult {
        // START:
        let loop_start = self.start_loop();

        // Evaluate condition.
        self.compile_expr(&while_.cond)?;
        // If the condition is false, go to END.
        let jump_to_end = self.emit_jump(Op::JumpIfFalse);
        // Discard the condition.
        self.emit_op(Op::Pop);
        // Evaluate the body of the loop.
        self.compile_stmt(&while_.body)?;
        // Go to START.
        self.emit_loop(loop_start)?;

        // END:
        self.patch_jump(jump_to_end)?;
        // Discard the condition.
        self.emit_op(Op::Pop);

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
            self.emit_op(Op::SetLocal(idx));
            return Ok(());
        }

        let name = Value::String(Rc::new(assign.name.to_string()));
        let idx = self.create_constant(name)?;
        self.emit_op(Op::SetGlobal(idx));
        Ok(())
    }

    fn compile_expr_call(&mut self, call: &ExprCall) -> CompileResult {
        self.compile_expr(&call.callee)?;

        let arg_count =
            ArgCount::try_from(call.args.len()).context("too many arguments in function call")?;
        for arg in &call.args {
            // Push all arguments to the stack. The function treats them as
            // locals.
            self.compile_expr(arg)?;
        }

        self.emit_op(Op::Call(arg_count));
        Ok(())
    }

    fn compile_expr_literal(&mut self, expr: &ExprLiteral) -> CompileResult {
        match expr {
            ExprLiteral::Nil => self.emit_op(Op::Nil),
            ExprLiteral::Bool(false) => self.emit_op(Op::False),
            ExprLiteral::Bool(true) => self.emit_op(Op::True),
            ExprLiteral::Number(number) => {
                let value = Value::Number(*number);
                let constant = self.create_constant(value)?;
                self.emit_op(Op::Constant(constant));
            }
            ExprLiteral::String(string) => {
                let value = Value::String(Rc::new(string.to_string()));
                let constant = self.create_constant(value)?;
                self.emit_op(Op::Constant(constant));
            }
        };
        Ok(())
    }

    fn compile_expr_infix(&mut self, expr: &ExprInfix) -> CompileResult {
        match expr.op {
            OpInfix::LogicOr => {
                self.compile_expr(&expr.lt)?;
                // If the first expression is false, go to RIGHT_EXPR.
                let jump_to_right_expr = self.emit_jump(Op::JumpIfFalse);
                // Otherwise, go to END.
                let jump_to_end = self.emit_jump(Op::Jump);

                // RIGHT_EXPR:
                self.patch_jump(jump_to_right_expr)?;
                // Discard the left value.
                self.emit_op(Op::Pop);
                // Evaluate the right expression.
                self.compile_expr(&expr.rt)?;

                // END:
                // Short-circuit to the end.
                self.patch_jump(jump_to_end)?;
            }
            OpInfix::LogicAnd => {
                self.compile_expr(&expr.lt)?;
                // If the first expression is false, go to END.
                let jump_to_end = self.emit_jump(Op::JumpIfFalse);
                // Otherwise, evaluate the right expression.
                self.emit_op(Op::Pop);
                self.compile_expr(&expr.rt)?;

                // END:
                // Short-circuit to the end.
                self.patch_jump(jump_to_end)?;
            }
            OpInfix::Equal => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Equal);
            }
            OpInfix::NotEqual => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Equal);
                self.emit_op(Op::Not);
            }
            OpInfix::Greater => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Greater);
            }
            // "Greater or equal to" is equivalent to "not less than".
            OpInfix::GreaterEqual => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Less);
                self.emit_op(Op::Not);
            }
            OpInfix::Less => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Less);
            }
            // "Less or equal to" is equivalent to "not greater than".
            OpInfix::LessEqual => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Greater);
                self.emit_op(Op::Not);
            }
            OpInfix::Add => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Add);
            }
            OpInfix::Subtract => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Subtract)
            }
            OpInfix::Multiply => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Multiply);
            }
            OpInfix::Divide => {
                self.compile_expr(&expr.lt)?;
                self.compile_expr(&expr.rt)?;
                self.emit_op(Op::Divide);
            }
        };

        Ok(())
    }

    fn compile_expr_prefix(&mut self, expr: &ExprPrefix) -> CompileResult {
        self.compile_expr(&expr.rt)?;
        match expr.op {
            OpPrefix::Negate => self.emit_op(Op::Negate),
            OpPrefix::Not => self.emit_op(Op::Not),
        };
        Ok(())
    }

    fn compile_expr_variable(&mut self, variable: &ExprVariable) -> CompileResult {
        if let Some(idx) = self.resolve_local(&variable.name)? {
            self.emit_op(Op::GetLocal(idx));
            return Ok(());
        }

        let name = Value::String(Rc::new(variable.name.to_string()));
        let idx = self.create_constant(name)?;
        self.emit_op(Op::GetGlobal(idx));
        Ok(())
    }

    // Writes an Op to the bytecode of the current function.
    fn emit_op(&mut self, op: Op) {
        self.ctx.function.chunk.code.push(op);
    }

    /// Creates a new constant in the current function's constant pool and
    /// returns its index. If an identical constant already exists in the pool,
    /// the index of that constant is returned instead.
    fn create_constant(&mut self, value: Value) -> CompileResult<ConstantIdx> {
        let idx = match self.ctx.function.chunk.constants.iter().position(|c| c == &value) {
            Some(idx) => idx,
            None => {
                let idx = self.ctx.function.chunk.constants.len();
                self.ctx.function.chunk.constants.push(value);
                idx
            }
        };
        ConstantIdx::try_from(idx)
            .ok()
            .context("cannot define more than 256 constants within a chunk")
    }

    /// Emits a blank jump instruction and returns its location. The location is
    /// later used to patch the jump instruction.
    fn emit_jump(&mut self, op: fn(JumpOffset) -> Op) -> usize {
        let location = self.ctx.function.chunk.code.len();
        self.emit_op(op(JumpOffset::MAX));
        location
    }

    /// Patches the jump instruction at the given location to jump to the
    /// current instruction.
    fn patch_jump(&mut self, location: usize) -> CompileResult {
        let offset = self.ctx.function.chunk.code.len() - location - 1;
        let offset = JumpOffset::try_from(offset).context("jump offset too large")?;
        match &mut self.ctx.function.chunk.code[location] {
            Op::Jump(to) | Op::JumpIfFalse(to) => *to = offset,
            _ => panic!("tried to patch an op that is not a jump"),
        };
        Ok(())
    }

    /// Marks the starting position of a loop.
    fn start_loop(&self) -> usize {
        self.ctx.function.chunk.code.len()
    }

    /// Jumps to the starting position of a loop.
    fn emit_loop(&mut self, loop_start: usize) -> CompileResult {
        // +1 to the offset to skip over the loop instruction itself.
        let offset = self.ctx.function.chunk.code.len() - loop_start + 1;
        let offset = JumpOffset::try_from(offset).context("loop offset too large")?;
        self.emit_op(Op::Loop(offset));
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
        while self.ctx.locals.last().map(|l| l.depth > self.scope_depth).unwrap_or(false) {
            // Remove locals that are no longer in scope.
            self.ctx.locals.pop();
            self.emit_op(Op::Pop);
        }
    }

    /// Pushes the current ctx to parent and assigns it to the given ctx.
    fn begin_ctx(&mut self, ctx: CompilerCtx) {
        let ctx = mem::replace(&mut self.ctx, ctx);
        self.ctx.parent = Some(Box::new(ctx));
    }

    /// Pops the current ctx and extracts a [`Function`] from it.
    fn end_ctx(&mut self) -> Function {
        let parent = self.ctx.parent.take().expect("tried to end context in a script");
        let ctx = mem::replace(&mut self.ctx, *parent);
        ctx.function
    }

    /// Creates a local or global variable (based on the current scope) out of
    /// the top value of the stack.
    fn create_variable(&mut self, name: &str) -> CompileResult {
        // If the scope depth is 0, create a global variable.
        if self.scope_depth == 0 {
            let name = Value::String(Rc::new(name.to_string()));
            let name = self.create_constant(name)?;
            self.emit_op(Op::DefineGlobal(name));
            return Ok(());
        }

        // Otherwise, create a local variable.
        self.create_local(name)
    }

    /// Creates a named local variable out of the top value of the stack.
    fn create_local(&mut self, name: &str) -> CompileResult {
        if self
            .ctx
            .locals
            .iter()
            .rev()
            .take_while(|l| l.depth >= self.scope_depth)
            .any(|l| l.name == name)
        {
            bail!("'{name}' has already been defined in this scope");
        }

        self.ctx.locals.push(Local::new(name, self.scope_depth));
        Ok(())
    }

    /// Finds the index of the local variable with the given name and the
    /// maximum scope depth. If not available, returns [`None`].
    fn resolve_local(&self, name: &str) -> Result<Option<StackIdx>> {
        let idx = match self.ctx.locals.iter().rposition(|l| l.name == name) {
            Some(idx) => Some(
                StackIdx::try_from(idx).context("more than 256 local variables were defined")?,
            ),
            None => None,
        };
        Ok(idx)
    }
}

#[derive(Debug)]
struct CompilerCtx {
    function: Function,
    type_: FunctionType,
    locals: Vec<Local>,
    parent: Option<Box<CompilerCtx>>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum FunctionType {
    /// A function that has been defined in code.
    Function,
    /// The global-level function that is called when the program starts.
    Script,
}

#[derive(Clone, Debug)]
struct Local {
    /// The name of the variable.
    name: String,
    /// The scope depth of the variable, i.e. the number of nested scopes that
    /// surround it. This starts at 1, because global scopes don't have local
    /// variables.
    depth: usize,
}

impl Local {
    fn new<S: Into<String>>(name: S, depth: usize) -> Self {
        Self { name: name.into(), depth }
    }
}
