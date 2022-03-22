use crate::repl::{LoxPrompt, LoxValidator};
use crate::syntax;
use crate::vm::compiler::Compiler;
use crate::vm::vm::VM;

use clap::Parser;
use reedline::{Reedline, Signal};

use std::fs;
use std::io;

#[derive(Debug, Parser)]
#[clap(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    REPL {
        #[clap(long)]
        debug: bool,
    },
    Run {
        path: String,
        #[clap(long)]
        debug: bool,
        #[clap(long)]
        profile: bool,
    },
}

impl Cmd {
    pub fn run(&self) {
        match self {
            Cmd::REPL { debug } => repl(*debug, false),
            Cmd::Run { path, debug, profile } => run(path, *debug, *profile),
        }
    }
}

pub fn repl(debug: bool, profile: bool) {
    let validator = Box::new(LoxValidator);
    let mut editor = Reedline::create().unwrap().with_validator(validator);

    let stdout = io::stdout();
    let stdout = stdout.lock();

    let stderr = io::stdout();
    let stderr = stderr.lock();

    let mut vm = VM::new(stdout, stderr, debug, profile);
    loop {
        match editor.read_line(&LoxPrompt) {
            Ok(Signal::Success(line)) => {
                let program = match syntax::parse(&line) {
                    Ok(program) => program,
                    Err(err) => {
                        syntax::report_err("<stdin>", &line, err).unwrap();
                        continue;
                    }
                };
                let compiler = Compiler::new();
                let function = compiler.compile(&program).unwrap();
                vm.run(function);
            }
            Ok(Signal::CtrlC) => {
                println!("CTRL-C");
                break;
            }
            Ok(Signal::CtrlD) => {
                println!("CTRL-D");
                break;
            }
            Ok(Signal::CtrlL) => {
                editor.clear_screen().unwrap();
            }
            Err(err) => {
                println!("error: {:?}", err);
                break;
            }
        }
    }
}

fn run(path: &str, debug: bool, profile: bool) {
    let source = fs::read_to_string(&path).unwrap();
    let program = match syntax::parse(&source) {
        Ok(program) => program,
        Err(err) => {
            syntax::report_err(path, &source, err).unwrap();
            return;
        }
    };
    let compiler = Compiler::new();
    let function = compiler.compile(&program).unwrap();

    let stdout = io::stdout();
    let stdout = stdout.lock();

    let stderr = io::stderr();
    let stderr = stderr.lock();

    let mut vm = VM::new(stdout, stderr, debug, profile);
    vm.run(function);
}
