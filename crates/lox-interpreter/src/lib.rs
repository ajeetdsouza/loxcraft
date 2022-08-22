mod env;
mod interpreter;
mod object;
mod resolver;

use resolver::Resolver;

use lox_common::error::Error;
use lox_syntax::ast::Program;

pub use interpreter::Interpreter;

pub fn resolve(program: &mut Program) -> Vec<Error> {
    Resolver::default().resolve(program)
}
