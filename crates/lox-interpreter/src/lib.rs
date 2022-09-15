mod env;
mod interpreter;
mod object;
mod resolver;

pub use interpreter::Interpreter;
use lox_common::error::ErrorS;
use lox_syntax::ast::Program;
use resolver::Resolver;

pub fn resolve(program: &mut Program) -> Vec<ErrorS> {
    Resolver::default().resolve(program)
}
