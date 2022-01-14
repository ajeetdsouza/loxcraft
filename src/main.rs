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
                        readline.add_history_entry(line.as_str());
                        let tokens = Lexer::new(&line).collect::<Vec<_>>();
                        println!("{:?}", tokens);

                        use crate::syntax::grammar::ExprParser;
                        let result = ExprParser::new().parse(tokens.into_iter());
                        println!("{:?}", result);
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
