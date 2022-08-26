use crate::env::Env;
use crate::object::{Callable, Object};
use crate::Interpreter;

use gc::{Finalize, Trace};
use lox_common::error::Result;
use lox_common::types::Span;

use std::fmt::{self, Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Eq, Finalize, Trace, PartialEq)]
pub enum Native {
    Clock,
}

impl Callable for Native {
    fn arity(&self) -> usize {
        match self {
            Native::Clock => 0,
        }
    }

    fn name(&self) -> &str {
        match self {
            Native::Clock => "clock",
        }
    }

    fn call_unchecked(
        &self,
        _interpreter: &mut Interpreter,
        _env: &mut Env,
        _args: Vec<Object>,
        _span: &Span,
    ) -> Result<Object> {
        match self {
            Native::Clock => {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                Ok(Object::Number(now.as_millis() as f64 / 1000.0))
            }
        }
    }
}

impl Display for Native {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<native {}>", self.name())
    }
}

impl Into<Object> for Native {
    fn into(self) -> Object {
        Object::Native(self)
    }
}
