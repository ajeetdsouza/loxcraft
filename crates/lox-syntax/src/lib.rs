pub mod ast;
pub mod lexer;
pub mod parser;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::{Parser, ParserError};

pub fn parse(source: &str) -> Result<Program, ParserError> {
    let lexer = Lexer::new(source);
    let parser = Parser::new();
    parser.parse(lexer.into_iter())
}
