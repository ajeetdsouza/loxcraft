use lox_vm::compiler::Compiler;
use lox_vm::vm::VM;
use serde::Serialize;
use termcolor::{Color, WriteColor};
use wasm_bindgen::prelude::*;

use std::io::{self, Write};

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn loxRun(source: &str) {
    console_error_panic_hook::set_once();

    let compiler = Compiler::new();
    let mut errors = Vec::new();
    let function = compiler.compile(source, &mut errors);
    let output = Output::new();

    if !errors.is_empty() {
        let mut writer = HtmlWriter::new(&output);
        lox_vm::report_err(&mut writer, source, errors);
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
        let text = String::from_utf8_lossy(buf).to_string();
        let message = serde_json::to_string(&Message::Output { text }).unwrap();
        postMessage(&message);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Provides a [`WriteColor`] implementation for HTML, using Bootstrap 5.1
/// classes.
struct HtmlWriter<W> {
    writer: W,
    span_count: usize,
}

impl<W> HtmlWriter<W> {
    fn new(writer: W) -> Self {
        HtmlWriter { writer, span_count: 0 }
    }
}

impl<W: Write> Write for HtmlWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let escaped = String::from_utf8_lossy(buf);
        let escaped = askama_escape::escape(&escaped, askama_escape::Html).to_string();
        write!(self.writer, "{}", escaped)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write> WriteColor for HtmlWriter<W> {
    fn supports_color(&self) -> bool {
        true
    }

    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> std::io::Result<()> {
        if spec.reset() {
            self.reset()?;
        }

        let mut classes = Vec::new();
        if let Some(fg) = spec.fg() {
            match fg {
                Color::Black => classes.push("text-black"),
                Color::Blue => classes.push("text-primary"),
                Color::Green => classes.push("text-success"),
                Color::Red => classes.push("text-danger"),
                Color::White => classes.push("text-white"),
                Color::Yellow => classes.push("text-warning"),
                _ => (),
            };
        }
        if let Some(bg) = spec.bg() {
            match bg {
                Color::Blue => classes.push("bg-primary"),
                Color::Black => classes.push("bg-black"),
                Color::Green => classes.push("bg-success"),
                Color::Red => classes.push("bg-danger"),
                Color::White => classes.push("bg-white"),
                Color::Yellow => classes.push("bg-warning"),
                _ => (),
            };
        }
        if spec.bold() {
            classes.push("fw-bold");
        }
        if spec.dimmed() {
            classes.push("opacity-75");
        }
        if spec.italic() {
            classes.push("fst-italic");
        }
        if spec.underline() {
            classes.push("text-decoration-underline");
        }

        if !classes.is_empty() {
            write!(self.writer, r#"<span class="{}">"#, classes.join(" "))?;
            self.span_count += 1;
        }
        Ok(())
    }

    fn reset(&mut self) -> std::io::Result<()> {
        for _ in 0..self.span_count {
            write!(self.writer, "</span>")?;
        }
        self.span_count = 0;
        Ok(())
    }
}
