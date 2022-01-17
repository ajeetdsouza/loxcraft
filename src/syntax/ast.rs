#[derive(Debug)]
pub enum Stmt {
    Expr(StmtExpr),
    Print(StmtPrint),
    Var(StmtVar),
}

#[derive(Debug)]
pub struct StmtExpr {
    pub expr: Expr,
}

#[derive(Debug)]
pub struct StmtPrint {
    pub expr: Expr,
}

#[derive(Debug)]
pub struct StmtVar {
    pub name: String,
    pub expr: Expr,
}

#[derive(Debug)]
pub enum Expr {
    Literal(ExprLiteral),
    Infix(Box<ExprInfix>),
    Prefix(Box<ExprPrefix>),
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
