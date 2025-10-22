//! Abstract Syntax Tree for HogTrace language

use super::lexer::Span;
use std::fmt;

/// Top-level AST node representing a complete HogTrace program
#[derive(Debug, Clone)]
pub struct AstProgram {
    pub probes: Vec<AstProbe>,
    pub span: Span,
}

/// A probe definition
#[derive(Debug, Clone)]
pub struct AstProbe {
    pub spec: ProbeSpec,
    pub predicate: Option<AstExpr>,
    pub body: Vec<AstStatement>,
    pub span: Span,
}

/// Probe specification
#[derive(Debug, Clone)]
pub struct ProbeSpec {
    pub provider: Provider,
    pub module_function: ModuleFunction,
    pub probe_point: ProbePoint,
    pub span: Span,
}

/// Provider type (fn or py)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Fn,
    Py,
}

/// Module and function path
#[derive(Debug, Clone)]
pub struct ModuleFunction {
    pub parts: Vec<ModulePart>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ModulePart {
    Ident(String),
    Wildcard,
}

/// Probe point (entry, exit, etc.)
#[derive(Debug, Clone)]
pub enum ProbePoint {
    Entry,
    Exit,
    EntryOffset(i64),
    ExitOffset(i64),
}

/// Statements in action blocks
#[derive(Debug, Clone)]
pub enum AstStatement {
    Assignment {
        var: RequestVar,
        value: AstExpr,
        span: Span,
    },
    Sample {
        spec: SampleSpec,
        span: Span,
    },
    Capture {
        is_send: bool,
        args: CaptureArgs,
        span: Span,
    },
}

/// Request-scoped variable
#[derive(Debug, Clone)]
pub struct RequestVar {
    pub is_request: bool, // true for $request, false for $req
    pub field: String,
    pub span: Span,
}

/// Sample specification
#[derive(Debug, Clone)]
pub enum SampleSpec {
    Percentage(i64),
    Ratio { numerator: i64, denominator: i64 },
}

/// Capture arguments
#[derive(Debug, Clone)]
pub enum CaptureArgs {
    Positional(Vec<AstExpr>),
    Named(Vec<NamedArg>),
}

#[derive(Debug, Clone)]
pub struct NamedArg {
    pub name: String,
    pub value: AstExpr,
    pub span: Span,
}

/// Expressions
#[derive(Debug, Clone)]
pub enum AstExpr {
    // Literals
    Int {
        value: i64,
        span: Span,
    },
    Float {
        value: f64,
        span: Span,
    },
    String {
        value: String,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    None {
        span: Span,
    },

    // Identifiers
    Ident {
        name: String,
        span: Span,
    },
    RequestVar {
        var: RequestVar,
        span: Span,
    },

    // Binary operations
    Binary {
        op: BinaryOp,
        left: Box<AstExpr>,
        right: Box<AstExpr>,
        span: Span,
    },

    // Unary operations
    Unary {
        op: UnaryOp,
        expr: Box<AstExpr>,
        span: Span,
    },

    // Field access (obj.field)
    FieldAccess {
        object: Box<AstExpr>,
        field: String,
        span: Span,
    },

    // Index access (obj[index])
    IndexAccess {
        object: Box<AstExpr>,
        index: Box<AstExpr>,
        span: Span,
    },

    // Function call
    Call {
        function: String,
        args: Vec<AstExpr>,
        span: Span,
    },
}

impl AstExpr {
    pub fn span(&self) -> Span {
        match self {
            AstExpr::Int { span, .. }
            | AstExpr::Float { span, .. }
            | AstExpr::String { span, .. }
            | AstExpr::Bool { span, .. }
            | AstExpr::None { span }
            | AstExpr::Ident { span, .. }
            | AstExpr::RequestVar { span, .. }
            | AstExpr::Binary { span, .. }
            | AstExpr::Unary { span, .. }
            | AstExpr::FieldAccess { span, .. }
            | AstExpr::IndexAccess { span, .. }
            | AstExpr::Call { span, .. } => *span,
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Lt,
    Gt,
    LtEq,
    GtEq,
    Eq,
    NotEq,

    // Logical
    And,
    Or,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
}

// Display implementations for debugging and testing

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Fn => write!(f, "fn"),
            Provider::Py => write!(f, "py"),
        }
    }
}

impl fmt::Display for ProbePoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbePoint::Entry => write!(f, "entry"),
            ProbePoint::Exit => write!(f, "exit"),
            ProbePoint::EntryOffset(n) => write!(f, "entry+{}", n),
            ProbePoint::ExitOffset(n) => write!(f, "exit+{}", n),
        }
    }
}

impl fmt::Display for ModuleFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            match part {
                ModulePart::Ident(s) => write!(f, "{}", s)?,
                ModulePart::Wildcard => write!(f, "*")?,
            }
        }
        Ok(())
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::LtEq => write!(f, "<="),
            BinaryOp::GtEq => write!(f, ">="),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::NotEq => write!(f, "!="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::lexer::Position;

    fn dummy_span() -> Span {
        Span::new(Position::start(), Position::start())
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::Fn.to_string(), "fn");
        assert_eq!(Provider::Py.to_string(), "py");
    }

    #[test]
    fn test_probe_point_display() {
        assert_eq!(ProbePoint::Entry.to_string(), "entry");
        assert_eq!(ProbePoint::Exit.to_string(), "exit");
        assert_eq!(ProbePoint::EntryOffset(10).to_string(), "entry+10");
        assert_eq!(ProbePoint::ExitOffset(5).to_string(), "exit+5");
    }

    #[test]
    fn test_module_function_display() {
        let mf = ModuleFunction {
            parts: vec![
                ModulePart::Ident("myapp".to_string()),
                ModulePart::Ident("users".to_string()),
                ModulePart::Ident("create".to_string()),
            ],
            span: dummy_span(),
        };
        assert_eq!(mf.to_string(), "myapp.users.create");
    }

    #[test]
    fn test_module_function_with_wildcard() {
        let mf = ModuleFunction {
            parts: vec![
                ModulePart::Ident("myapp".to_string()),
                ModulePart::Wildcard,
            ],
            span: dummy_span(),
        };
        assert_eq!(mf.to_string(), "myapp.*");
    }

    #[test]
    fn test_binary_op_display() {
        assert_eq!(BinaryOp::Add.to_string(), "+");
        assert_eq!(BinaryOp::Eq.to_string(), "==");
        assert_eq!(BinaryOp::And.to_string(), "&&");
    }

    #[test]
    fn test_unary_op_display() {
        assert_eq!(UnaryOp::Not.to_string(), "!");
    }

    #[test]
    fn test_ast_expr_span() {
        let span = dummy_span();
        let expr = AstExpr::Int { value: 42, span };
        assert_eq!(expr.span(), span);
    }

    #[test]
    fn test_request_var_construction() {
        let var = RequestVar {
            is_request: false,
            field: "user_id".to_string(),
            span: dummy_span(),
        };
        assert!(!var.is_request);
        assert_eq!(var.field, "user_id");
    }

    #[test]
    fn test_sample_spec_variants() {
        let percentage = SampleSpec::Percentage(50);
        match percentage {
            SampleSpec::Percentage(n) => assert_eq!(n, 50),
            _ => panic!("Expected percentage"),
        }

        let ratio = SampleSpec::Ratio {
            numerator: 1,
            denominator: 10,
        };
        match ratio {
            SampleSpec::Ratio { numerator, denominator } => {
                assert_eq!(numerator, 1);
                assert_eq!(denominator, 10);
            }
            _ => panic!("Expected ratio"),
        }
    }

    #[test]
    fn test_capture_args_variants() {
        let positional = CaptureArgs::Positional(vec![]);
        match positional {
            CaptureArgs::Positional(_) => (),
            _ => panic!("Expected positional"),
        }

        let named = CaptureArgs::Named(vec![]);
        match named {
            CaptureArgs::Named(_) => (),
            _ => panic!("Expected named"),
        }
    }

    #[test]
    fn test_ast_statement_variants() {
        let assign = AstStatement::Assignment {
            var: RequestVar {
                is_request: false,
                field: "x".to_string(),
                span: dummy_span(),
            },
            value: AstExpr::Int {
                value: 42,
                span: dummy_span(),
            },
            span: dummy_span(),
        };
        match assign {
            AstStatement::Assignment { .. } => (),
            _ => panic!("Expected assignment"),
        }

        let sample = AstStatement::Sample {
            spec: SampleSpec::Percentage(10),
            span: dummy_span(),
        };
        match sample {
            AstStatement::Sample { .. } => (),
            _ => panic!("Expected sample"),
        }

        let capture = AstStatement::Capture {
            is_send: false,
            args: CaptureArgs::Positional(vec![]),
            span: dummy_span(),
        };
        match capture {
            AstStatement::Capture { .. } => (),
            _ => panic!("Expected capture"),
        }
    }

    #[test]
    fn test_binary_expr_construction() {
        let left = Box::new(AstExpr::Int {
            value: 1,
            span: dummy_span(),
        });
        let right = Box::new(AstExpr::Int {
            value: 2,
            span: dummy_span(),
        });

        let binary = AstExpr::Binary {
            op: BinaryOp::Add,
            left,
            right,
            span: dummy_span(),
        };

        match binary {
            AstExpr::Binary { op, .. } => assert_eq!(op, BinaryOp::Add),
            _ => panic!("Expected binary"),
        }
    }

    #[test]
    fn test_unary_expr_construction() {
        let expr = Box::new(AstExpr::Bool {
            value: true,
            span: dummy_span(),
        });

        let unary = AstExpr::Unary {
            op: UnaryOp::Not,
            expr,
            span: dummy_span(),
        };

        match unary {
            AstExpr::Unary { op, .. } => assert_eq!(op, UnaryOp::Not),
            _ => panic!("Expected unary"),
        }
    }

    #[test]
    fn test_field_access_construction() {
        let obj = Box::new(AstExpr::Ident {
            name: "user".to_string(),
            span: dummy_span(),
        });

        let access = AstExpr::FieldAccess {
            object: obj,
            field: "name".to_string(),
            span: dummy_span(),
        };

        match access {
            AstExpr::FieldAccess { field, .. } => assert_eq!(field, "name"),
            _ => panic!("Expected field access"),
        }
    }

    #[test]
    fn test_index_access_construction() {
        let obj = Box::new(AstExpr::Ident {
            name: "arr".to_string(),
            span: dummy_span(),
        });
        let index = Box::new(AstExpr::Int {
            value: 0,
            span: dummy_span(),
        });

        let access = AstExpr::IndexAccess {
            object: obj,
            index,
            span: dummy_span(),
        };

        match access {
            AstExpr::IndexAccess { .. } => (),
            _ => panic!("Expected index access"),
        }
    }

    #[test]
    fn test_function_call_construction() {
        let call = AstExpr::Call {
            function: "test".to_string(),
            args: vec![],
            span: dummy_span(),
        };

        match call {
            AstExpr::Call { function, args, .. } => {
                assert_eq!(function, "test");
                assert_eq!(args.len(), 0);
            }
            _ => panic!("Expected call"),
        }
    }
}
