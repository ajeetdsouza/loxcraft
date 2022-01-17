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
            let mut readline = Editor::<()>::new();
            loop {
                let result = readline.readline(">>> ");
                match result {
                    Ok(line) => {
                        use crate::syntax::grammar::StmtParser;
                        use crate::vm::{Chunk, VM};

                        readline.add_history_entry(line.as_str());

                        let tokens = Lexer::new(&line);
                        let stmt = StmtParser::new().parse(tokens.into_iter()).unwrap();

                        let mut chunk = Chunk::new();
                        chunk.compile(&stmt);

                        let mut vm = VM::new(&chunk);
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
            Ok(())
        }
    }
}
