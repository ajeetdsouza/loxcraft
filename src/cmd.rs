use crate::repl::{Highlighter, Prompt, Validator};
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
    let highlighter = Box::new(Highlighter::new());
    let validator = Box::new(Validator);
    let mut editor = Reedline::create()
        .expect("failed to create prompt")
        .with_highlighter(highlighter)
        .with_validator(validator);

    let stdout = io::stdout();
    let stdout = stdout.lock();
    let stderr = io::stdout();
    let stderr = stderr.lock();
    let mut vm = VM::new(stdout, stderr, debug, profile);

    loop {
        match editor.read_line(&Prompt) {
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
                eprintln!("CTRL-C");
            }
            Ok(Signal::CtrlD) => {
                eprintln!("CTRL-D");
                break;
            }
            Ok(Signal::CtrlL) => {
                if let Err(e) = editor.clear_screen() {
                    eprintln!("error: unable to clear screen: {:?}", e)
                };
            }
            Err(e) => {
                eprintln!("error: {:?}", e);
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
