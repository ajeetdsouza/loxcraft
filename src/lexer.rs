use logos::Logos;

use crate::token::Token;

pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
    line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
            line: 1,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = (Token, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let token = self.inner.next()?;
            let slice = self.inner.slice();

            if token == Token::Error && slice == "\n" {
                self.line += 1
            } else {
                return Some((token, self.inner.slice()));
            }
        }
    }
}
