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

pub trait Visitor {
    type Result;

    fn visit_expr(&mut self, expr: &Expr) -> Self::Result {
        match expr {
            Expr::Literal(literal) => self.visit_expr_literal(literal),
            Expr::Infix(infix) => self.visit_expr_infix(infix),
            Expr::Prefix(prefix) => self.visit_expr_prefix(prefix),
        }
    }

    fn visit_expr_literal(&mut self, expr: &ExprLiteral) -> Self::Result;
    fn visit_expr_infix(&mut self, expr: &ExprInfix) -> Self::Result;
    fn visit_expr_prefix(&mut self, expr: &ExprPrefix) -> Self::Result;
}
