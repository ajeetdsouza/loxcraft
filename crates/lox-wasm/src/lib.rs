#![allow(clippy::unused_unit)]

use lox_vm::vm::VM;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

use std::io::{self, Write};

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

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn loxRun(source: &str) {
    console_error_panic_hook::set_once();

    let output = Output::new();
    let compiler = lox_vm::compiler::Compiler::new();
    let function = match compiler.compile(source) {
        Ok(val) => val,
        Err(err) => {
            lox_vm::report_err(source, err, &output);
            postMessage(&serde_json::to_string(&Message::CompileFailure).unwrap());
            return;
        }
    };
    VM::new(&output, &output, false).run(function);

    let message = Message::ExitSuccess; // TODO: VM::run() should return a Result.
    postMessage(&serde_json::to_string(&message).unwrap());
}
