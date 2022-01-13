enum Expr {
    Literal(Literal),
    Infix(Box<InfixExpr>),
    Prefix(Box<PrefixExpr>),
}

struct Literal {
    name: String,
}

struct InfixExpr {
    lt: Expr,
    op: InfixOp,
    rt: Expr,
}

enum InfixOp {
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

struct PrefixExpr {
    op: PrefixOp,
    expr: Expr,
}

enum PrefixOp {
    Negate,
}
