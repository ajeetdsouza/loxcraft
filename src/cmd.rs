use crate::repl::{self, Prompt};

use clap::Parser as Clap;
use lox_interpreter::Interpreter;
use reedline::Signal;

use std::fs;
use std::io::{self, Write};

#[derive(Clap, Debug)]
#[clap(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    Playground {
        #[clap(long, default_value = "3000")]
        port: u16,
    },
    Repl,
    Run {
        path: String,
    },
}

impl Cmd {
    pub fn run(&self) {
        match self {
            Cmd::Playground { port } => lox_playground::serve(*port),
            Cmd::Repl => repl(),
            Cmd::Run { path } => run(path),
        }
    }
}

pub fn repl() {
    let stdout = io::stdout();
    let mut interpreter = Interpreter::new(stdout);
    let mut editor = repl::editor().unwrap();

    loop {
        match editor.read_line(&Prompt) {
            Ok(Signal::Success(line)) => {
                let mut errors = Vec::new();
                let program = lox_syntax::parse(&line, &mut errors);
                if !errors.is_empty() {
                    let mut buffer = termcolor::Buffer::ansi();
                    lox_interpreter::report_err(&mut buffer, &line, errors);
                    io::stderr().write_all(buffer.as_slice()).unwrap();
                    continue;
                }
                if let Err(e) = interpreter.run(&program) {
                    println!("{e}")
                }
            }
            Ok(Signal::CtrlC) => eprintln!("^C"),
            Ok(Signal::CtrlD) => break,
            Err(e) => {
                eprintln!("error: {:?}", e);
                break;
            }
        }
    }
}

fn run(path: &str) {
    let source = fs::read_to_string(&path).unwrap();
    todo!()
    // let compiler = Compiler::new();
    // let mut errors = Vec::new();
    // let function = compiler.compile(&source, &mut errors);
    // if !errors.is_empty() {
    //     let mut buffer = termcolor::Buffer::ansi();
    //     lox_vm::report_err(&mut buffer, &source, errors);
    //     io::stderr().write_all(buffer.as_slice()).unwrap();
    //     return;
    // };

    // let stdout = io::stdout().lock();
    // let stderr = io::stderr().lock();
    // let mut vm = VM::new(stdout, stderr, debug);
    // vm.run(function);
}
