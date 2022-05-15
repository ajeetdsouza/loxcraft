use std::ops::Range;

pub type Spanned<T> = (T, Span);
pub type Span = Range<usize>;

#[derive(Debug, Default)]
pub struct Program {
    pub stmts: Vec<Spanned<Stmt>>,
}

#[derive(Debug, PartialEq)]
pub enum Stmt {
    Block(StmtBlock),
    Expr(StmtExpr),
    For(Box<StmtFor>),
    Fun(Box<StmtFun>),
    If(Box<StmtIf>),
    Print(StmtPrint),
    Return(StmtReturn),
    Var(StmtVar),
    While(Box<StmtWhile>),
    Error,
}

#[derive(Debug, PartialEq)]
pub struct StmtBlock {
    pub stmts: Vec<Spanned<Stmt>>,
}

/// An expression statement evaluates an expression and discards the result.
#[derive(Debug, PartialEq)]
pub struct StmtExpr {
    pub value: Expr,
}

#[derive(Debug, PartialEq)]
pub struct StmtFor {
    pub init: Option<Stmt>,
    pub cond: Option<Expr>,
    pub incr: Option<Expr>,
    pub body: Spanned<Stmt>,
}

#[derive(Debug, PartialEq)]
pub struct StmtFun {
    pub name: String,
    pub params: Vec<String>,
    pub body: StmtBlock,
}

#[derive(Debug, PartialEq)]
pub struct StmtIf {
    pub cond: Expr,
    pub then: Spanned<Stmt>,
    pub else_: Option<Spanned<Stmt>>,
}

#[derive(Debug, PartialEq)]
pub struct StmtPrint {
    pub value: Expr,
}

#[derive(Debug, PartialEq)]
pub struct StmtReturn {
    pub value: Option<Expr>,
}

#[derive(Debug, PartialEq)]
pub struct StmtVar {
    pub name: String,
    pub value: Option<Expr>,
}

#[derive(Debug, PartialEq)]
pub struct StmtWhile {
    pub cond: Expr,
    pub body: Spanned<Stmt>,
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Assign(Box<ExprAssign>),
    Call(Box<ExprCall>),
    Literal(ExprLiteral),
    Infix(Box<ExprInfix>),
    Prefix(Box<ExprPrefix>),
    Variable(ExprVariable),
}

#[derive(Debug, PartialEq)]
pub struct ExprAssign {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, PartialEq)]
pub struct ExprCall {
    pub callee: Expr,
    pub args: Vec<Expr>,
}

#[derive(Debug, PartialEq)]
pub enum ExprLiteral {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Debug, PartialEq)]
pub struct ExprInfix {
    pub lt: Expr,
    pub op: OpInfix,
    pub rt: Expr,
}

#[derive(Debug, Eq, PartialEq)]
pub enum OpInfix {
    /// Short-circuiting logical OR.
    LogicOr,
    /// Short-circuiting logical AND.
    LogicAnd,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, PartialEq)]
pub struct ExprPrefix {
    pub op: OpPrefix,
    pub rt: Expr,
}

#[derive(Debug, Eq, PartialEq)]
pub enum OpPrefix {
    Negate,
    Not,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ExprVariable {
    pub name: String,
}
