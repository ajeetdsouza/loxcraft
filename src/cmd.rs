use crate::repl;
use lox_vm::compiler::Compiler;
use lox_vm::vm::VM;

use clap::Parser as Clap;
use reedline::Signal;

use std::fs;
use std::io;

#[derive(Clap, Debug)]
#[clap(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    Playground {
        #[clap(long, default_value = "3000")]
        port: u16,
    },
    REPL {
        #[clap(long)]
        debug: bool,
    },
    Run {
        path: String,
        #[clap(long)]
        debug: bool,
    },
}

impl Cmd {
    pub fn run(&self) {
        match self {
            Cmd::Playground { port } => lox_playground::serve(*port),
            Cmd::REPL { debug } => repl(*debug),
            Cmd::Run { path, debug } => run(path, *debug),
        }
    }
}

pub fn playground() {
    // lox_playground::run();
    // lox_playground::;
}

pub fn repl(debug: bool) {
    let mut editor = repl::editor().unwrap();
    let stdout = io::stdout();
    let stdout = stdout.lock();
    let stderr = io::stderr();
    let stderr = stderr.lock();
    let mut vm = VM::new(stdout, stderr, debug);

    loop {
        match editor.read_line(&repl::Prompt) {
            Ok(Signal::Success(line)) => {
                let compiler = Compiler::new();
                let function = match compiler.compile(&line) {
                    Ok(function) => function,
                    Err(err) => {
                        lox_vm::report_err(&line, err, io::stderr());
                        continue;
                    }
                };
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

fn run(path: &str, debug: bool) {
    let source = fs::read_to_string(&path).unwrap();
    let compiler = Compiler::new();
    let function = match compiler.compile(&source) {
        Ok(program) => program,
        Err(err) => {
            lox_vm::report_err(&source, err, io::stderr());
            return;
        }
    };

    let stdout = io::stdout();
    let stdout = stdout.lock();

    let stderr = io::stderr();
    let stderr = stderr.lock();

    let mut vm = VM::new(stdout, stderr, debug);
    vm.run(function);
}
