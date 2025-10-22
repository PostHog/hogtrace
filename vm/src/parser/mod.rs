//! HogTrace recursive descent parser
//!
//! This module provides a complete parser for the HogTrace language, converting
//! source text into an AST and then compiling it to VM bytecode.

pub mod lexer;
pub mod ast;
pub mod error;
pub mod compiler;

#[cfg(test)]
mod tests;

pub use lexer::{Lexer, Token, TokenKind, Span};
pub use ast::*;
pub use error::{ParseError, ParseResult, ErrorKind};
pub use compiler::Compiler;

use crate::Program;

/// Parse HogTrace source code into a Program
pub fn parse(source: &str) -> ParseResult<Program> {
    let lexer = Lexer::new(source);
    let ast = Parser::new(lexer).parse_program()?;
    Compiler::new().compile(ast)
}

/// The main parser struct
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    peek: Token,
    source: &'a str,
}

impl<'a> Parser<'a> {
    /// Create a new parser from a lexer
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let source = lexer.source();
        let current = lexer.next_token();
        let peek = lexer.next_token();
        Parser {
            lexer,
            current,
            peek,
            source,
        }
    }

    /// Add source context to an error
    fn enrich_error(&self, error: ParseError) -> ParseError {
        error.with_source(self.source)
    }

    /// Parse a complete HogTrace program
    pub fn parse_program(&mut self) -> ParseResult<AstProgram> {
        let start = self.current.span.start;
        let mut probes = Vec::new();

        while self.current.kind != TokenKind::Eof {
            probes.push(self.parse_probe()?);
        }

        let end = if let Some(last) = probes.last() {
            last.span.end
        } else {
            start
        };

        Ok(AstProgram {
            probes,
            span: Span::new(start, end),
        })
    }

    /// Parse a single probe definition
    fn parse_probe(&mut self) -> ParseResult<AstProbe> {
        let start = self.current.span.start;

        // Parse probe specification
        let spec = self.parse_probe_spec()?;

        // Parse optional predicate
        let predicate = if self.check(&TokenKind::Slash) {
            Some(self.parse_predicate()?)
        } else {
            None
        };

        // Parse action block
        self.expect(TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && self.current.kind != TokenKind::Eof {
            body.push(self.parse_statement()?);
        }
        let close = self.expect(TokenKind::RBrace)?;

        Ok(AstProbe {
            spec,
            predicate,
            body,
            span: Span::new(start, close.span.end),
        })
    }

    /// Parse probe specification: fn:module.function:entry
    fn parse_probe_spec(&mut self) -> ParseResult<ProbeSpec> {
        let start = self.current.span.start;

        // Parse provider (fn or py)
        let provider = match &self.current.kind {
            TokenKind::Fn => {
                self.advance();
                Provider::Fn
            }
            TokenKind::Py => {
                self.advance();
                Provider::Py
            }
            _ => {
                return Err(self.enrich_error(
                    ParseError::with_kind(
                        ErrorKind::InvalidProbeSpec,
                        "Expected 'fn' or 'py' at start of probe specification",
                        self.current.span,
                    )
                    .with_suggestion("Probe specifications must start with 'fn:' or 'py:'")
                ))
            }
        };

        self.expect(TokenKind::Colon)?;

        // Parse module.function path
        let module_function = self.parse_module_function()?;

        self.expect(TokenKind::Colon)?;

        // Parse probe point (entry, exit, entry+N, exit+N)
        let probe_point = self.parse_probe_point()?;

        let end = self.current.span.start;

        Ok(ProbeSpec {
            provider,
            module_function,
            probe_point,
            span: Span::new(start, end),
        })
    }

    /// Parse module.function path (e.g., myapp.users.create or myapp.*)
    fn parse_module_function(&mut self) -> ParseResult<ModuleFunction> {
        let start = self.current.span.start;
        let mut parts = Vec::new();

        loop {
            match &self.current.kind {
                TokenKind::Ident(name) => {
                    parts.push(ModulePart::Ident(name.clone()));
                    self.advance();
                }
                TokenKind::Star => {
                    parts.push(ModulePart::Wildcard);
                    self.advance();
                }
                _ => {
                    return Err(ParseError::at_span(
                        "Expected identifier or '*'",
                        self.current.span,
                    ))
                }
            }

            if !self.check(&TokenKind::Dot) {
                break;
            }
            self.advance(); // consume dot
        }

        if parts.is_empty() {
            return Err(ParseError::at_span(
                "Expected module/function path",
                self.current.span,
            ));
        }

        Ok(ModuleFunction {
            parts,
            span: Span::new(start, self.current.span.start),
        })
    }

    /// Parse probe point (entry, exit, entry+N, exit+N)
    fn parse_probe_point(&mut self) -> ParseResult<ProbePoint> {
        match &self.current.kind {
            TokenKind::Entry => {
                self.advance();
                // Check for +offset
                if self.check(&TokenKind::Plus) {
                    self.advance();
                    if let TokenKind::Int(n) = self.current.kind {
                        let offset = n;
                        self.advance();
                        Ok(ProbePoint::EntryOffset(offset))
                    } else {
                        Err(ParseError::at_span(
                            "Expected integer offset",
                            self.current.span,
                        ))
                    }
                } else {
                    Ok(ProbePoint::Entry)
                }
            }
            TokenKind::Exit => {
                self.advance();
                // Check for +offset
                if self.check(&TokenKind::Plus) {
                    self.advance();
                    if let TokenKind::Int(n) = self.current.kind {
                        let offset = n;
                        self.advance();
                        Ok(ProbePoint::ExitOffset(offset))
                    } else {
                        Err(ParseError::at_span(
                            "Expected integer offset",
                            self.current.span,
                        ))
                    }
                } else {
                    Ok(ProbePoint::Exit)
                }
            }
            _ => {
                // Check for common typos
                let suggestion = if let TokenKind::Ident(s) = &self.current.kind {
                    match s.as_str() {
                        "entr" | "entyr" | "entre" => Some("Did you mean 'entry'?".to_string()),
                        "exi" | "ext" | "exti" => Some("Did you mean 'exit'?".to_string()),
                        _ => Some("Probe points must be 'entry', 'exit', 'entry+N', or 'exit+N'".to_string()),
                    }
                } else {
                    Some("Probe points must be 'entry', 'exit', 'entry+N', or 'exit+N'".to_string())
                };

                Err(self.enrich_error(
                    ParseError::with_kind(
                        ErrorKind::InvalidProbeSpec,
                        format!("Expected 'entry' or 'exit', found {}", self.current.kind),
                        self.current.span,
                    )
                    .with_suggestion(suggestion.unwrap())
                ))
            }
        }
    }

    /// Parse predicate: /expression/
    fn parse_predicate(&mut self) -> ParseResult<AstExpr> {
        // Note: The first '/' is consumed as part of checking in parse_probe
        // Actually no - we check for it but don't consume it. Let's consume it here.
        self.advance(); // consume opening '/'

        // Parse expression. The expression parser will stop when it sees
        // a '/' that would be a division operator at the current precedence,
        // but we need to be smarter. For now, we'll parse the full expression
        // and rely on the fact that division has higher precedence than most
        // operators, so top-level predicates won't have unparenthesized division.

        // Actually, the issue is that parse_expr will consume '/' as division.
        // We need a special context for this. For now, let's just parse
        // until we can't continue, then check for the closing '/'.

        let expr = self.parse_predicate_expr()?;
        self.advance(); // consume closing '/'
        Ok(expr)
    }

    /// Parse expression inside a predicate (stops at unmatched '/')
    fn parse_predicate_expr(&mut self) -> ParseResult<AstExpr> {
        // We'll use a special version that tracks paren depth
        // and stops at '/' when not inside parens/brackets
        self.parse_predicate_expr_with_precedence(0, 0)
    }

    /// Parse predicate expression with precedence, tracking delimiter depth
    fn parse_predicate_expr_with_precedence(&mut self, min_precedence: u8, depth: usize) -> ParseResult<AstExpr> {
        let mut left = self.parse_unary_expr_for_predicate(depth)?;
        left = self.parse_postfix_expr_for_predicate(left, depth)?;

        while let Some((op, precedence)) = self.current_binary_op_for_predicate(depth) {
            if precedence < min_precedence {
                break;
            }

            self.advance(); // consume operator
            let right = self.parse_predicate_expr_with_precedence(precedence + 1, depth)?;
            let span = Span::new(left.span().start, right.span().end);
            left = AstExpr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary_expr_for_predicate(&mut self, depth: usize) -> ParseResult<AstExpr> {
        match &self.current.kind {
            TokenKind::Not => {
                let start = self.current.span.start;
                self.advance();
                let expr = self.parse_unary_expr_for_predicate(depth)?;
                let span = Span::new(start, expr.span().end);
                Ok(AstExpr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span,
                })
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_predicate_expr_with_precedence(0, depth + 1)?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => self.parse_primary_expr(),
        }
    }

    fn parse_postfix_expr_for_predicate(&mut self, mut expr: AstExpr, depth: usize) -> ParseResult<AstExpr> {
        loop {
            match &self.current.kind {
                TokenKind::Dot => {
                    self.advance();
                    let field_tok = self.expect(TokenKind::Ident("".to_string()))?;
                    let field = if let TokenKind::Ident(s) = field_tok.kind {
                        s
                    } else {
                        return Err(ParseError::at_span(
                            "Expected field name after '.'",
                            field_tok.span,
                        ));
                    };
                    let span = Span::new(expr.span().start, field_tok.span.end);
                    expr = AstExpr::FieldAccess {
                        object: Box::new(expr),
                        field,
                        span,
                    };
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_predicate_expr_with_precedence(0, depth + 1)?;
                    let close = self.expect(TokenKind::RBracket)?;
                    let span = Span::new(expr.span().start, close.span.end);
                    expr = AstExpr::IndexAccess {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn current_binary_op_for_predicate(&self, depth: usize) -> Option<(BinaryOp, u8)> {
        // If we're at depth 0 and see '/', it's the predicate terminator, not division
        if depth == 0 && matches!(self.current.kind, TokenKind::Slash) {
            return None;
        }
        self.current_binary_op()
    }

    /// Parse a statement
    pub fn parse_statement(&mut self) -> ParseResult<AstStatement> {
        match &self.current.kind {
            // Assignment: $req.field = expr;
            TokenKind::Req | TokenKind::Request => self.parse_assignment(),

            // Sample directive: sample 10%;
            TokenKind::Sample => self.parse_sample_directive(),

            // Capture/send statement: capture(...); or send(...);
            TokenKind::Capture | TokenKind::Send => self.parse_capture_statement(),

            _ => Err(ParseError::at_span(
                format!("Expected statement, found {}", self.current.kind),
                self.current.span,
            )),
        }
    }

    /// Parse assignment statement: $req.field = expr;
    fn parse_assignment(&mut self) -> ParseResult<AstStatement> {
        let start = self.current.span.start;
        let is_request = matches!(self.current.kind, TokenKind::Request);
        self.advance();

        self.expect(TokenKind::Dot)?;
        let field_tok = self.expect(TokenKind::Ident("".to_string()))?;
        let field = if let TokenKind::Ident(s) = field_tok.kind {
            s
        } else {
            return Err(ParseError::at_span(
                "Expected field name",
                field_tok.span,
            ));
        };

        let var_span = Span::new(start, field_tok.span.end);
        let var = RequestVar {
            is_request,
            field,
            span: var_span,
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let semi = self.expect(TokenKind::Semi)?;

        Ok(AstStatement::Assignment {
            var,
            value,
            span: Span::new(start, semi.span.end),
        })
    }

    /// Parse sample directive: sample 10%; or sample 1/10;
    fn parse_sample_directive(&mut self) -> ParseResult<AstStatement> {
        let start = self.current.span.start;
        self.advance(); // consume 'sample'

        let numerator = if let TokenKind::Int(n) = self.current.kind {
            let num = n;
            self.advance();
            num
        } else {
            return Err(ParseError::at_span("Expected integer", self.current.span));
        };

        let spec = if self.check(&TokenKind::Percent) {
            self.advance();
            SampleSpec::Percentage(numerator)
        } else if self.check(&TokenKind::Slash) {
            self.advance();
            let denominator = if let TokenKind::Int(n) = self.current.kind {
                let num = n;
                self.advance();
                num
            } else {
                return Err(ParseError::at_span("Expected integer", self.current.span));
            };
            SampleSpec::Ratio { numerator, denominator }
        } else {
            return Err(ParseError::at_span(
                "Expected '%' or '/' after sample number",
                self.current.span,
            ));
        };

        let semi = self.expect(TokenKind::Semi)?;

        Ok(AstStatement::Sample {
            spec,
            span: Span::new(start, semi.span.end),
        })
    }

    /// Parse capture/send statement: capture(...); or send(...);
    fn parse_capture_statement(&mut self) -> ParseResult<AstStatement> {
        let start = self.current.span.start;
        let is_send = matches!(self.current.kind, TokenKind::Send);
        self.advance();

        self.expect(TokenKind::LParen)?;

        // Parse arguments (could be empty, positional, or named)
        let args = if self.check(&TokenKind::RParen) {
            CaptureArgs::Positional(vec![])
        } else {
            self.parse_capture_args()?
        };

        self.expect(TokenKind::RParen)?;
        let semi = self.expect(TokenKind::Semi)?;

        Ok(AstStatement::Capture {
            is_send,
            args,
            span: Span::new(start, semi.span.end),
        })
    }

    /// Parse capture arguments (named or positional)
    fn parse_capture_args(&mut self) -> ParseResult<CaptureArgs> {
        // Check if first argument is named (ident=expr)
        if let TokenKind::Ident(_) = &self.current.kind {
            if let TokenKind::Eq = &self.peek.kind {
                // Named arguments
                return self.parse_named_capture_args();
            }
        }

        // Positional arguments
        let mut args = vec![];
        loop {
            args.push(self.parse_expr()?);
            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance(); // consume comma
        }
        Ok(CaptureArgs::Positional(args))
    }

    /// Parse named capture arguments
    fn parse_named_capture_args(&mut self) -> ParseResult<CaptureArgs> {
        let mut args = vec![];
        loop {
            let start = self.current.span.start;
            let name_tok = self.expect(TokenKind::Ident("".to_string()))?;
            let name = if let TokenKind::Ident(s) = name_tok.kind {
                s
            } else {
                return Err(ParseError::at_span("Expected identifier", name_tok.span));
            };

            self.expect(TokenKind::Eq)?;
            let value = self.parse_expr()?;
            let span = Span::new(start, value.span().end);

            args.push(NamedArg { name, value, span });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance(); // consume comma
        }
        Ok(CaptureArgs::Named(args))
    }

    /// Parse an expression
    pub fn parse_expr(&mut self) -> ParseResult<AstExpr> {
        self.parse_expr_with_precedence(0)
    }

    /// Parse expression with operator precedence climbing
    fn parse_expr_with_precedence(&mut self, min_precedence: u8) -> ParseResult<AstExpr> {
        // Parse left-hand side (primary expression with possible unary operators)
        let mut left = self.parse_unary_expr()?;

        // Handle postfix operators (field access, indexing, function calls)
        left = self.parse_postfix_expr(left)?;

        // Parse binary operators with precedence climbing
        while let Some((op, precedence)) = self.current_binary_op() {
            if precedence < min_precedence {
                break;
            }

            self.advance(); // consume operator
            let right = self.parse_expr_with_precedence(precedence + 1)?;
            let span = Span::new(left.span().start, right.span().end);
            left = AstExpr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    /// Parse unary expression (!, -expr, or primary)
    fn parse_unary_expr(&mut self) -> ParseResult<AstExpr> {
        match &self.current.kind {
            TokenKind::Not => {
                let start = self.current.span.start;
                self.advance();
                let expr = self.parse_unary_expr()?;
                let span = Span::new(start, expr.span().end);
                Ok(AstExpr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span,
                })
            }
            _ => self.parse_primary_expr(),
        }
    }

    /// Parse primary expression (literals, identifiers, parenthesized expressions)
    fn parse_primary_expr(&mut self) -> ParseResult<AstExpr> {
        let tok = self.current.clone();
        match &tok.kind {
            // Literals
            TokenKind::Int(n) => {
                let value = *n;
                self.advance();
                Ok(AstExpr::Int {
                    value,
                    span: tok.span,
                })
            }
            TokenKind::Float(f) => {
                let value = *f;
                self.advance();
                Ok(AstExpr::Float {
                    value,
                    span: tok.span,
                })
            }
            TokenKind::String(s) => {
                let value = s.clone();
                self.advance();
                Ok(AstExpr::String {
                    value,
                    span: tok.span,
                })
            }
            TokenKind::Bool(b) => {
                let value = *b;
                self.advance();
                Ok(AstExpr::Bool {
                    value,
                    span: tok.span,
                })
            }
            TokenKind::None => {
                self.advance();
                Ok(AstExpr::None { span: tok.span })
            }

            // Request variables ($req.field or $request.field)
            TokenKind::Req | TokenKind::Request => {
                self.parse_request_var_expr()
            }

            // Identifiers (could be variable or function call)
            TokenKind::Ident(_) => {
                let name = if let TokenKind::Ident(s) = &tok.kind {
                    s.clone()
                } else {
                    unreachable!()
                };
                self.advance();

                // Check if this is a function call
                if self.check(&TokenKind::LParen) {
                    self.parse_function_call(name, tok.span)
                } else {
                    Ok(AstExpr::Ident {
                        name,
                        span: tok.span,
                    })
                }
            }

            // Parenthesized expression
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                // Just return the inner expression
                Ok(expr)
            }

            _ => Err(ParseError::at_span(
                format!("Expected expression, found {}", tok.kind),
                tok.span,
            )),
        }
    }

    /// Parse postfix operators (field access, indexing)
    fn parse_postfix_expr(&mut self, mut expr: AstExpr) -> ParseResult<AstExpr> {
        loop {
            match &self.current.kind {
                // Field access: expr.field
                TokenKind::Dot => {
                    self.advance();
                    let field_tok = self.expect(TokenKind::Ident("".to_string()))?;
                    let field = if let TokenKind::Ident(s) = field_tok.kind {
                        s
                    } else {
                        return Err(ParseError::at_span(
                            "Expected field name after '.'",
                            field_tok.span,
                        ));
                    };
                    let span = Span::new(expr.span().start, field_tok.span.end);
                    expr = AstExpr::FieldAccess {
                        object: Box::new(expr),
                        field,
                        span,
                    };
                }

                // Index access: expr[index]
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    let close = self.expect(TokenKind::RBracket)?;
                    let span = Span::new(expr.span().start, close.span.end);
                    expr = AstExpr::IndexAccess {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }

                _ => break,
            }
        }
        Ok(expr)
    }

    /// Parse request variable expression ($req.field or $request.field)
    fn parse_request_var_expr(&mut self) -> ParseResult<AstExpr> {
        let start = self.current.span.start;
        let is_request = matches!(self.current.kind, TokenKind::Request);
        self.advance();

        self.expect(TokenKind::Dot)?;
        let field_tok = self.expect(TokenKind::Ident("".to_string()))?;
        let field = if let TokenKind::Ident(s) = field_tok.kind {
            s
        } else {
            return Err(ParseError::at_span(
                "Expected field name after $req. or $request.",
                field_tok.span,
            ));
        };

        let span = Span::new(start, field_tok.span.end);
        let var = RequestVar {
            is_request,
            field,
            span,
        };

        Ok(AstExpr::RequestVar {
            var,
            span,
        })
    }

    /// Parse function call (name already consumed)
    fn parse_function_call(&mut self, name: String, name_span: Span) -> ParseResult<AstExpr> {
        self.expect(TokenKind::LParen)?;

        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance(); // consume comma
            }
        }

        let close = self.expect(TokenKind::RParen)?;
        let span = Span::new(name_span.start, close.span.end);

        Ok(AstExpr::Call {
            function: name,
            args,
            span,
        })
    }

    /// Get the current binary operator and its precedence
    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        let (op, prec) = match &self.current.kind {
            // Logical OR (lowest precedence)
            TokenKind::Or => (BinaryOp::Or, 1),
            // Logical AND
            TokenKind::And => (BinaryOp::And, 2),
            // Equality
            TokenKind::EqEq => (BinaryOp::Eq, 3),
            TokenKind::NotEq => (BinaryOp::NotEq, 3),
            // Comparison
            TokenKind::Lt => (BinaryOp::Lt, 4),
            TokenKind::Gt => (BinaryOp::Gt, 4),
            TokenKind::LtEq => (BinaryOp::LtEq, 4),
            TokenKind::GtEq => (BinaryOp::GtEq, 4),
            // Addition/Subtraction
            TokenKind::Plus => (BinaryOp::Add, 5),
            TokenKind::Minus => (BinaryOp::Sub, 5),
            // Multiplication/Division/Modulo (highest precedence)
            TokenKind::Star => (BinaryOp::Mul, 6),
            TokenKind::Slash => (BinaryOp::Div, 6),
            TokenKind::Percent => (BinaryOp::Mod, 6),
            _ => return None,
        };
        Some((op, prec))
    }

    /// Advance to the next token
    fn advance(&mut self) {
        self.current = std::mem::replace(&mut self.peek, self.lexer.next_token());
    }

    /// Check if current token matches a kind
    fn check(&self, kind: &TokenKind) -> bool {
        // For Ident, we do a special check
        match (kind, &self.current.kind) {
            (TokenKind::Ident(_), TokenKind::Ident(_)) => true,
            _ => &self.current.kind == kind,
        }
    }

    /// Expect a specific token kind and advance
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        if self.check(&kind) {
            let tok = self.current.clone();
            self.advance();
            Ok(tok)
        } else {
            Err(ParseError::expected(kind, self.current.clone()))
        }
    }
}
