use lox_interpreter::Interpreter;
use pretty_assertions::assert_eq;
use test_generator::test_resources;

use std::fs;
use std::io::Write;
use std::str;

#[test_resources("tests/lang/**/*.lox")]
fn lox(path: &str) {
    let source = fs::read_to_string(path).unwrap();

    let mut exp_output = String::new();
    for line in source.lines() {
        const OUT_COMMENT: &str = "// out: ";
        if let Some(idx) = line.rfind(OUT_COMMENT) {
            exp_output += &line[idx + OUT_COMMENT.len()..];
            exp_output += "\n";
        }
    }

    let mut got_output = Vec::new();
    let errors = Interpreter::new(&mut got_output).run(&source);
    if let Some(e) = errors.first() {
        writeln!(&mut got_output, "{e}").unwrap();
    }
    assert_eq!(exp_output, std::str::from_utf8(&got_output).unwrap());
}
