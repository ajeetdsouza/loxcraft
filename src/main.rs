#![allow(dead_code)]
mod app;
mod syntax;
mod vm;

use crate::app::App;
use crate::syntax::lexer::Lexer;

use clap::Parser as _;
use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::io;

fn main() -> io::Result<()> {
    let app = App::parse();
    match app {
        App::Repl => {
            use crate::vm::{Chunk, VM};

            let mut readline = Editor::<()>::new();

            let chunk = Chunk::new();
            let mut vm = VM::new(chunk);

            loop {
                let result = readline.readline(">>> ");
                match result {
                    Ok(line) => {
                        use crate::syntax::grammar::StmtParser;

                        readline.add_history_entry(line.as_str());

                        let tokens = Lexer::new(&line);
                        let stmt = StmtParser::new().parse(tokens.into_iter()).unwrap();

                        vm.chunk.compile(&stmt);
                        vm.run().unwrap();
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
        App::Run { .. } => todo!(),
    }

    Ok(())
}
