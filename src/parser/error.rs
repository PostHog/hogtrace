//! Error types for the parser

use super::lexer::{Span, Token, TokenKind};
use std::fmt;

/// Result type for parser operations
pub type ParseResult<T> = Result<T, Box<ParseError>>;

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLevel {
    Error,
    Warning,
}

/// Error kind for categorization and better messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedToken,
    UnexpectedEof,
    InvalidToken,
    MissingDelimiter,
    InvalidProbeSpec,
    InvalidExpression,
    InvalidStatement,
    Other,
}

/// Parse error with location information and helpful context
#[derive(Debug, Clone)]
pub struct ParseError {
    pub kind: ErrorKind,
    pub level: ErrorLevel,
    pub message: String,
    pub span: Span,
    pub suggestion: Option<String>,
    pub source: Option<String>,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: String, span: Span) -> Self {
        ParseError {
            kind: ErrorKind::Other,
            level: ErrorLevel::Error,
            message,
            span,
            suggestion: None,
            source: None,
        }
    }

    /// Create an "expected token" error
    pub fn expected(expected: TokenKind, found: Token) -> Self {
        let message = format!("Expected {}, found {}", expected, found.kind);
        let suggestion = Self::suggest_for_expected(&expected, &found.kind);

        ParseError {
            kind: if found.kind == TokenKind::Eof {
                ErrorKind::UnexpectedEof
            } else {
                ErrorKind::UnexpectedToken
            },
            level: ErrorLevel::Error,
            message,
            span: found.span,
            suggestion,
            source: None,
        }
    }

    /// Create a custom error at a specific span
    pub fn at_span(message: impl Into<String>, span: Span) -> Self {
        ParseError {
            kind: ErrorKind::Other,
            level: ErrorLevel::Error,
            message: message.into(),
            span,
            suggestion: None,
            source: None,
        }
    }

    /// Create an error with a specific kind
    pub fn with_kind(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        ParseError {
            kind,
            level: ErrorLevel::Error,
            message: message.into(),
            span,
            suggestion: None,
            source: None,
        }
    }

    /// Add a suggestion to the error
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add source code reference for better error display
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Convert into a boxed error for ParseResult
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    /// Suggest fixes for common "expected token" errors
    fn suggest_for_expected(expected: &TokenKind, found: &TokenKind) -> Option<String> {
        match (expected, found) {
            (TokenKind::Semi, _) => {
                Some("Add a semicolon ';' at the end of the statement".to_string())
            }
            (TokenKind::RBrace, TokenKind::Eof) => {
                Some("Add a closing brace '}' to match the opening brace".to_string())
            }
            (TokenKind::RParen, TokenKind::Eof) => {
                Some("Add a closing parenthesis ')' to match the opening parenthesis".to_string())
            }
            (TokenKind::RBracket, TokenKind::Eof) => {
                Some("Add a closing bracket ']' to match the opening bracket".to_string())
            }
            _ => Option::None,
        }
    }

    /// Format the error with source code context
    pub fn format_with_source(&self, filename: &str) -> String {
        let mut output = String::new();

        // Error header
        output.push_str(&format!(
            "{}: {}\n",
            match self.level {
                ErrorLevel::Error => "Error",
                ErrorLevel::Warning => "Warning",
            },
            self.message
        ));

        // Location
        output.push_str(&format!(
            "  --> {}:{}:{}\n",
            filename, self.span.start.line, self.span.start.column
        ));

        // Source code snippet if available
        if let Some(source) = &self.source {
            let lines: Vec<&str> = source.lines().collect();
            let line_idx = self.span.start.line.saturating_sub(1);

            if line_idx < lines.len() {
                let line = lines[line_idx];
                let line_num = self.span.start.line;
                let line_num_width = line_num.to_string().len();

                output.push_str(&format!("{:width$} |\n", "", width = line_num_width));
                output.push_str(&format!("{} | {}\n", line_num, line));

                // Add caret indicators
                let start_col = self.span.start.column.saturating_sub(1);
                let end_col = if self.span.start.line == self.span.end.line {
                    self.span.end.column.saturating_sub(1)
                } else {
                    line.len()
                };

                let indicator_len = (end_col.saturating_sub(start_col)).max(1);
                output.push_str(&format!(
                    "{:width$} | {:>start$}{:^<len$}\n",
                    "",
                    "",
                    "",
                    width = line_num_width,
                    start = start_col + 1,
                    len = indicator_len
                ));
            }
        }

        // Suggestion if available
        if let Some(suggestion) = &self.suggestion {
            output.push_str(&format!("   = help: {}\n", suggestion));
        }

        output
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error at {}: {}", self.span, self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n  help: {}", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Basic Error Construction Tests =====

    #[test]
    fn test_error_with_suggestion() {
        let span = Span::new(Position::new(1, 5, 4), Position::new(1, 10, 9));
        let err = ParseError::at_span("Something went wrong", span)
            .with_suggestion("Try using 'entry' instead");

        assert!(err.suggestion.is_some());
        assert_eq!(err.suggestion.unwrap(), "Try using 'entry' instead");
    }

    #[test]
    fn test_error_with_source() {
        let span = Span::new(Position::new(1, 5, 4), Position::new(1, 10, 9));
        let source = "fn:myapp.test:entry";
        let err = ParseError::at_span("Test error", span).with_source(source);

        assert!(err.source.is_some());
        assert_eq!(err.source.unwrap(), source);
    }

    #[test]
    fn test_error_with_kind() {
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4));
        let err = ParseError::with_kind(
            ErrorKind::InvalidProbeSpec,
            "Invalid probe specification",
            span,
        );

        assert_eq!(err.kind, ErrorKind::InvalidProbeSpec);
        assert_eq!(err.level, ErrorLevel::Error);
        assert_eq!(err.message, "Invalid probe specification");
    }

    #[test]
    fn test_error_chaining() {
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4));
        let err = ParseError::with_kind(ErrorKind::InvalidExpression, "Invalid expression", span)
            .with_suggestion("Check your syntax")
            .with_source("test + ");

        assert_eq!(err.kind, ErrorKind::InvalidExpression);
        assert_eq!(err.suggestion, Some("Check your syntax".to_string()));
        assert_eq!(err.source, Some("test + ".to_string()));
    }

    // ===== Expected Token Error Tests =====

    #[test]
    fn test_expected_token_error() {
        let token = Token::new(
            TokenKind::Ident("test".to_string()),
            Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4)),
        );
        let err = ParseError::expected(TokenKind::Semi, token);

        assert_eq!(err.kind, ErrorKind::UnexpectedToken);
        assert!(err.suggestion.is_some());
        assert!(err.message.contains("Expected"));
    }

    #[test]
    fn test_expected_semicolon_suggestion() {
        let token = Token::new(
            TokenKind::RBrace,
            Span::new(Position::new(3, 1, 50), Position::new(3, 2, 51)),
        );
        let err = ParseError::expected(TokenKind::Semi, token);

        assert!(err.suggestion.is_some());
        assert!(err.suggestion.unwrap().contains("semicolon"));
    }

    #[test]
    fn test_expected_rbrace_eof() {
        let token = Token::new(
            TokenKind::Eof,
            Span::new(Position::new(10, 1, 200), Position::new(10, 1, 200)),
        );
        let err = ParseError::expected(TokenKind::RBrace, token);

        assert_eq!(err.kind, ErrorKind::UnexpectedEof);
        assert!(err.suggestion.is_some());
        assert!(err.suggestion.unwrap().contains("closing brace"));
    }

    #[test]
    fn test_expected_rparen_eof() {
        let token = Token::new(
            TokenKind::Eof,
            Span::new(Position::new(5, 20, 100), Position::new(5, 20, 100)),
        );
        let err = ParseError::expected(TokenKind::RParen, token);

        assert_eq!(err.kind, ErrorKind::UnexpectedEof);
        assert!(err.suggestion.is_some());
        assert!(err.suggestion.unwrap().contains("closing parenthesis"));
    }

    #[test]
    fn test_expected_rbracket_eof() {
        let token = Token::new(
            TokenKind::Eof,
            Span::new(Position::new(7, 15, 150), Position::new(7, 15, 150)),
        );
        let err = ParseError::expected(TokenKind::RBracket, token);

        assert_eq!(err.kind, ErrorKind::UnexpectedEof);
        assert!(err.suggestion.is_some());
        assert!(err.suggestion.unwrap().contains("closing bracket"));
    }

    #[test]
    fn test_expected_no_suggestion() {
        let token = Token::new(
            TokenKind::Plus,
            Span::new(Position::new(2, 10, 30), Position::new(2, 11, 31)),
        );
        let err = ParseError::expected(TokenKind::Ident("test".to_string()), token);

        // Should have no suggestion for this case
        assert!(err.suggestion.is_none());
    }

    // ===== Error Formatting Tests =====

    #[test]
    fn test_error_format_with_source() {
        let span = Span::new(Position::new(2, 10, 20), Position::new(2, 14, 24));
        let source = "fn:myapp.test:entry\n/ arg0 > 10 /\n{\n    capture(args);\n}";

        let err = ParseError::at_span("Expected semicolon", span)
            .with_source(source)
            .with_suggestion("Add ';' at the end");

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("Error: Expected semicolon"));
        assert!(formatted.contains("test.hogtrace:2:10"));
        assert!(formatted.contains("help: Add ';' at the end"));
    }

    #[test]
    fn test_error_format_single_line() {
        let source = "fn:myapp.test:entr";
        let span = Span::new(Position::new(1, 15, 14), Position::new(1, 19, 18));

        let err = ParseError::with_kind(ErrorKind::InvalidProbeSpec, "Invalid probe point", span)
            .with_source(source)
            .with_suggestion("Did you mean 'entry'?");

        let formatted = err.format_with_source("typo.hogtrace");
        assert!(formatted.contains("Error: Invalid probe point"));
        assert!(formatted.contains("typo.hogtrace:1:15"));
        assert!(formatted.contains("fn:myapp.test:entr"));
        assert!(formatted.contains("help: Did you mean 'entry'?"));
    }

    #[test]
    fn test_error_format_multiline_source() {
        let source = "fn:myapp.test:entry\n{\n    $var = 42\n}";
        let span = Span::new(Position::new(3, 14, 34), Position::new(3, 15, 35));

        let err = ParseError::at_span("Expected semicolon", span)
            .with_source(source)
            .with_suggestion("Add ';' at the end of the statement");

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("3 |"));
        assert!(formatted.contains("$var = 42"));
        assert!(formatted.contains("help: Add ';' at the end"));
    }

    #[test]
    fn test_error_format_without_source() {
        let span = Span::new(Position::new(5, 10, 80), Position::new(5, 15, 85));
        let err = ParseError::at_span("Parse error", span);

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("Error: Parse error"));
        assert!(formatted.contains("test.hogtrace:5:10"));
        // Should not contain source snippet
        assert!(!formatted.contains(" | "));
    }

    #[test]
    fn test_error_format_caret_indicators() {
        let source = "fn:myapp.test:entry";
        let span = Span::new(Position::new(1, 4, 3), Position::new(1, 9, 8));

        let err = ParseError::at_span("Test error", span).with_source(source);

        let formatted = err.format_with_source("test.hogtrace");
        // Should contain line number and source
        assert!(formatted.contains("1 | fn:myapp.test:entry"));
        // Should contain caret indicators (though hard to test exact format)
        assert!(formatted.contains(" | "));
    }

    #[test]
    fn test_error_format_line_number_padding() {
        let source =
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11";
        let span = Span::new(Position::new(11, 1, 60), Position::new(11, 7, 66));

        let err = ParseError::at_span("Error on line 11", span).with_source(source);

        let formatted = err.format_with_source("test.hogtrace");
        // Line number should be properly padded (2 digits)
        assert!(formatted.contains("11 | line11"));
    }

    #[test]
    fn test_error_display_trait() {
        let span = Span::new(Position::new(2, 5, 15), Position::new(2, 10, 20));
        let err = ParseError::at_span("Test error message", span).with_suggestion("Try this fix");

        let display_output = format!("{}", err);
        assert!(display_output.contains("Parse error at"));
        assert!(display_output.contains("Test error message"));
        assert!(display_output.contains("help: Try this fix"));
    }

    #[test]
    fn test_error_display_without_suggestion() {
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4));
        let err = ParseError::at_span("Error without suggestion", span);

        let display_output = format!("{}", err);
        assert!(display_output.contains("Error without suggestion"));
        assert!(!display_output.contains("help:"));
    }

    // ===== Error Kind Tests =====

    #[test]
    fn test_all_error_kinds() {
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4));

        let kinds = vec![
            ErrorKind::UnexpectedToken,
            ErrorKind::UnexpectedEof,
            ErrorKind::InvalidToken,
            ErrorKind::MissingDelimiter,
            ErrorKind::InvalidProbeSpec,
            ErrorKind::InvalidExpression,
            ErrorKind::InvalidStatement,
            ErrorKind::Other,
        ];

        for kind in kinds {
            let err = ParseError::with_kind(kind, "Test", span);
            assert_eq!(err.kind, kind);
        }
    }

    #[test]
    fn test_error_level_variants() {
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4));

        let mut err = ParseError::at_span("Test", span);
        assert_eq!(err.level, ErrorLevel::Error);

        err.level = ErrorLevel::Warning;
        assert_eq!(err.level, ErrorLevel::Warning);
    }

    #[test]
    fn test_warning_level_formatting() {
        let span = Span::new(Position::new(1, 5, 4), Position::new(1, 10, 9));
        let mut err = ParseError::at_span("Test warning", span);
        err.level = ErrorLevel::Warning;

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("Warning: Test warning"));
    }

    // ===== Edge Cases =====

    #[test]
    fn test_error_at_start_of_file() {
        let source = "invalid";
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 8, 7));

        let err = ParseError::at_span("Invalid probe spec", span).with_source(source);

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("test.hogtrace:1:1"));
    }

    #[test]
    fn test_error_at_end_of_file() {
        let source = "fn:myapp.test:entry\n{\n    capture(x)\n}";
        let span = Span::new(Position::new(4, 1, 38), Position::new(4, 2, 39));

        let err = ParseError::at_span("Unexpected end of input", span).with_source(source);

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("test.hogtrace:4:1"));
    }

    #[test]
    fn test_error_spanning_multiple_chars() {
        let source = "fn:myapp.verylongfunction:entry";
        let span = Span::new(Position::new(1, 9, 8), Position::new(1, 26, 25));

        let err = ParseError::at_span("Invalid function name", span).with_source(source);

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("1 | fn:myapp.verylongfunction:entry"));
    }

    #[test]
    fn test_error_with_empty_source() {
        let source = "";
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 1, 0));

        let err = ParseError::at_span("Empty file", span).with_source(source);

        let formatted = err.format_with_source("empty.hogtrace");
        assert!(formatted.contains("Error: Empty file"));
        assert!(formatted.contains("empty.hogtrace:1:1"));
    }

    #[test]
    fn test_error_span_across_lines() {
        let source = "fn:myapp.test:entry\n{\n    capture(\n        arg\n    );\n}";
        // Span from line 3 to line 5
        let span = Span::new(Position::new(3, 5, 25), Position::new(5, 5, 50));

        let err = ParseError::at_span("Multi-line error", span).with_source(source);

        let formatted = err.format_with_source("test.hogtrace");
        assert!(formatted.contains("test.hogtrace:3:5"));
        // Should only show the first line of the span
        assert!(formatted.contains("3 | "));
    }

    #[test]
    fn test_clone_error() {
        let span = Span::new(Position::new(1, 5, 4), Position::new(1, 10, 9));
        let err1 = ParseError::at_span("Test error", span).with_suggestion("Fix it");

        let err2 = err1.clone();
        assert_eq!(err1.message, err2.message);
        assert_eq!(err1.suggestion, err2.suggestion);
        assert_eq!(err1.kind, err2.kind);
    }

    #[test]
    fn test_error_equality() {
        let span = Span::new(Position::new(1, 5, 4), Position::new(1, 10, 9));
        let err1 = ParseError::at_span("Test", span);
        let err2 = ParseError::at_span("Test", span);

        // ParseError implements PartialEq via its fields
        assert_eq!(err1.kind, err2.kind);
        assert_eq!(err1.message, err2.message);
        assert_eq!(err1.span, err2.span);
    }
}
