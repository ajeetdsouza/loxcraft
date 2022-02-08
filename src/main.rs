use lox::app::App;
use lox::syntax::grammar::ProgramParser;
use lox::syntax::lexer::Lexer;
use lox::vm::compiler::Compiler;
use lox::vm::VM;

use clap::Parser as _;
use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::fs;
use std::io;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> io::Result<()> {
    let app = App::parse();
    match app {
        App::Repl => {
            let mut readline = Editor::<()>::new();

            let stdout = io::stdout();
            let stdout = stdout.lock();
            let mut vm = VM::new(stdout);

            loop {
                let result = readline.readline(">>> ");
                match result {
                    Ok(line) => {
                        readline.add_history_entry(line.as_str());

                        let tokens = Lexer::new(&line);
                        let program = ProgramParser::new().parse(tokens.into_iter()).unwrap();
                        let compiler = Compiler::new_script();
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
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
        }
        App::Run { path } => {
            let source = fs::read_to_string(path).unwrap();
            let tokens = Lexer::new(&source);
            let program = ProgramParser::new().parse(tokens.into_iter()).unwrap();
            let compiler = Compiler::new_script();
            let function = compiler.compile(&program).unwrap();

            let stdout = io::stdout();
            let stdout = stdout.lock();
            let mut vm = VM::new(stdout);
            vm.run(function);
        }
    }

    Ok(())
}
