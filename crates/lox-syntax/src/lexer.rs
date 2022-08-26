use logos::Logos;
use lox_common::error::{Error, ErrorS, SyntaxError};

use std::num::ParseFloatError;

pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
    pending: Option<(usize, Token, usize)>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { inner: Token::lexer(source), pending: None }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(usize, Token, usize), ErrorS>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.pending.take() {
            return Some(Ok(token));
        }

        match self.inner.next()? {
            Token::Error => {
                let mut span = self.inner.span();

                // Check for unterminated string.
                if self.inner.slice().starts_with('"') {
                    return Some(Err((
                        Error::SyntaxError(SyntaxError::UnterminatedString),
                        span.clone(),
                    )));
                }

                // Recover error.
                while let Some(token) = self.inner.next() {
                    let span_new = self.inner.span();
                    if span.end == span_new.start {
                        span.end = span_new.end;
                    } else {
                        self.pending = Some((span_new.start, token, span_new.end));
                        break;
                    }
                }

                Some(Err((
                    Error::SyntaxError(SyntaxError::UnexpectedInput {
                        token: self.inner.source()[span.start..span.end].to_string(),
                    }),
                    span.clone(),
                )))
            }
            token => {
                let span = self.inner.span();
                Some(Ok((span.start, token, span.end)))
            }
        }
    }
}

#[derive(Clone, Debug, Logos, PartialEq)]
pub enum Token {
    // Single-character tokens.
    #[token("(")]
    LtParen,
    #[token(")")]
    RtParen,
    #[token("{")]
    LtBrace,
    #[token("}")]
    RtBrace,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("-")]
    Minus,
    #[token("+")]
    Plus,
    #[token(";")]
    Semicolon,
    #[token("/")]
    Slash,
    #[token("*")]
    Asterisk,

    // One or two character tokens.
    #[token("!")]
    Bang,
    #[token("!=")]
    BangEqual,
    #[token("=")]
    Equal,
    #[token("==")]
    EqualEqual,
    #[token(">")]
    Greater,
    #[token(">=")]
    GreaterEqual,
    #[token("<")]
    Less,
    #[token("<=")]
    LessEqual,

    // Literals.
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", lex_identifier)]
    Identifier(String),
    #[regex(r#""[^"]*""#, lex_string)]
    String(String),
    #[regex(r#"[0-9]+(\.[0-9]+)?"#, lex_number)]
    Number(f64),

    // Keywords.
    #[token("and")]
    And,
    #[token("class")]
    Class,
    #[token("else")]
    Else,
    #[token("false")]
    False,
    #[token("for")]
    For,
    #[token("fun")]
    Fun,
    #[token("if")]
    If,
    #[token("nil")]
    Nil,
    #[token("or")]
    Or,
    #[token("print")]
    Print,
    #[token("return")]
    Return,
    #[token("super")]
    Super,
    #[token("this")]
    This,
    #[token("true")]
    True,
    #[token("var")]
    Var,
    #[token("while")]
    While,

    #[regex(r"//.*", logos::skip)]
    #[regex(r"[ \r\n\t\f]+", logos::skip)]
    #[error]
    Error,
}

fn lex_number(lexer: &mut logos::Lexer<Token>) -> Result<f64, ParseFloatError> {
    let slice = lexer.slice();
    slice.parse::<f64>()
}

fn lex_string(lexer: &mut logos::Lexer<Token>) -> String {
    let slice = lexer.slice();
    slice[1..slice.len() - 1].to_string()
}

fn lex_identifier(lexer: &mut logos::Lexer<Token>) -> String {
    let slice = lexer.slice();
    slice.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn lex_invalid_token() {
        let exp = vec![
            Err((
                Error::SyntaxError(SyntaxError::UnexpectedInput { token: "@foo".to_string() }),
                0..4,
            )),
            Ok((5, Token::Identifier("bar".to_string()), 8)),
        ];
        let got = Lexer::new("@foo bar").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }

    #[test]
    fn lex_unterminated_string() {
        let exp = vec![Err((Error::SyntaxError(SyntaxError::UnterminatedString), 0..5))];
        let got = Lexer::new("\"\nfoo").collect::<Vec<_>>();
        assert_eq!(exp, got);
    }
}
