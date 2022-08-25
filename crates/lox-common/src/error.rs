use crate::types::Span;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use termcolor::WriteColor;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("AttributeError: {0}")]
    AttributeError(AttributeError),
    #[error("IOError: {0}")]
    IoError(IoError),
    #[error("NameError: {0}")]
    NameError(NameError),
    #[error("SyntaxError: {0}")]
    SyntaxError(SyntaxError),
    #[error("TypeError: {0}")]
    TypeError(TypeError),
}

impl AsDiagnostic for Error {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        match self {
            Error::AttributeError(e) => e.as_diagnostic(),
            Error::IoError(e) => e.as_diagnostic(),
            Error::NameError(e) => e.as_diagnostic(),
            Error::SyntaxError(e) => e.as_diagnostic(),
            Error::TypeError(e) => e.as_diagnostic(),
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AttributeError {
    #[error("{type_:?} object has no attribute {name:?}")]
    NoSuchAttribute { type_: String, name: String, span: Span },
}

impl AsDiagnostic for AttributeError {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        match self {
            AttributeError::NoSuchAttribute { span, .. } => Diagnostic::error()
                .with_code("AttributeError")
                .with_message(self.to_string())
                .with_labels(vec![Label::primary((), span.clone())]),
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum IoError {
    #[error("unable to write to file: {file:?}")]
    WriteError { file: String, span: Span },
}

impl AsDiagnostic for IoError {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        match self {
            IoError::WriteError { span, .. } => Diagnostic::error()
                .with_code("IOError")
                .with_message(self.to_string())
                .with_labels(vec![Label::primary((), span.clone())]),
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum NameError {
    #[error("name {name:?} is already defined")]
    AlreadyDefined { name: String, span: Span },
    #[error("name {name:?} is not defined")]
    NotDefined { name: String, span: Span },
}

impl AsDiagnostic for NameError {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        match self {
            NameError::AlreadyDefined { span, .. } | NameError::NotDefined { span, .. } => {
                Diagnostic::error()
                    .with_code("NameError")
                    .with_message(self.to_string())
                    .with_labels(vec![Label::primary((), span.clone())])
            }
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum SyntaxError {
    #[error("extraneous input: {token:?}")]
    ExtraToken { token: String, span: Span },
    #[error("invalid input")]
    InvalidToken { span: Span },
    #[error("unexpected input")]
    UnexpectedInput { token: String, span: Span },
    #[error("unexpected end of file")]
    UnrecognizedEOF { expected: Vec<String>, span: Span },
    #[error("unexpected {token:?}")]
    UnrecognizedToken { token: String, expected: Vec<String>, span: Span },
    #[error("unterminated string")]
    UnterminatedString { span: Span },
    #[error(r#""return" outside function"#)]
    ReturnOutsideFunction { span: Span },
}

impl AsDiagnostic for SyntaxError {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        match self {
            SyntaxError::ExtraToken { span, .. }
            | SyntaxError::InvalidToken { span }
            | SyntaxError::ReturnOutsideFunction { span }
            | SyntaxError::UnexpectedInput { span, .. }
            | SyntaxError::UnterminatedString { span } => Diagnostic::error()
                .with_code("SyntaxError")
                .with_message(self.to_string())
                .with_labels(vec![Label::primary((), span.clone())]),
            SyntaxError::UnrecognizedEOF { expected, span }
            | SyntaxError::UnrecognizedToken { expected, span, .. } => Diagnostic::error()
                .with_code("SyntaxError")
                .with_message(self.to_string())
                .with_labels(vec![Label::primary((), span.clone())])
                .with_notes(vec![format!("expected: {}", one_of(expected))]),
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum TypeError {
    #[error("{name}() takes {exp_args} arguments but {got_args} were given")]
    ArityMismatch { name: String, exp_args: usize, got_args: usize, span: Span },
    #[error("init() should use an empty return, not {type_:?}")]
    InitInvalidReturnType { type_: String, span: Span },
    #[error("{type_:?} object is not callable")]
    NotCallable { type_: String, span: Span },
    #[error(r#"superclass should be of type "class", not {type_:?}"#)]
    SuperclassInvalidType { type_: String, span: Span },
    #[error("unsupported operand type for {op}: {rt_type:?}")]
    UnsupportedOperandPrefix { op: String, rt_type: String, span: Span },
    #[error("unsupported operand type(s) for {op}: {lt_type:?} and {rt_type:?}")]
    UnsupportedOperandInfix { op: String, lt_type: String, rt_type: String, span: Span },
}

impl AsDiagnostic for TypeError {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        match self {
            TypeError::ArityMismatch { span, .. }
            | TypeError::InitInvalidReturnType { span, .. }
            | TypeError::NotCallable { span, .. }
            | TypeError::SuperclassInvalidType { span, .. }
            | TypeError::UnsupportedOperandPrefix { span, .. }
            | TypeError::UnsupportedOperandInfix { span, .. } => Diagnostic::error()
                .with_code("TypeError")
                .with_message(self.to_string())
                .with_labels(vec![Label::primary((), span.clone())]),
        }
    }
}

trait AsDiagnostic {
    fn as_diagnostic(&self) -> Diagnostic<()>;
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

pub fn report_err(writer: &mut dyn WriteColor, source: &str, e: Error) {
    let file = SimpleFile::new("<script>", source);
    let config = term::Config::default();
    let diagnostic = e.as_diagnostic();
    term::emit(writer, &config, &file, &diagnostic).unwrap();
}
