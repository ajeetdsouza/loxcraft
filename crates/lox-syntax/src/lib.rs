pub mod ast;
pub mod lexer;
pub mod parser;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;

use lalrpop_util::ParseError;
use lox_common::error::{Error, SyntaxError};

pub fn is_complete(source: &str) -> bool {
    let lexer = Lexer::new(source);
    let parser = Parser::new();
    let mut errors = Vec::new();
    if let Err(e) = parser.parse(&mut errors, lexer) {
        errors.push(e);
    };
    !errors.iter().any(|e| matches!(e, ParseError::UnrecognizedEOF { .. }))
}

pub fn parse(source: &str) -> (Program, Vec<Error>) {
    let lexer = Lexer::new(source);
    let parser = Parser::new();
    let mut errors = Vec::new();

    let mut parser_errors = Vec::new();
    let program = match parser.parse(&mut parser_errors, lexer) {
        Ok(program) => program,
        Err(err) => {
            parser_errors.push(err);
            Program::default()
        }
    };

    errors.extend(parser_errors.into_iter().map(|err| match err {
        ParseError::ExtraToken { token: (start, _, end) } => {
            Error::SyntaxError(SyntaxError::ExtraToken {
                token: source[start..end].to_string(),
                span: start..end,
            })
        }
        ParseError::InvalidToken { location } => {
            Error::SyntaxError(SyntaxError::InvalidToken { span: location..location })
        }
        ParseError::UnrecognizedEOF { location, expected } => {
            Error::SyntaxError(SyntaxError::UnrecognizedEOF { expected, span: location..location })
        }
        ParseError::UnrecognizedToken { token: (start, _, end), expected } => {
            Error::SyntaxError(SyntaxError::UnrecognizedToken {
                token: source[start..end].to_string(),
                expected,
                span: start..end,
            })
        }
        ParseError::User { error } => error,
    }));

    (program, errors)
}
