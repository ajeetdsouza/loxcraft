use lox_common::error::{Error, Result, TypeError};
use lox_common::types::Span;

use crate::env::Env;
use crate::object::Object;
use crate::Interpreter;

pub trait Callable {
    fn arity(&self) -> usize;

    fn name(&self) -> &str;

    fn call_unchecked(
        &self,
        interpreter: &mut Interpreter,
        env: &mut Env,
        args: Vec<Object>,
        span: &Span,
    ) -> Result<Object>;

    fn call(&self, interpreter: &mut Interpreter, env: &mut Env, args: Vec<Object>, span: &Span) -> Result<Object> {
        let exp_args = self.arity() as u8;
        let got_args = args.len() as u8;
        if exp_args != got_args {
            return Err((
                Error::TypeError(TypeError::ArityMismatch { name: self.name().to_string(), exp_args, got_args }),
                span.clone(),
            ));
        }
        self.call_unchecked(interpreter, env, args, span)
    }
}
