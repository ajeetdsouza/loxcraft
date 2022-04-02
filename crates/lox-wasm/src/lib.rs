use std::io::Write;

use lox_vm::vm::VM;
use wasm_bindgen::prelude::*;
use web_sys::Element;

struct Output(Element);

impl Output {
    const ELEMENT_ID: &'static str = "output";

    fn new() -> Self {
        let output = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(Self::ELEMENT_ID)
            .unwrap();
        Self(output)
    }

    fn clear(&self) {
        self.0.set_text_content(Some(""));
    }
}

impl Write for &Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let buf = std::str::from_utf8(buf).unwrap();
        self.0.append_with_str_1(buf).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen]
pub fn lox_run(source: &str) {
    let output = Output::new();
    output.clear();

    let program = lox_syntax::parse(source).unwrap();
    let compiler = lox_vm::compiler::Compiler::new();
    let function = compiler.compile(&program).unwrap();

    VM::new(&output, &output, false).run(function);
}
