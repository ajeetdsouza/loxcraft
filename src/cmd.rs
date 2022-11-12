use std::fs;
use std::io::{self, Write};

use anyhow::{bail, Context, Result};
use clap::Parser;
use lox_common::error::ErrorS;
use lox_vm::VM;

#[derive(Debug, Parser)]
#[command(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    Lsp,
    Playground {
        #[arg(long, default_value = "3000")]
        port: u16,
    },
    Repl,
    Run {
        path: String,
    },
}

impl Cmd {
    pub fn run(&self) -> Result<()> {
        #[allow(unused_variables)]
        match self {
            #[cfg(feature = "lsp")]
            Cmd::Lsp => lox_lsp::serve(),
            #[cfg(not(feature = "lsp"))]
            Cmd::Lsp => bail!("'lsp' feature is not enabled"),

            #[cfg(feature = "playground")]
            Cmd::Playground { port } => lox_playground::serve(*port),
            #[cfg(not(feature = "playground"))]
            Cmd::Playground { .. } => bail!("'playground' feature is not enabled"),

            #[cfg(feature = "repl")]
            Cmd::Repl => lox_repl::run(),
            #[cfg(not(feature = "repl"))]
            Cmd::Repl => bail!("'repl' feature is not enabled"),

            Cmd::Run { path } => {
                let source = fs::read_to_string(path)
                    .with_context(|| format!("could not read file: {}", path))?;
                let mut vm = VM::default();
                let stdout = &mut io::stdout().lock();
                if let Err(e) = vm.run(&source, stdout) {
                    report_err(&source, e);
                    bail!("program exited with errors");
                }
                Ok(())
            }
        }
    }
}

fn report_err(source: &str, errors: Vec<ErrorS>) {
    let mut buffer = termcolor::Buffer::ansi();
    for err in errors {
        lox_common::error::report_error(&mut buffer, source, &err);
    }
    io::stderr().write_all(buffer.as_slice()).expect("failed to write to stderr");
}
