use crate::repl::{self, Prompt};

use anyhow::{bail, Context, Result};
use clap::Parser as Clap;
use lox_common::error::Error;
use lox_interpreter::Interpreter;
use reedline::Signal;

use std::fs;
use std::io::{self, Write};

#[derive(Clap, Debug)]
#[clap(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    #[cfg(feature = "playground")]
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
    pub fn run(&self) -> Result<()> {
        match self {
            #[cfg(feature = "playground")]
            Cmd::Playground { port } => lox_playground::serve(*port),
            Cmd::Repl => repl(),
            Cmd::Run { path } => run(path),
        }
    }
}

pub fn repl() -> Result<()> {
    let stdout = &mut io::stdout().lock();
    let mut interpreter = Interpreter::new(stdout);
    let mut editor = repl::editor().context("could not start REPL")?;

    loop {
        match editor.read_line(&Prompt) {
            Ok(Signal::Success(line)) => {
                let errors = interpreter.run(&line);
                if !errors.is_empty() {
                    report_err(&line, errors);
                }
            }
            Ok(Signal::CtrlC) => eprintln!("^C"),
            Ok(Signal::CtrlD) => break,
            Err(e) => {
                eprintln!("error: {e:?}");
                break;
            }
        }
    }

    Ok(())
}

fn run(path: &str) -> Result<()> {
    let source =
        fs::read_to_string(&path).with_context(|| format!("could not read file: {}", path))?;
    let stdout = &mut io::stdout().lock();
    let mut interpreter = Interpreter::new(stdout);
    let errors = interpreter.run(&source);
    if !errors.is_empty() {
        report_err(&source, errors);
        bail!("program exited with errors")
    }
    Ok(())
}

fn report_err(source: &str, errors: Vec<Error>) {
    let mut buffer = termcolor::Buffer::ansi();
    for e in errors {
        lox_common::error::report_err(&mut buffer, source, e);
    }
    io::stderr().write_all(buffer.as_slice()).expect("failed to write to stderr");
}
