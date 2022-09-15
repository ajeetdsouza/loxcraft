use std::io::Write;
use std::path::Path;
use std::{fs, str};

use lox_vm::VM;
use pretty_assertions::assert_eq;
use test_generator::test_resources;

#[test_resources("./res/examples/**/*.lox")]
fn lox(path: &str) {
    let path = Path::new("../..").join(path);
    let source = fs::read_to_string(path).unwrap();
    let mut exp_output = String::new();
    for line in source.lines() {
        const OUT_COMMENT: &str = "// out: ";
        if let Some(idx) = line.find(OUT_COMMENT) {
            exp_output += &line[idx + OUT_COMMENT.len()..];
            exp_output += "\n";
        }
    }

    let mut got_output = Vec::new();
    if let Err(e) = VM::new().run(&source, &mut got_output) {
        let (e, _) = e.first().expect("received empty error");
        writeln!(&mut got_output, "{}", e).expect("could not write to output");
    }
    let got_output = str::from_utf8(&got_output).expect("invalid UTF-8 in output");
    assert_eq!(exp_output, got_output);
}
