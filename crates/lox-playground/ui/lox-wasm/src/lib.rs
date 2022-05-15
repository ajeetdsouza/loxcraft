use serde::Serialize;
use std::io::{self, Write};

use lox_vm::vm::VM;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn loxRun(source: &str) {
    console_error_panic_hook::set_once();

    let output = Output::new();
    let compiler = lox_vm::compiler::Compiler::new();
    let mut errors = Vec::new();
    let function = compiler.compile(source, &mut errors);
    if !errors.is_empty() {
        lox_vm::report_err(source, errors, &output);
        postMessage(&serde_json::to_string(&Message::CompileFailure).unwrap());
        return;
    };

    VM::new(&output, &output, false).run(function);
    let message = Message::ExitSuccess; // TODO: VM::run() should return a Result.
    postMessage(&serde_json::to_string(&message).unwrap());
}

#[allow(dead_code)]
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
    #[wasm_bindgen(js_namespace = self)]
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
