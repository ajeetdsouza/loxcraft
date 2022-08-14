use codespan_reporting::diagnostic::Diagnostic;
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use termcolor::WriteColor;

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
