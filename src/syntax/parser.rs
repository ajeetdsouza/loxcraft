use lalrpop_util::{lalrpop_mod, ParseError};

use crate::syntax::lexer::{LexerError, Token};

pub type Parser = grammar::ProgramParser;

pub type ParserError = ParseError<usize, Token, LexerError>;

lalrpop_mod!(grammar, "/syntax/grammar.rs");
