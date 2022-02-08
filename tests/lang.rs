use lox::syntax;
use lox::vm::compiler::Compiler;
use lox::vm::VM;

use regex::Regex;
use test_generator::test_resources;

use std::fs;
use std::io::Write;

thread_local! {
    static RE_EXPECT: Regex = Regex::new(r" // expect: (.*)").unwrap();
}

#[test_resources("tests/lang/**/*.lox")]
fn run_file(path: &str) {
    let source = fs::read_to_string(path).unwrap();
    let program = syntax::parse(&source).unwrap();
    let compiler = Compiler::new_script();
    let function = compiler.compile(&program).unwrap();

    let mut got = Vec::new();
    let mut vm = VM::new(&mut got, false);
    vm.run(function);

    let mut expected = Vec::new();
    RE_EXPECT.with(|re| {
        for captures in re.captures_iter(&source) {
            writeln!(expected, "{}", &captures[1]).unwrap();
        }
    });

    assert_eq!(got, expected);
}
