pub mod ast;
pub mod lexer;
pub mod parser;

use lalrpop_util::ParseError;

use crate::error::{Error, ErrorS, SyntaxError};
use crate::syntax::ast::Program;
use crate::syntax::lexer::Lexer;
use crate::syntax::parser::Parser;

pub fn is_complete(source: &str) -> bool {
    let lexer = Lexer::new(source);
    let parser = Parser::new();
    let mut errors = Vec::new();
    if let Err(e) = parser.parse(&mut errors, lexer) {
        errors.push(e);
    };
    !errors.iter().any(|e| matches!(e, ParseError::UnrecognizedEof { .. }))
}

pub fn parse(source: &str, offset: usize) -> Result<Program, Vec<ErrorS>> {
    let lexer = Lexer::new(source).map(|token| match token {
        Ok((l, token, r)) => Ok((l + offset, token, r + offset)),
        Err((e, span)) => Err((e, span.start + offset..span.end + offset)),
    });
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
        ParseError::ExtraToken { token: (start, _, end) } => (
            Error::SyntaxError(SyntaxError::ExtraToken {
                token: source[start - offset..end - offset].to_string(),
            }),
            start..end,
        ),
        ParseError::InvalidToken { location } => {
            (Error::SyntaxError(SyntaxError::InvalidToken), location..location)
        }
        ParseError::UnrecognizedEof { location, expected } => {
            (Error::SyntaxError(SyntaxError::UnrecognizedEof { expected }), location..location)
        }
        ParseError::UnrecognizedToken { token: (start, _, end), expected } => (
            Error::SyntaxError(SyntaxError::UnrecognizedToken {
                token: source[start - offset..end - offset].to_string(),
                expected,
            }),
            start..end,
        ),
        ParseError::User { error } => error,
    }));

    if errors.is_empty() { Ok(program) } else { Err(errors) }
}
