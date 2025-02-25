use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use loxcraft::error::report_error;
use loxcraft::vm::VM;
use serde::Serialize;
use termcolor::{Color, WriteColor};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn loxRun(source: &str) {
    let writer = Output::new();
    let mut writer = HtmlWriter::new(writer);
    match VM::default().run(source, &mut writer) {
        Ok(()) => postMessage(&Message::ExitSuccess.to_string()),
        Err(errors) => {
            for e in errors.iter() {
                report_error(&mut writer, source, e);
            }
            postMessage(&Message::ExitFailure.to_string());
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Message {
    ExitFailure,
    ExitSuccess,
    Output { text: String },
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).expect("could not serialize message"))
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = self)]
    fn postMessage(s: &str);
}

#[derive(Debug)]
struct Output;

impl Output {
    fn new() -> Self {
        Self
    }
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf).to_string();
        postMessage(&Message::Output { text }.to_string());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Provides a [`WriteColor`] implementation for HTML, using Tailwind CSS classes.
#[derive(Debug)]
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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let escaped = String::from_utf8_lossy(buf);
        let escaped = askama_escape::escape(&escaped, askama_escape::Html).to_string();
        write!(self.writer, "{escaped}")?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write> WriteColor for HtmlWriter<W> {
    fn supports_color(&self) -> bool {
        true
    }

    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        if spec.reset() {
            self.reset()?;
        }

        let mut classes = Vec::new();
        if let Some(fg) = spec.fg() {
            match fg {
                Color::Black => classes.push("text-zinc-950"),
                Color::Blue => classes.push("text-blue-300"),
                Color::Green => classes.push("text-lime-300"),
                Color::Red => classes.push("text-red-500"),
                Color::Yellow => classes.push("text-amber-300"),
                _ => classes.push("text-zinc-50"),
            };
        }
        if let Some(bg) = spec.bg() {
            match bg {
                Color::Black => classes.push("bg-zinc-950"),
                Color::Blue => classes.push("bg-blue-300"),
                Color::Green => classes.push("bg-lime-300"),
                Color::Red => classes.push("bg-red-500"),
                Color::Yellow => classes.push("bg-amber-300"),
                _ => classes.push("bg-zinc-50"),
            };
        }
        if spec.bold() {
            classes.push("font-bold");
        }
        if spec.dimmed() {
            classes.push("text-opacity-75");
        }
        if spec.italic() {
            classes.push("italic");
        }
        if spec.underline() {
            classes.push("underline");
        }

        if !classes.is_empty() {
            write!(self.writer, r#"<span class="{}">"#, classes.join(" "))?;
            self.span_count += 1;
        }
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        for _ in 0..self.span_count {
            write!(self.writer, "</span>")?;
        }
        self.span_count = 0;
        Ok(())
    }
}
