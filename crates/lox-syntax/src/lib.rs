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

// pub fn report_err(name: &str, source: &str, err: ParserError) -> io::Result<()> {
//     use codespan_reporting::diagnostic::{Diagnostic, Label};
//     use codespan_reporting::files::{Error, SimpleFile};
//     use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
//     use codespan_reporting::term::{self, Config};

//     let (label, range, notes);
//     match err {
//         ParserError::ExtraToken { token } => {
//             label = "unexpected token";
//             range = token.0..token.2;
//             notes = Vec::new();
//         }
//         ParserError::InvalidToken { location } => {
//             label = "invalid token";
//             range = location..location;
//             notes = Vec::new();
//         }
//         ParserError::UnrecognizedEOF { location, expected } => {
//             label = "unrecognized EOF";
//             range = location..location;
//             notes = vec![format!("expected one of: {} after this token", expected.join(", "))];
//         }
//         ParserError::UnrecognizedToken { token, expected } => {
//             label = "unrecognized token";
//             range = token.0..token.2;
//             notes = vec![format!("expected one of: {} after this token", expected.join(", "))];
//         }
//         ParserError::User { error: err } => {
//             label = "unexpected input";
//             range = err.location..err.location + 1;
//             notes = Vec::new();
//         }
//     };

//     let writer = StandardStream::stderr(ColorChoice::Auto);
//     let config = Config::default();
//     let file = SimpleFile::new(name, source);
//     let diagnostic = Diagnostic::error()
//         .with_message(label)
//         .with_labels(vec![Label::primary((), range)])
//         .with_notes(notes);
//     term::emit(&mut writer.lock(), &config, &file, &diagnostic).map_err(|err| match err {
//         Error::Io(err) => err,
//         _ => panic!("invalid error generated: {err}"),
//     })?;

//     Ok(())
// }
