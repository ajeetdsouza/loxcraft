use anyhow::{Context, Result};
use logos::{Lexer, Logos};

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
    #[regex(r#"[0-9]+(\.[0-9]*)?"#, lex_number)]
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
    #[regex(r"[ \r\t\f]+", logos::skip)]
    Error,
}

fn lex_number(lexer: &mut Lexer<Token>) -> Result<f64> {
    let slice = lexer.slice();
    slice
        .parse::<f64>()
        .with_context(|| format!("failed to parse number: {}", slice))
}

fn lex_string(lexer: &mut Lexer<Token>) -> String {
    let slice = lexer.slice();
    slice[1..slice.len() - 1].to_string()
}

fn lex_identifier(lexer: &mut Lexer<Token>) -> String {
    let slice = lexer.slice();
    slice.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_equal() {
        check("==", Token::EqualEqual);
    }

    #[test]
    fn test_number() {
        check("123", Token::Number(123.0));
    }

    #[test]
    fn test_number_decimal() {
        check("123.456", Token::Number(123.456));
    }

    #[test]
    fn test_comment() {
        let mut lexer = Token::lexer("// This is a comment");
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_string_comment() {
        check(r#""// This is a comment""#, Token::String("".to_string()))
    }

    #[test]
    fn test_newline_error() {
        check("\n", Token::Error);
    }

    /// Asserts that the entire input is parsed into the given token.
    fn check(input: &str, token: Token) {
        let mut lexer = Token::lexer(input);
        assert_eq!(lexer.next(), Some(token));
        assert_eq!(lexer.slice(), input);
    }
}
