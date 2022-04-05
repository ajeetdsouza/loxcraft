use logos::Logos;

use std::num::ParseFloatError;

pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { inner: Token::lexer(source) }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(usize, Token, usize), LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let span = self.inner.span();
        match self.inner.next()? {
            Token::Error => Some(Err(LexerError { location: span.start })),
            token => Some(Ok((span.start, token, span.end))),
        }
    }
}

#[derive(Debug)]
pub struct LexerError {
    pub location: usize,
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

    #[error]
    #[regex(r"//.*", logos::skip)]
    #[regex(r"[ \r\n\t\f]+", logos::skip)]
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
