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

pub fn report_err<W: Write>(source: &str, mut errors: Vec<Diagnostic<()>>, mut writer: W) {
    errors.sort_unstable_by_key(|e| {
        e.labels.first().map(|label| (label.range.start, label.range.end))
    });

    let file = SimpleFile::new("<script>", source);
    let mut buffer = termcolor::Buffer::ansi();
    let config = term::Config::default();
    for err in errors {
        term::emit(&mut buffer, &config, &file, &err).unwrap();
    }

    writer.write_all(buffer.as_slice()).unwrap();
}
