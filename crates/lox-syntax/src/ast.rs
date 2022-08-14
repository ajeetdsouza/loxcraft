use std::fmt::{self, Display, Formatter};
use std::ops::Range;

pub type Spanned<T> = (T, Span);
pub type Span = Range<usize>;

pub type StmtS = Spanned<Stmt>;
pub type ExprS = Spanned<Expr>;

#[derive(Debug, Default)]
pub struct Program {
    pub stmts: Vec<StmtS>,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct StmtBlock {
    pub stmts: Vec<StmtS>,
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
    pub name: String,
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
    Literal(ExprLiteral),
    Infix(Box<ExprInfix>),
    Prefix(Box<ExprPrefix>),
    Variable(ExprVariable),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprAssign {
    pub name: String,
    pub value: ExprS,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprCall {
    pub callee: ExprS,
    pub args: Vec<ExprS>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExprVariable {
    pub name: String,
}
