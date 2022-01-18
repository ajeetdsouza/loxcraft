#[derive(Debug)]
pub enum Stmt {
    Block(StmtBlock),
    Expr(StmtExpr),
    For(Box<StmtFor>),
    If(Box<StmtIf>),
    Print(StmtPrint),
    Var(StmtVar),
    While(Box<StmtWhile>),
}

#[derive(Debug)]
pub struct StmtBlock {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub struct StmtExpr {
    pub expr: Expr,
}

#[derive(Debug)]
pub struct StmtFor {
    pub init: Option<Stmt>,
    pub cond: Option<Expr>,
    pub incr: Option<Expr>,
    pub body: Stmt,
}

#[derive(Debug)]
pub struct StmtIf {
    pub cond: Expr,
    pub then: Stmt,
    pub else_: Option<Stmt>,
}

#[derive(Debug)]
pub struct StmtPrint {
    pub expr: Expr,
}

#[derive(Debug)]
pub struct StmtVar {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug)]
pub struct StmtWhile {
    pub cond: Expr,
    pub body: Stmt,
}

#[derive(Debug)]
pub enum Expr {
    Assign(Box<ExprAssign>),
    Literal(ExprLiteral),
    Infix(Box<ExprInfix>),
    Prefix(Box<ExprPrefix>),
    Variable(ExprVariable),
}

#[derive(Debug)]
pub struct ExprAssign {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug)]
pub enum ExprLiteral {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Debug)]
pub struct ExprInfix {
    pub lt: Expr,
    pub op: OpInfix,
    pub rt: Expr,
}

#[derive(Debug)]
pub enum OpInfix {
    LogicOr,
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

#[derive(Debug)]
pub struct ExprPrefix {
    pub op: OpPrefix,
    pub expr: Expr,
}

#[derive(Debug)]
pub enum OpPrefix {
    Negate,
    Not,
}

#[derive(Debug)]
pub struct ExprVariable {
    pub name: String,
}
