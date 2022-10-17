use std::fs;
use std::io::{self, Write};

use anyhow::{bail, Context, Result};
use clap::Parser;
use lox_common::error::ErrorS;
use lox_vm::VM;

#[derive(Debug, Parser)]
#[command(
    about,
    author,
    disable_help_subcommand = true,
    propagate_version = true,
    version
)]
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
            Cmd::Lsp => {
                #[cfg(not(feature = "lsp"))]
                bail!("'lsp' feature is not enabled");

                #[cfg(feature = "lsp")]
                lox_lsp::serve()
            }
            Cmd::Playground { port } => {
                #[cfg(not(feature = "playground"))]
                bail!("'playground' feature is not enabled");

                #[cfg(feature = "playground")]
                lox_playground::serve(*port)
            }
            Cmd::Repl => {
                #[cfg(not(feature = "repl"))]
                bail!("'repl' feature is not enabled");

                #[cfg(feature = "repl")]
                lox_repl::run()
            }
            Cmd::Run { path } => {
                let source = fs::read_to_string(&path).with_context(|| {
                    format!("could not read file: {}", path)
                })?;
                let mut vm = VM::default();
                let stdout = &mut io::stdout().lock();
                if let Err(e) = vm.run(&source, stdout) {
                    report_err(&source, e);
                    bail!("program exited with errors")
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
    io::stderr()
        .write_all(buffer.as_slice())
        .expect("failed to write to stderr");
}
