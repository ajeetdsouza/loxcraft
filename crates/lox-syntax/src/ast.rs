use lox_common::types::Span;

use std::fmt::{self, Display, Formatter};

pub type Spanned<T> = (T, Span);
pub type StmtS = Spanned<Stmt>;
pub type ExprS = Spanned<Expr>;

#[derive(Debug, Default)]
pub struct Program {
    pub stmts: Vec<StmtS>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Block(StmtBlock),
    Class(StmtClass),
    Expr(StmtExpr),
    For(Box<StmtFor>),
    Fun(StmtFun),
    If(Box<StmtIf>),
    Print(StmtPrint),
    Return(StmtReturn),
    Var(StmtVar),
    While(Box<StmtWhile>),
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtBlock {
    pub stmts: Vec<StmtS>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtClass {
    pub name: String,
    pub super_: Option<ExprS>,
    pub methods: Vec<Spanned<StmtFun>>,
}

/// An expression statement evaluates an expression and discards the result.
#[derive(Clone, Debug, PartialEq)]
pub struct StmtExpr {
    pub value: ExprS,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtFor {
    pub init: Option<StmtS>,
    pub cond: Option<ExprS>,
    pub incr: Option<ExprS>,
    pub body: StmtS,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtFun {
    pub name: String,
    pub params: Vec<String>,
    pub body: StmtBlock,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtIf {
    pub cond: ExprS,
    pub then: StmtS,
    pub else_: Option<StmtS>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtPrint {
    pub value: ExprS,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtReturn {
    pub value: Option<ExprS>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtVar {
    pub var: Var,
    pub value: Option<ExprS>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtWhile {
    pub cond: ExprS,
    pub body: StmtS,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Assign(Box<ExprAssign>),
    Call(Box<ExprCall>),
    Get(Box<ExprGet>),
    Literal(ExprLiteral),
    Infix(Box<ExprInfix>),
    Prefix(Box<ExprPrefix>),
    Set(Box<ExprSet>),
    Super(ExprSuper),
    Var(ExprVar),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprAssign {
    pub var: Var,
    pub value: ExprS,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprCall {
    pub callee: ExprS,
    pub args: Vec<ExprS>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprGet {
    pub object: ExprS,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprLiteral {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprInfix {
    pub lt: ExprS,
    pub op: OpInfix,
    pub rt: ExprS,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

impl Display for OpInfix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let op = match self {
            OpInfix::Add => "+",
            OpInfix::Subtract => "-",
            OpInfix::Multiply => "*",
            OpInfix::Divide => "/",
            OpInfix::Less => "<",
            OpInfix::LessEqual => "<=",
            OpInfix::Greater => ">",
            OpInfix::GreaterEqual => ">=",
            OpInfix::Equal => "==",
            OpInfix::NotEqual => "!=",
            OpInfix::LogicAnd => "and",
            OpInfix::LogicOr => "or",
        };
        write!(f, "{}", op)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprPrefix {
    pub op: OpPrefix,
    pub rt: ExprS,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OpPrefix {
    Negate,
    Not,
}

impl Display for OpPrefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let op = match self {
            OpPrefix::Negate => "-",
            OpPrefix::Not => "!",
        };
        write!(f, "{}", op)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprSet {
    pub object: ExprS,
    pub name: String,
    pub value: ExprS,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprSuper {
    pub super_: Var,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExprVar {
    pub var: Var,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Var {
    pub name: String,
    /// This field is initialized as [`None`] by the parser, and is later filled
    /// by the resolver.
    pub depth: Option<usize>,
}
