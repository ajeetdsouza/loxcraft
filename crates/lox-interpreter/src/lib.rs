mod env;
mod interpreter;
mod object;
mod resolver;

use resolver::Resolver;

use lox_common::error::ErrorS;
use lox_syntax::ast::Program;

pub use interpreter::Interpreter;

pub fn resolve(program: &mut Program) -> Vec<ErrorS> {
    Resolver::default().resolve(program)
}
