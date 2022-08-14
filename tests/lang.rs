use pretty_assertions::assert_eq;
use test_generator::test_resources;

use std::fmt::Write;
use std::fs;
use std::str;

#[test_resources("tests/lang/**/*.lox")]
fn lox(path: &str) {
    let source = fs::read_to_string(path).unwrap();

    let mut exp_out = String::new();
    let mut exp_err = String::new();

    for line in source.lines() {
        if let Some((_, out)) = line.split_once("// out: ") {
            writeln!(&mut exp_out, "{out}").unwrap();
        } else if let Some((_, out)) = line.split_once("// err: ") {
            writeln!(&mut exp_err, "{out}").unwrap();
        }
    }

    todo!()
    // let mut errors = Vec::new();
    // let function = Compiler::new().compile(&source, &mut errors);
    // assert_eq!(errors, Vec::new());

    // let mut got_out = Vec::new();
    // let mut got_err = Vec::new();
    // VM::new(&mut got_out, &mut got_err, false).run(function);
    // let got_out = str::from_utf8(&got_out).unwrap();
    // let got_err = str::from_utf8(&got_err).unwrap();

    // assert_eq!(exp_out, got_out);
    // assert_eq!(exp_err, got_err);
}
