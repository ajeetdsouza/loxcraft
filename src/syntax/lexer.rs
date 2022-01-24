use crate::syntax::token::Token;

use anyhow::Result;
use logos::Logos;

pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(usize, Token, usize)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next()? {
            Token::Error => panic!("unexpected character '{}'", self.inner.slice()),
            token => {
                let span = self.inner.span();
                Some(Ok((span.start, token, span.end)))
            }
        }
    }
}
