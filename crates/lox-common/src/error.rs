use std::io;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use termcolor::WriteColor;
use thiserror::Error;

use crate::types::{Span, Spanned};

pub type Result<T, E = ErrorS> = std::result::Result<T, E>;
pub type ErrorS = Spanned<Error>;

#[derive(Debug, Error, Eq, PartialEq)]
#[remain::sorted]
pub enum Error {
    #[error("AttributeError: {0}")]
    AttributeError(AttributeError),
    #[error("IOError: {0}")]
    IoError(IoError),
    #[error("NameError: {0}")]
    NameError(NameError),
    #[error("OverflowError: {0}")]
    OverflowError(OverflowError),
    #[error("SyntaxError: {0}")]
    SyntaxError(SyntaxError),
    #[error("TypeError: {0}")]
    TypeError(TypeError),
}

impl AsDiagnostic for Error {
    #[remain::check]
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        #[remain::sorted]
        match self {
            Error::AttributeError(e) => e.as_diagnostic(span),
            Error::IoError(e) => e.as_diagnostic(span),
            Error::NameError(e) => e.as_diagnostic(span),
            Error::OverflowError(e) => e.as_diagnostic(span),
            Error::SyntaxError(e) => e.as_diagnostic(span),
            Error::TypeError(e) => e.as_diagnostic(span),
        }
    }
}

macro_rules! impl_from_error {
    ($($error:tt),+) => {$(
        impl From<$error> for Error {
            fn from(e: $error) -> Self {
                Error::$error(e)
            }
        }
    )+};
}

impl_from_error!(AttributeError, IoError, NameError, OverflowError, SyntaxError, TypeError);

#[derive(Debug, Error, Eq, PartialEq)]
#[remain::sorted]
pub enum AttributeError {
    #[error("{type_:?} object has no attribute {name:?}")]
    NoSuchAttribute { type_: String, name: String },
}

impl AsDiagnostic for AttributeError {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        Diagnostic::error()
            .with_code("AttributeError")
            .with_message(self.to_string())
            .with_labels(vec![Label::primary((), span.clone())])
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
#[remain::sorted]
pub enum IoError {
    #[error("unable to write to file: {file:?}")]
    WriteError { file: String },
}

impl AsDiagnostic for IoError {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        Diagnostic::error()
            .with_code("IOError")
            .with_message(self.to_string())
            .with_labels(vec![Label::primary((), span.clone())])
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
#[remain::sorted]
pub enum NameError {
    #[error("cannot access variable {name:?} in its own initializer")]
    AccessInsideInitializer { name: String },
    #[error("name {name:?} is already defined")]
    AlreadyDefined { name: String },
    #[error("class {name:?} inherits from itself")]
    ClassInheritFromSelf { name: String },
    #[error("name {name:?} is not defined")]
    NotDefined { name: String },
}

impl AsDiagnostic for NameError {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        Diagnostic::error()
            .with_code("NameError")
            .with_message(self.to_string())
            .with_labels(vec![Label::primary((), span.clone())])
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
#[remain::sorted]
pub enum OverflowError {
    #[error("jump body is too large")]
    JumpTooLarge,
    #[error("stack overflow")]
    StackOverflow,
    #[error("cannot use more than 256 arguments in a function")]
    TooManyArgs,
    #[error("cannot define more than 256 constants in a function")]
    TooManyConstants,
    #[error("cannot define more than 256 local variables in a function")]
    TooManyLocals,
    #[error("cannot define more than 256 parameters in a function")]
    TooManyParams,
    #[error("cannot use more than 256 closure variables in a function")]
    TooManyUpvalues,
}

impl AsDiagnostic for OverflowError {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        Diagnostic::error()
            .with_code("OverflowError")
            .with_message(self.to_string())
            .with_labels(vec![Label::primary((), span.clone())])
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
#[remain::sorted]
pub enum SyntaxError {
    #[error("extraneous input: {token:?}")]
    ExtraToken { token: String },
    #[error("invalid input")]
    InvalidToken,
    #[error(r#"init() should not return a value"#)]
    ReturnInInitializer,
    #[error(r#""return" used outside function"#)]
    ReturnOutsideFunction,
    #[error(r#""super" used outside class"#)]
    SuperOutsideClass,
    #[error(r#""super" used in class without a superclass"#)]
    SuperWithoutSuperclass,
    #[error(r#""this" used outside class"#)]
    ThisOutsideClass,
    #[error("unexpected input")]
    UnexpectedInput { token: String },
    #[error("unexpected end of file")]
    UnrecognizedEOF { expected: Vec<String> },
    #[error("unexpected {token:?}")]
    UnrecognizedToken { token: String, expected: Vec<String> },
    #[error("unterminated string")]
    UnterminatedString,
}

impl AsDiagnostic for SyntaxError {
    #[remain::check]
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        let mut diagnostic = Diagnostic::error()
            .with_code("SyntaxError")
            .with_message(self.to_string())
            .with_labels(vec![Label::primary((), span.clone())]);
        #[remain::sorted]
        match self {
            SyntaxError::UnrecognizedEOF { expected, .. }
            | SyntaxError::UnrecognizedToken { expected, .. } => {
                diagnostic = diagnostic.with_notes(vec![format!("expected: {}", one_of(expected))]);
            }
            _ => {}
        };
        diagnostic
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
#[remain::sorted]
pub enum TypeError {
    #[error("{name}() takes {exp_args} arguments but {got_args} were given")]
    ArityMismatch { name: String, exp_args: u8, got_args: u8 },
    #[error("init() should use an empty return, not {type_:?}")]
    InitInvalidReturnType { type_: String },
    #[error("{type_:?} object is not callable")]
    NotCallable { type_: String },
    #[error(r#"superclass should be of type "class", not {type_:?}"#)]
    SuperclassInvalidType { type_: String },
    #[error("unsupported operand type(s) for {op}: {lt_type:?} and {rt_type:?}")]
    UnsupportedOperandInfix { op: String, lt_type: String, rt_type: String },
    #[error("unsupported operand type for {op}: {rt_type:?}")]
    UnsupportedOperandPrefix { op: String, rt_type: String },
}

impl AsDiagnostic for TypeError {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        Diagnostic::error()
            .with_code("TypeError")
            .with_message(self.to_string())
            .with_labels(vec![Label::primary((), span.clone())])
    }
}

trait AsDiagnostic {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()>;
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

pub fn report_errors(writer: &mut impl io::Write, source: &str, errors: &[ErrorS]) {
    let mut buffer = termcolor::Buffer::ansi();
    for err in errors {
        report_error(&mut buffer, source, err);
    }
    writer.write_all(buffer.as_slice()).expect("failed to write to output");
}

pub fn report_error(writer: &mut impl WriteColor, source: &str, (error, span): &ErrorS) {
    let file = SimpleFile::new("<script>", source);
    let config = term::Config::default();
    let diagnostic = error.as_diagnostic(span);
    term::emit(writer, &config, &file, &diagnostic).expect("failed to write to output");
}
