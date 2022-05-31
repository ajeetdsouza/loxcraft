pub mod ast;
pub mod lexer;
pub mod parser;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use lalrpop_util::ParseError;

pub fn is_complete(source: &str) -> bool {
    let lexer = Lexer::new(source);
    let parser = Parser::new();
    let mut errors = Vec::new();
    if let Err(e) = parser.parse(&mut errors, lexer) {
        errors.push(e);
    };
    !errors.iter().any(|e| matches!(e, ParseError::UnrecognizedEOF { .. }))
}

pub fn parse(source: &str, errors: &mut Vec<Diagnostic<()>>) -> Program {
    let lexer = Lexer::new(source);
    let parser = Parser::new();

    let mut parser_errors = Vec::new();
    let program = match parser.parse(&mut parser_errors, lexer) {
        Ok(program) => program,
        Err(err) => {
            parser_errors.push(err);
            Program::default()
        }
    };

    errors.extend(parser_errors.into_iter().map(|err| {
        match err {
            ParseError::ExtraToken { token: (start, _, end) } => Diagnostic::error()
                .with_message(format!("extraneous input: {:?}", &source[start..end]))
                .with_labels(vec![Label::primary((), start..end)]),
            ParseError::InvalidToken { location } => Diagnostic::error()
                .with_message("invalid input")
                .with_labels(vec![Label::primary((), location..location)]),
            ParseError::UnrecognizedEOF { location, expected } => Diagnostic::error()
                .with_message("unexpected end of file")
                .with_labels(vec![Label::primary((), location..location)])
                .with_notes(vec![format!("expected: {}", one_of(&expected))]),
            ParseError::UnrecognizedToken { token: (start, _, end), expected } => {
                Diagnostic::error()
                    .with_message(format!("unexpected {:?}", &source[start..end]))
                    .with_labels(vec![Label::primary((), start..end)])
                    .with_notes(vec![format!("expected: {}", one_of(&expected))])
            }
            ParseError::User { error } => Diagnostic::error()
                .with_message(error.message.unwrap_or_else(|| {
                    format!("unexpected {:?}", &source[error.span.start..error.span.end])
                }))
                .with_labels(vec![Label::primary((), error.span)]),
        }
    }));

    program
}

fn one_of(tokens: &[String]) -> String {
    let (token_last, tokens) = match tokens.split_last() {
        Some((token_last, &[])) => return token_last.to_string(),
        Some((token_last, tokens)) => (token_last, tokens),
        None => return "nothing".to_string(),
    };

    let mut output = String::new();
    for token in tokens {
        output.push_str(token);
        output.push_str(", ");
    }
    output.push_str("or ");
    output.push_str(token_last);
    output
}
