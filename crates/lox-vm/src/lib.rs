#![allow(clippy::new_without_default, clippy::module_inception)]

mod chunk;
pub mod compiler;
mod op;
mod value;
pub mod vm;

use lox_syntax::parser::ParserError;

use std::io::Write;

pub fn report_err<W: Write>(source: &str, err: ParserError, mut writer: W) {
    use codespan_reporting::diagnostic::{Diagnostic, Label};
    use codespan_reporting::files::SimpleFile;
    use codespan_reporting::term;

    let (label, range, notes);
    match err {
        ParserError::ExtraToken { token } => {
            label = "unexpected token";
            range = token.0..token.2;
            notes = Vec::new();
        }
        ParserError::InvalidToken { location } => {
            label = "invalid token";
            range = location..location;
            notes = Vec::new();
        }
        ParserError::UnrecognizedEOF { location, expected } => {
            label = "unrecognized EOF";
            range = location..location;
            notes = vec![format!("expected one of: {} after this token", expected.join(", "))];
        }
        ParserError::UnrecognizedToken { token, expected } => {
            label = "unrecognized token";
            range = token.0..token.2;
            notes = vec![format!("expected one of: {} after this token", expected.join(", "))];
        }
        ParserError::User { error: err } => {
            label = "unexpected input";
            range = err.location..err.location + 1;
            notes = Vec::new();
        }
    };

    let mut buffer = termcolor::Buffer::ansi();
    let config = term::Config::default();
    let file = SimpleFile::new("<script>", source);
    let diagnostic = Diagnostic::error()
        .with_message(label)
        .with_labels(vec![Label::primary((), range)])
        .with_notes(notes);
    term::emit(&mut buffer, &config, &file, &diagnostic).unwrap();

    writer.write_all(&buffer.into_inner()).unwrap();
}
