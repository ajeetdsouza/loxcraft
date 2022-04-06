#![allow(clippy::new_without_default, clippy::module_inception)]

mod chunk;
pub mod compiler;
mod op;
mod value;
pub mod vm;

use codespan_reporting::diagnostic::Diagnostic;
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;

use std::io::Write;

pub fn report_err<W: Write>(source: &str, diagnostic: Diagnostic<()>, mut writer: W) {
    let file = SimpleFile::new("<script>", source);

    let mut buffer = termcolor::Buffer::ansi();
    let config = term::Config::default();
    term::emit(&mut buffer, &config, &file, &diagnostic).unwrap();

    writer.write_all(&buffer.into_inner()).unwrap();
}
