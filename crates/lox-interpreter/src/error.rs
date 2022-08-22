use crate::object::Object;

use codespan_reporting::diagnostic::Diagnostic;
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use termcolor::WriteColor;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
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

#[derive(Debug, Error)]
pub enum AttributeError {
    #[error(r#""{type_}" object has no attribute "{name}""#)]
    NoSuchAttribute { type_: String, name: String },
}

#[derive(Debug, Error)]
pub enum IoError {
    #[error("unable to write to file: {:?}", file)]
    WriteError { file: String },
}

#[derive(Debug, Error)]
pub enum NameError {
    #[error("name {:?} is already defined", name)]
    AlreadyDefined { name: String },
    #[error("name {:?} is not defined", name)]
    NotDefined { name: String },
}

#[derive(Debug, Error)]
pub enum SyntaxError {
    #[error(r#""return" outside function"#)]
    ReturnOutsideFunction { object: Option<Object> },
}

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("{name}() takes {exp_args} arguments but {got_args} were given")]
    ArityMismatch { name: String, exp_args: usize, got_args: usize },
    #[error(r#"init() should use an empty return, not "{type_}""#)]
    InitInvalidReturnType { type_: String },
    #[error("{:?} object is not callable", type_)]
    NotCallable { type_: String },
    #[error(r#"superclass should be of type "class", not "{type_}""#)]
    SuperclassInvalidType { type_: String },
    #[error("unsupported operand type(s) for {}: {:?}", op, rt_type)]
    UnsupportedOperandPrefix { op: String, rt_type: String },
    #[error("unsupported operand type(s) for {}: {:?} and {:?}", op, lt_type, rt_type)]
    UnsupportedOperandInfix { op: String, lt_type: String, rt_type: String },
}

pub fn report_err(writer: &mut dyn WriteColor, source: &str, mut errors: Vec<Diagnostic<()>>) {
    errors.sort_unstable_by_key(|e| {
        e.labels.first().map(|label| (label.range.start, label.range.end))
    });

    let file = SimpleFile::new("<script>", source);
    let config = term::Config::default();

    for err in errors {
        term::emit(writer, &config, &file, &err).unwrap();
    }
}
