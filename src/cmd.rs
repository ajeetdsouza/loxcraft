use crate::repl;
use lox_syntax::parser::ParserError;
use lox_vm::compiler::Compiler;
use lox_vm::vm::VM;

use clap::Parser as Clap;
use reedline::Signal;

use std::fs;
use std::io;

#[derive(Clap, Debug)]
#[clap(about, author, disable_help_subcommand = true, propagate_version = true, version)]
pub enum Cmd {
    Playground {
        #[clap(long, default_value = "3000")]
        port: u16,
    },
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

impl Cmd {
    pub fn run(&self) {
        match self {
            Cmd::Playground { port } => lox_playground::serve(*port),
            Cmd::REPL { debug } => repl(*debug),
            Cmd::Run { path, debug } => run(path, *debug),
        }
    }
}

pub fn playground() {
    // lox_playground::run();
    // lox_playground::;
}

pub fn repl(debug: bool) {
    let mut editor = repl::editor().unwrap();
    let stdout = io::stdout();
    let stdout = stdout.lock();
    let stderr = io::stdout();
    let stderr = stderr.lock();
    let mut vm = VM::new(stdout, stderr, debug);

    loop {
        match editor.read_line(&repl::Prompt) {
            Ok(Signal::Success(line)) => {
                let program = match lox_syntax::parse(&line) {
                    Ok(program) => program,
                    Err(err) => {
                        report_err("<stdin>", &line, err).unwrap();
                        continue;
                    }
                };
                let compiler = Compiler::new();
                let function = compiler.compile(&program).unwrap();
                vm.run(function);
            }
            Ok(Signal::CtrlC) => {
                eprintln!("CTRL-C");
            }
            Ok(Signal::CtrlD) => {
                eprintln!("CTRL-D");
                break;
            }
            Ok(Signal::CtrlL) => {
                if let Err(e) = editor.clear_screen() {
                    eprintln!("error: unable to clear screen: {:?}", e)
                };
            }
            Err(e) => {
                eprintln!("error: {:?}", e);
                break;
            }
        }
    }
}

fn run(path: &str, debug: bool) {
    let source = fs::read_to_string(&path).unwrap();
    let program = match lox_syntax::parse(&source) {
        Ok(program) => program,
        Err(err) => {
            report_err(path, &source, err).unwrap();
            return;
        }
    };
    let compiler = Compiler::new();
    let function = compiler.compile(&program).unwrap();

    let stdout = io::stdout();
    let stdout = stdout.lock();

    let stderr = io::stderr();
    let stderr = stderr.lock();

    let mut vm = VM::new(stdout, stderr, debug);
    vm.run(function);
}

pub fn report_err(name: &str, source: &str, err: ParserError) -> io::Result<()> {
    use codespan_reporting::diagnostic::{Diagnostic, Label};
    use codespan_reporting::files::{Error, SimpleFile};
    use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
    use codespan_reporting::term::{self, Config};

    let (label, range, notes);
    match err {
        ParserError::ExtraToken { token } => {
            label = "unexpected token";
            range = token.0..token.2;
            notes = Vec::new();
        }
        ParserError::InvalidToken { location } => {
            label = "invalid token";
            range = location..location;
            notes = Vec::new();
        }
        ParserError::UnrecognizedEOF { location, expected } => {
            label = "unrecognized EOF";
            range = location..location;
            notes = vec![format!("expected one of: {} after this token", expected.join(", "))];
        }
        ParserError::UnrecognizedToken { token, expected } => {
            label = "unrecognized token";
            range = token.0..token.2;
            notes = vec![format!("expected one of: {} after this token", expected.join(", "))];
        }
        ParserError::User { error: err } => {
            label = "unexpected input";
            range = err.location..err.location + 1;
            notes = Vec::new();
        }
    };

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = Config::default();
    let file = SimpleFile::new(name, source);
    let diagnostic = Diagnostic::error()
        .with_message(label)
        .with_labels(vec![Label::primary((), range)])
        .with_notes(notes);
    term::emit(&mut writer.lock(), &config, &file, &diagnostic).map_err(|err| match err {
        Error::Io(err) => err,
        _ => panic!("invalid error generated: {err}"),
    })?;

    Ok(())
}
