#[derive(Debug)]
pub enum Expr {
    Literal(Literal),
    Infix(Box<ExprInfix>),
    Prefix(Box<ExprPrefix>),
}

#[derive(Debug)]
pub enum Literal {
    Nil,
    Bool(bool),
    String(String),
    Number(f64),
}

#[derive(Debug)]
pub struct ExprInfix {
    pub lt: Expr,
    pub op: OpInfix,
    pub rt: Expr,
}

#[derive(Debug)]
pub enum OpInfix {
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
