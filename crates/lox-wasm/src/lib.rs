#![allow(clippy::unused_unit)]

use lox_syntax::parser::ParserError;
use lox_vm::vm::VM;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

use std::io::{self, Write};
use std::panic;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Message {
    Output { text: String },
    CompileSuccess,
    CompileFailure,
    ExitSuccess,
    ExitFailure,
}

#[wasm_bindgen]
extern "C" {
    fn postMessage(s: &str);
}

struct Output;

impl Output {
    fn new() -> Self {
        Self
    }
}

impl Write for &Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = ansi_to_html::convert_escaped(&String::from_utf8_lossy(buf)).unwrap();
        let message = serde_json::to_string(&Message::Output { text }).unwrap();
        postMessage(&message);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen]
pub fn lox_setup_panic() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub fn lox_run(source: &str) {
    console_error_panic_hook::set_once();

    let output = Output::new();
    let program = match lox_syntax::parse(source) {
        Ok(val) => val,
        Err(err) => {
            report_err(source, err, &output);
            return;
        }
    };

    let compiler = lox_vm::compiler::Compiler::new();
    let result = compiler.compile(&program);

    let message = if result.is_ok() { Message::CompileSuccess } else { Message::CompileFailure };
    postMessage(&serde_json::to_string(&message).unwrap());

    let function = match result {
        Ok(function) => function,
        Err(err) => {
            write!(&output, "{:?}", err).unwrap();
            return;
        }
    };
    VM::new(&output, &output, false).run(function);

    let message = Message::ExitSuccess; // TODO: VM::run() should return a Result.
    postMessage(&serde_json::to_string(&message).unwrap());
}

fn report_err(source: &str, err: ParserError, mut output: &Output) {
    use codespan_reporting::diagnostic::{Diagnostic, Label};
    use codespan_reporting::files::SimpleFile;
    use codespan_reporting::term;
    use codespan_reporting::term::termcolor;

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

    output.write_all(&buffer.into_inner()).unwrap();
}
