#[derive(Debug)]
pub enum Expr {
    Literal(Literal),
    Infix(Box<InfixExpr>),
    Prefix(Box<PrefixExpr>),
}

#[derive(Debug)]
pub enum Literal {
    Nil,
    Bool(bool),
    String(String),
    Number(f64),
}

#[derive(Debug)]
pub struct InfixExpr {
    lt: Expr,
    op: InfixOp,
    rt: Expr,
}

#[derive(Debug)]
pub enum InfixOp {
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
pub struct PrefixExpr {
    op: PrefixOp,
    expr: Expr,
}

#[derive(Debug)]
pub enum PrefixOp {
    Negate,
    Not,
}
