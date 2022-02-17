use crate::syntax;
use crate::vm::compiler::Compiler;
use crate::vm::vm::VM;

use clap::{AppSettings, Parser};

use std::fs;
use std::io;

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    version,
    global_setting(AppSettings::DisableHelpSubcommand),
    global_setting(AppSettings::PropagateVersion)
)]
pub enum App {
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

impl App {
    pub fn run(&self) {
        match self {
            App::REPL { debug } => repl(*debug),
            App::Run { path, debug } => run(path, *debug),
        }
    }
}

pub fn repl(debug: bool) {
    use rustyline::error::ReadlineError;
    use rustyline::Editor;

    let mut readline = Editor::<()>::new();

    let stdout = io::stdout();
    let stdout = stdout.lock();

    let mut vm = VM::new(stdout, debug);
    loop {
        let result = readline.readline(">>> ");
        match result {
            Ok(line) => {
                readline.add_history_entry(&line);
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
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("error: {:?}", err);
                break;
            }
        }
    }
}

fn run(path: &str, debug: bool) {
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

    let mut vm = VM::new(stdout, debug);
    vm.run(function);
}
