use std::fs;
use std::io::{self, Write};

use anyhow::{Context, Result, bail};
use clap::Parser;

use crate::error::ErrorS;
use crate::vm::VM;

#[derive(Debug, Parser)]
#[command(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    Lsp,
    Repl,
    Run { path: String },
}

impl Cmd {
    pub fn run(&self) -> Result<()> {
        #[allow(unused_variables)]
        match self {
            #[cfg(feature = "lsp")]
            Cmd::Lsp => crate::lsp::serve(),
            #[cfg(not(feature = "lsp"))]
            Cmd::Lsp => bail!("loxcraft was not compiled with the lsp feature"),

            #[cfg(feature = "repl")]
            Cmd::Repl => crate::repl::run(),
            #[cfg(not(feature = "repl"))]
            Cmd::Repl => bail!("loxcraft was not compiled with the repl feature"),

            Cmd::Run { path } => {
                let source = fs::read_to_string(path)
                    .with_context(|| format!("could not read file: {path}"))?;
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
        crate::error::report_error(&mut buffer, source, &err);
    }
    io::stderr().write_all(buffer.as_slice()).expect("failed to write to stderr");
}
