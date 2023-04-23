use std::io::Write;
use std::{fs, str};

use loxcraft::vm::VM;
use pretty_assertions::assert_eq;
use test_generator::test_resources;

#[test_resources("res/examples/**/*.lox")]
fn lox(path: &str) {
    // Miri is too slow to run these tests.
    const MIRI_SKIP_PATHS: &[&str] =
        &["res/examples/limit/loop_too_large.lox", "res/examples/limit/stack_overflow.lox"];
    if cfg!(miri) && MIRI_SKIP_PATHS.contains(&path) {
        return;
    }

    let source = fs::read_to_string(path).expect("unable to read test file");
    let mut exp_output = String::new();
    for line in source.lines() {
        const OUT_COMMENT: &str = "// out: ";
        if let Some(idx) = line.find(OUT_COMMENT) {
            exp_output += &line[idx + OUT_COMMENT.len()..];
            exp_output += "\n";
        }
    }

    let mut got_output = Vec::new();
    if let Err(e) = VM::default().run(&source, &mut got_output) {
        let (e, _) = e.first().expect("received empty error");
        writeln!(&mut got_output, "{e}").expect("could not write to output");
    }
    let got_output = str::from_utf8(&got_output).expect("invalid UTF-8 in output");
    assert_eq!(exp_output, got_output);
}
