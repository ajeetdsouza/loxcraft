use logos::Logos;

#[derive(Clone, Copy, Debug, Eq, Hash, Logos, Ord, PartialEq, PartialOrd)]
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
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    #[regex(r#""[^"]*""#)]
    String,
    #[regex(r#"[0-9]+(\.[0-9]*)?"#)]
    Number,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_equal() {
        check("==", Token::EqualEqual);
    }

    #[test]
    fn test_number() {
        check("123", Token::Number);
    }

    #[test]
    fn test_number_decimal() {
        check("123.456", Token::Number);
    }

    #[test]
    fn test_comment() {
        let mut lexer = Token::lexer("// This is a comment");
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_string_comment() {
        check(r#""// This is a comment""#, Token::String)
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
