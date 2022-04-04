#![allow(clippy::unused_unit)]

use lox_syntax::parser::ParserError;
use lox_vm::vm::VM;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::MessagePort;

use std::io::{self, Write};

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Message {
    Output { text: String },
}

struct Output(MessagePort);

impl Output {
    fn new(port: MessagePort) -> Self {
        Self(port)
    }
}

impl Write for &Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let message = Message::Output { text: String::from_utf8_lossy(buf).to_string() };
        let message = serde_json::to_string(&message).unwrap();
        self.0.post_message(&JsValue::from_str(&message)).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen]
pub fn lox_run(source: &str, ws_stdout: MessagePort) {
    let output = Output::new(ws_stdout);
    let program = match lox_syntax::parse(source) {
        Ok(val) => val,
        Err(err) => {
            report_err(source, err, &output);
            return;
        }
    };
    let compiler = lox_vm::compiler::Compiler::new();
    let function = match compiler.compile(&program) {
        Ok(val) => val,
        Err(err) => {
            write!(&output, "{:?}", err).unwrap();
            return;
        }
    };
    VM::new(&output, &output, false).run(function);
}

fn report_err(source: &str, err: ParserError, stderr: &Output) {
    use codespan_reporting::diagnostic::{Diagnostic, Label};
    use codespan_reporting::files::SimpleFile;
    use codespan_reporting::term::termcolor;
    use codespan_reporting::term::{self, Config};

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
    let mut writer = termcolor::NoColor::new(stderr);
    let config = Config::default();
    let file = SimpleFile::new("src_file", source);
    let diagnostic = Diagnostic::error()
        .with_message(label)
        .with_labels(vec![Label::primary((), range)])
        .with_notes(notes);
    term::emit(&mut writer, &config, &file, &diagnostic).unwrap();
}
