//! Lexer for HogTrace language
//!
//! Converts source text into a stream of tokens with position tracking.

use std::fmt;

/// Position in source code (line and column, both 1-indexed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Position {
            line,
            column,
            offset,
        }
    }

    pub fn start() -> Self {
        Position {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Span represents a range in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Span { start, end }
    }

    pub fn single(pos: Position) -> Self {
        Span {
            start: pos,
            end: pos,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start.line == self.end.line {
            write!(
                f,
                "{}:{}-{}",
                self.start.line, self.start.column, self.end.column
            )
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// Token kinds in HogTrace language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    None,

    // Identifiers and keywords
    Ident(String),

    // Keywords
    Fn,      // fn
    Py,      // py
    Entry,   // entry
    Exit,    // exit
    Capture, // capture
    Send,    // send
    Sample,  // sample
    Req,     // $req
    Request, // $request

    // Operators
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %

    // Comparison
    Lt,    // <
    Gt,    // >
    LtEq,  // <=
    GtEq,  // >=
    EqEq,  // ==
    NotEq, // !=

    // Logical
    And, // &&
    Or,  // ||
    Not, // !

    // Assignment
    Eq, // =

    // Delimiters
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]

    // Punctuation
    Colon,  // :
    Semi,   // ;
    Comma,  // ,
    Dot,    // .
    Dollar, // $

    // Special
    FSlash,   // / (used in predicates /expr/)
    Wildcard, // * (when used as wildcard in module paths)

    // End of file
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Int(n) => write!(f, "integer '{}'", n),
            TokenKind::Float(n) => write!(f, "float '{}'", n),
            TokenKind::String(s) => write!(f, "string \"{}\"", s),
            TokenKind::Bool(b) => write!(f, "boolean '{}'", b),
            TokenKind::None => write!(f, "None"),
            TokenKind::Ident(s) => write!(f, "identifier '{}'", s),
            TokenKind::Fn => write!(f, "'fn'"),
            TokenKind::Py => write!(f, "'py'"),
            TokenKind::Entry => write!(f, "'entry'"),
            TokenKind::Exit => write!(f, "'exit'"),
            TokenKind::Capture => write!(f, "'capture'"),
            TokenKind::Send => write!(f, "'send'"),
            TokenKind::Sample => write!(f, "'sample'"),
            TokenKind::Req => write!(f, "'$req'"),
            TokenKind::Request => write!(f, "'$request'"),
            TokenKind::Plus => write!(f, "'+'"),
            TokenKind::Minus => write!(f, "'-'"),
            TokenKind::Star => write!(f, "'*'"),
            TokenKind::Slash => write!(f, "'/'"),
            TokenKind::Percent => write!(f, "'%'"),
            TokenKind::Lt => write!(f, "'<'"),
            TokenKind::Gt => write!(f, "'>'"),
            TokenKind::LtEq => write!(f, "'<='"),
            TokenKind::GtEq => write!(f, "'>='"),
            TokenKind::EqEq => write!(f, "'=='"),
            TokenKind::NotEq => write!(f, "'!='"),
            TokenKind::And => write!(f, "'&&'"),
            TokenKind::Or => write!(f, "'||'"),
            TokenKind::Not => write!(f, "'!'"),
            TokenKind::Eq => write!(f, "'='"),
            TokenKind::LParen => write!(f, "'('"),
            TokenKind::RParen => write!(f, "')'"),
            TokenKind::LBrace => write!(f, "'{{'"),
            TokenKind::RBrace => write!(f, "'}}'"),
            TokenKind::LBracket => write!(f, "'['"),
            TokenKind::RBracket => write!(f, "']'"),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::Semi => write!(f, "';'"),
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Dot => write!(f, "'.'"),
            TokenKind::Dollar => write!(f, "'$'"),
            TokenKind::FSlash => write!(f, "'/'"),
            TokenKind::Wildcard => write!(f, "'*'"),
            TokenKind::Eof => write!(f, "end of file"),
        }
    }
}

/// A token with its kind and source location
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }
}

/// The lexer tokenizes HogTrace source code
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::str::Chars<'a>,
    current: Option<char>,
    position: Position,
    token_start: Position,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source
    pub fn new(source: &'a str) -> Self {
        let mut chars = source.chars();
        let current = chars.next();
        Lexer {
            source,
            chars,
            current,
            position: Position::start(),
            token_start: Position::start(),
        }
    }

    /// Get the source code (useful for error reporting)
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();
        self.token_start = self.position;

        match self.current {
            None => self.make_token(TokenKind::Eof),
            Some(ch) => match ch {
                // Single-character tokens
                '(' => self.single_char_token(TokenKind::LParen),
                ')' => self.single_char_token(TokenKind::RParen),
                '{' => self.single_char_token(TokenKind::LBrace),
                '}' => self.single_char_token(TokenKind::RBrace),
                '[' => self.single_char_token(TokenKind::LBracket),
                ']' => self.single_char_token(TokenKind::RBracket),
                ';' => self.single_char_token(TokenKind::Semi),
                ',' => self.single_char_token(TokenKind::Comma),
                '.' => self.single_char_token(TokenKind::Dot),
                '%' => self.single_char_token(TokenKind::Percent),
                ':' => self.single_char_token(TokenKind::Colon),

                // Operators (potentially multi-character)
                '+' => self.single_char_token(TokenKind::Plus),
                '-' => self.single_char_token(TokenKind::Minus),
                '*' => self.single_char_token(TokenKind::Star),
                '/' => self.single_char_token(TokenKind::Slash),

                '!' => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        self.make_token(TokenKind::NotEq)
                    } else {
                        self.make_token(TokenKind::Not)
                    }
                }

                '=' => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        self.make_token(TokenKind::EqEq)
                    } else {
                        self.make_token(TokenKind::Eq)
                    }
                }

                '<' => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        self.make_token(TokenKind::LtEq)
                    } else {
                        self.make_token(TokenKind::Lt)
                    }
                }

                '>' => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        self.make_token(TokenKind::GtEq)
                    } else {
                        self.make_token(TokenKind::Gt)
                    }
                }

                '&' => {
                    self.advance();
                    if self.current == Some('&') {
                        self.advance();
                        self.make_token(TokenKind::And)
                    } else {
                        // Invalid single &
                        self.make_token(TokenKind::Ident("&".to_string()))
                    }
                }

                '|' => {
                    self.advance();
                    if self.current == Some('|') {
                        self.advance();
                        self.make_token(TokenKind::Or)
                    } else {
                        // Invalid single |
                        self.make_token(TokenKind::Ident("|".to_string()))
                    }
                }

                // Dollar (for $req, $request)
                '$' => self.lex_dollar_ident(),

                // String literals
                '"' | '\'' => self.lex_string(ch),

                // Numbers
                '0'..='9' => self.lex_number(),

                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => self.lex_ident(),

                // Unknown character
                _ => {
                    self.advance();
                    self.make_token(TokenKind::Ident(ch.to_string()))
                }
            },
        }
    }

    /// Advance to the next character
    fn advance(&mut self) {
        if let Some(ch) = self.current {
            if ch == '\n' {
                self.position.line += 1;
                self.position.column = 1;
            } else {
                self.position.column += 1;
            }
            self.position.offset += ch.len_utf8();
        }
        self.current = self.chars.next();
    }

    /// Peek at the next character without advancing
    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.advance();
                }
                Some('#') => {
                    // Line comment
                    self.advance();
                    while self.current.is_some() && self.current != Some('\n') {
                        self.advance();
                    }
                }
                Some('/') if self.peek() == Some('*') => {
                    // Block comment
                    self.advance(); // /
                    self.advance(); // *
                    while self.current.is_some() {
                        if self.current == Some('*') && self.peek() == Some('/') {
                            self.advance(); // *
                            self.advance(); // /
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    /// Create a token from the current token_start to current position
    fn make_token(&self, kind: TokenKind) -> Token {
        Token::new(kind, Span::new(self.token_start, self.position))
    }

    /// Consume a single character and create a token
    fn single_char_token(&mut self, kind: TokenKind) -> Token {
        self.advance();
        self.make_token(kind)
    }

    /// Lex a string literal
    fn lex_string(&mut self, quote: char) -> Token {
        self.advance(); // consume opening quote
        let mut value = String::new();

        while self.current.is_some() && self.current != Some(quote) {
            if self.current == Some('\\') {
                self.advance();
                // Handle escape sequences
                match self.current {
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('\\') => value.push('\\'),
                    Some('"') => value.push('"'),
                    Some('\'') => value.push('\''),
                    Some(ch) => value.push(ch), // Unknown escape, keep as-is
                    None => break,
                }
                self.advance();
            } else {
                value.push(self.current.unwrap());
                self.advance();
            }
        }

        if self.current == Some(quote) {
            self.advance(); // consume closing quote
        }

        self.make_token(TokenKind::String(value))
    }

    /// Lex a number (integer or float)
    fn lex_number(&mut self) -> Token {
        let mut value = String::new();
        let mut is_float = false;

        // Integer part
        while let Some('0'..='9') = self.current {
            value.push(self.current.unwrap());
            self.advance();
        }

        // Decimal point
        if self.current == Some('.') && matches!(self.peek(), Some('0'..='9')) {
            is_float = true;
            value.push('.');
            self.advance();

            while let Some('0'..='9') = self.current {
                value.push(self.current.unwrap());
                self.advance();
            }
        }

        // Exponent
        if matches!(self.current, Some('e') | Some('E')) {
            is_float = true;
            value.push(self.current.unwrap());
            self.advance();

            if matches!(self.current, Some('+') | Some('-')) {
                value.push(self.current.unwrap());
                self.advance();
            }

            while let Some('0'..='9') = self.current {
                value.push(self.current.unwrap());
                self.advance();
            }
        }

        if is_float {
            let num = value.parse::<f64>().unwrap_or(0.0);
            self.make_token(TokenKind::Float(num))
        } else {
            let num = value.parse::<i64>().unwrap_or(0);
            self.make_token(TokenKind::Int(num))
        }
    }

    /// Lex an identifier or keyword
    fn lex_ident(&mut self) -> Token {
        let mut value = String::new();

        while let Some(ch) = self.current {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match value.as_str() {
            "fn" => TokenKind::Fn,
            "py" => TokenKind::Py,
            "entry" => TokenKind::Entry,
            "exit" => TokenKind::Exit,
            "capture" => TokenKind::Capture,
            "send" => TokenKind::Send,
            "sample" => TokenKind::Sample,
            "True" => TokenKind::Bool(true),
            "False" => TokenKind::Bool(false),
            "None" => TokenKind::None,
            _ => TokenKind::Ident(value),
        };

        self.make_token(kind)
    }

    /// Lex $req or $request
    fn lex_dollar_ident(&mut self) -> Token {
        self.advance(); // consume $

        let mut value = String::new();
        while let Some(ch) = self.current {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match value.as_str() {
            "req" => TokenKind::Req,
            "request" => TokenKind::Request,
            _ => TokenKind::Ident(format!("${}", value)),
        };

        self.make_token(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_all(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn test_empty() {
        let tokens = lex_all("");
        assert_eq!(kinds(&tokens), vec![TokenKind::Eof]);
    }

    #[test]
    fn test_whitespace() {
        let tokens = lex_all("   \t\n\r\n  ");
        assert_eq!(kinds(&tokens), vec![TokenKind::Eof]);
    }

    #[test]
    fn test_single_char_tokens() {
        let tokens = lex_all("(){}[];:,.");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Semi,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_operators() {
        let tokens = lex_all("+ - * / % ! && || == != < > <= >=");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Not,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_assignment() {
        let tokens = lex_all("x = 5");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Ident("x".to_string()),
                TokenKind::Eq,
                TokenKind::Int(5),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let tokens = lex_all("fn py entry exit capture send sample True False None");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Fn,
                TokenKind::Py,
                TokenKind::Entry,
                TokenKind::Exit,
                TokenKind::Capture,
                TokenKind::Send,
                TokenKind::Sample,
                TokenKind::Bool(true),
                TokenKind::Bool(false),
                TokenKind::None,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_identifiers() {
        let tokens = lex_all("foo bar _baz test123 MyClass");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Ident("foo".to_string()),
                TokenKind::Ident("bar".to_string()),
                TokenKind::Ident("_baz".to_string()),
                TokenKind::Ident("test123".to_string()),
                TokenKind::Ident("MyClass".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_integers() {
        let tokens = lex_all("0 42 123 999");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Int(0),
                TokenKind::Int(42),
                TokenKind::Int(123),
                TokenKind::Int(999),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_floats() {
        let tokens = lex_all("3.15 0.5 2.0 1.5e10 1e-5 2.5E+3");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Float(3.15),
                TokenKind::Float(0.5),
                TokenKind::Float(2.0),
                TokenKind::Float(1.5e10),
                TokenKind::Float(1e-5),
                TokenKind::Float(2.5E+3),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_strings_double_quote() {
        let tokens = lex_all(r#""hello" "world""#);
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::String("hello".to_string()),
                TokenKind::String("world".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_strings_single_quote() {
        let tokens = lex_all("'hello' 'world'");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::String("hello".to_string()),
                TokenKind::String("world".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_string_escapes() {
        let tokens = lex_all(r#""hello\nworld" "tab\there" "quote\"inside""#);
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::String("hello\nworld".to_string()),
                TokenKind::String("tab\there".to_string()),
                TokenKind::String("quote\"inside".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_dollar_req() {
        let tokens = lex_all("$req $request");
        assert_eq!(
            kinds(&tokens),
            vec![TokenKind::Req, TokenKind::Request, TokenKind::Eof]
        );
    }

    #[test]
    fn test_line_comment() {
        let tokens = lex_all("foo # this is a comment\nbar");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Ident("foo".to_string()),
                TokenKind::Ident("bar".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_block_comment() {
        let tokens = lex_all("foo /* this is a\nmulti-line comment */ bar");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Ident("foo".to_string()),
                TokenKind::Ident("bar".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_position_tracking() {
        let mut lexer = Lexer::new("foo\n  bar");

        let tok1 = lexer.next_token();
        assert_eq!(tok1.kind, TokenKind::Ident("foo".to_string()));
        assert_eq!(tok1.span.start.line, 1);
        assert_eq!(tok1.span.start.column, 1);

        let tok2 = lexer.next_token();
        assert_eq!(tok2.kind, TokenKind::Ident("bar".to_string()));
        assert_eq!(tok2.span.start.line, 2);
        assert_eq!(tok2.span.start.column, 3);
    }

    #[test]
    fn test_complex_expression() {
        let tokens = lex_all("arg0 > 10 && arg1.name == \"test\"");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Ident("arg0".to_string()),
                TokenKind::Gt,
                TokenKind::Int(10),
                TokenKind::And,
                TokenKind::Ident("arg1".to_string()),
                TokenKind::Dot,
                TokenKind::Ident("name".to_string()),
                TokenKind::EqEq,
                TokenKind::String("test".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_probe_spec() {
        let tokens = lex_all("fn:myapp.test:entry");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Fn,
                TokenKind::Colon,
                TokenKind::Ident("myapp".to_string()),
                TokenKind::Dot,
                TokenKind::Ident("test".to_string()),
                TokenKind::Colon,
                TokenKind::Entry,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_function_call() {
        let tokens = lex_all("capture(args)");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Capture,
                TokenKind::LParen,
                TokenKind::Ident("args".to_string()),
                TokenKind::RParen,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_array_indexing() {
        let tokens = lex_all("arr[0]");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Ident("arr".to_string()),
                TokenKind::LBracket,
                TokenKind::Int(0),
                TokenKind::RBracket,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_sample_percentage() {
        let tokens = lex_all("sample 10%;");
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Sample,
                TokenKind::Int(10),
                TokenKind::Percent,
                TokenKind::Semi,
                TokenKind::Eof
            ]
        );
    }
}
