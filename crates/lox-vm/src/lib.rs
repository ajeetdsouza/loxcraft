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

    let file = SimpleFile::new("<script>", source);
    let diagnostic = match err {
        ParserError::ExtraToken { token: (start, _, end) } => Diagnostic::error()
            .with_message(format!("extraneous input: {:?}", &source[start..end]))
            .with_labels(vec![Label::primary((), start..end)]),
        ParserError::InvalidToken { location } => Diagnostic::error()
            .with_message("invalid input")
            .with_labels(vec![Label::primary((), location..location)]),
        ParserError::UnrecognizedEOF { location, expected } => Diagnostic::error()
            .with_message("unexpected end of file")
            .with_labels(vec![Label::primary((), location..location)])
            .with_notes(vec![format!("expected: {}", one_of(&expected))]),
        ParserError::UnrecognizedToken { token: (start, _, end), expected } => Diagnostic::error()
            .with_message(format!("unexpected {:?}", &source[start..end]))
            .with_labels(vec![Label::primary((), start..end)])
            .with_notes(vec![format!("expected: {}", one_of(&expected))]),
        ParserError::User { error } => Diagnostic::error()
            .with_message(error.message.unwrap_or_else(|| {
                format!("unexpected {:?}", &source[error.span.start..error.span.end])
            }))
            .with_labels(vec![Label::primary((), error.span)]),
    };

    let mut buffer = termcolor::Buffer::ansi();
    let config = term::Config::default();
    term::emit(&mut buffer, &config, &file, &diagnostic).unwrap();

    writer.write_all(&buffer.into_inner()).unwrap();
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
    output.pop();
    output.pop();
    output.push_str(" or ");
    output.push_str(token_last);
    output
}
