use crate::repl::{self, Prompt};

use anyhow::{bail, Context, Result};
use clap::Parser as Clap;
use lox_common::error::ErrorS;
use lox_vm::VM;
use reedline::Signal;

use std::fs;
use std::io::{self, Write};

#[derive(Clap, Debug)]
#[clap(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    Lsp,
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
            Cmd::Lsp => lox_lsp::serve(),
            #[cfg(feature = "playground")]
            Cmd::Playground { port } => lox_playground::serve(*port),
            Cmd::Repl => repl(),
            Cmd::Run { path } => run(path),
        }
    }
}

pub fn repl() -> Result<()> {
    let mut vm = VM::new();
    let mut editor = repl::editor().context("could not start REPL")?;

    loop {
        match editor.read_line(&Prompt) {
            Ok(Signal::Success(line)) => {
                if let Err(e) = vm.run(&line) {
                    report_err(&line, e);
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
    let mut vm = VM::new();
    if let Err(e) = vm.run(&source) {
        report_err(&source, e);
        bail!("program exited with errors")
    }
    Ok(())
}

fn report_err(source: &str, errors: Vec<ErrorS>) {
    let mut buffer = termcolor::Buffer::ansi();
    for err in errors {
        lox_common::error::report_err(&mut buffer, source, &err);
    }
    io::stderr().write_all(buffer.as_slice()).expect("failed to write to stderr");
}
