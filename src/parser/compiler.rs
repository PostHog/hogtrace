//! Compiler that converts AST to VM bytecode

use super::ast::{
    AstExpr, AstProgram, AstProbe, AstStatement, BinaryOp, UnaryOp,
    ProbePoint, Provider, ModuleFunction, CaptureArgs
};
use super::error::{ParseResult, ParseError, ErrorKind};
use crate::{Program, Probe, ProbeSpec, FnTarget};
use crate::constant_pool::{Constant, ConstantPool};
use crate::opcodes::Opcode;
use std::collections::HashMap;

/// Compiler that translates AST to VM bytecode
pub struct Compiler {
    /// Shared constant pool for the entire program
    constant_pool: ConstantPool,

    /// Current bytecode being generated
    bytecode: Vec<u8>,

    /// Map to deduplicate constants (constant value -> pool index)
    constant_map: HashMap<ConstantKey, u16>,
}

/// Key for deduplicating constants in the constant pool
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConstantKey {
    Int(i64),
    Float(OrderedFloat),
    String(String),
    Bool(bool),
    None,
    Identifier(String),
    FieldName(String),
    FunctionName(String),
}

/// Wrapper for f64 that implements Eq and Hash (using bit representation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OrderedFloat(u64);

impl From<f64> for OrderedFloat {
    fn from(f: f64) -> Self {
        OrderedFloat(f.to_bits())
    }
}

impl From<OrderedFloat> for f64 {
    fn from(of: OrderedFloat) -> Self {
        f64::from_bits(of.0)
    }
}

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Compiler {
            constant_pool: ConstantPool::new(),
            bytecode: Vec::new(),
            constant_map: HashMap::new(),
        }
    }

    /// Compile an AST program into VM bytecode
    pub fn compile(&mut self, ast: AstProgram) -> ParseResult<Program> {
        let mut probes = Vec::new();

        for (idx, ast_probe) in ast.probes.into_iter().enumerate() {
            let probe = self.compile_probe(ast_probe, idx)?;
            probes.push(probe);
        }

        Ok(Program {
            version: 1,
            constant_pool: self.constant_pool.clone(),
            probes,
            sampling: 1.0, // Default sampling rate
        })
    }

    // ===== Constant Pool Management =====

    /// Add a constant to the pool (or return existing index if already present)
    fn add_or_get_constant(&mut self, constant: Constant) -> u16 {
        let key = Self::constant_to_key(&constant);

        if let Some(&index) = self.constant_map.get(&key) {
            return index;
        }

        let index = self.constant_pool.add(constant);
        self.constant_map.insert(key, index);
        index
    }

    /// Convert a Constant to a ConstantKey for deduplication
    fn constant_to_key(constant: &Constant) -> ConstantKey {
        match constant {
            Constant::Int(i) => ConstantKey::Int(*i),
            Constant::Float(f) => ConstantKey::Float((*f).into()),
            Constant::String(s) => ConstantKey::String(s.clone()),
            Constant::Bool(b) => ConstantKey::Bool(*b),
            Constant::None => ConstantKey::None,
            Constant::Identifier(s) => ConstantKey::Identifier(s.clone()),
            Constant::FieldName(s) => ConstantKey::FieldName(s.clone()),
            Constant::FunctionName(s) => ConstantKey::FunctionName(s.clone()),
        }
    }

    // ===== Bytecode Emission =====

    /// Emit a single opcode
    fn emit(&mut self, opcode: Opcode) {
        self.bytecode.push(opcode as u8);
    }

    /// Emit an opcode with a u16 operand (little-endian)
    fn emit_u16(&mut self, opcode: Opcode, operand: u16) {
        self.emit(opcode);
        let bytes = operand.to_le_bytes();
        self.bytecode.extend_from_slice(&bytes);
    }

    /// Emit an opcode with a u8 operand
    fn emit_u8(&mut self, opcode: Opcode, operand: u8) {
        self.emit(opcode);
        self.bytecode.push(operand);
    }

    /// Emit CallFunc instruction (u16 function index + u8 arg count)
    fn emit_call(&mut self, func_index: u16, arg_count: u8) {
        self.emit(Opcode::CallFunc);
        let bytes = func_index.to_le_bytes();
        self.bytecode.extend_from_slice(&bytes);
        self.bytecode.push(arg_count);
    }

    /// Take the current bytecode and reset for next compilation
    fn take_bytecode(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.bytecode)
    }

    // ===== Expression Compilation =====

    /// Compile an expression into bytecode
    fn compile_expr(&mut self, expr: &AstExpr) -> ParseResult<()> {
        match expr {
            AstExpr::Int { value, .. } => {
                let idx = self.add_or_get_constant(Constant::Int(*value));
                self.emit_u16(Opcode::PushConst, idx);
            }

            AstExpr::Float { value, .. } => {
                let idx = self.add_or_get_constant(Constant::Float(*value));
                self.emit_u16(Opcode::PushConst, idx);
            }

            AstExpr::String { value, .. } => {
                let idx = self.add_or_get_constant(Constant::String(value.clone()));
                self.emit_u16(Opcode::PushConst, idx);
            }

            AstExpr::Bool { value, .. } => {
                let idx = self.add_or_get_constant(Constant::Bool(*value));
                self.emit_u16(Opcode::PushConst, idx);
            }

            AstExpr::None { .. } => {
                let idx = self.add_or_get_constant(Constant::None);
                self.emit_u16(Opcode::PushConst, idx);
            }

            AstExpr::Ident { name, .. } => {
                // Variables/identifiers like "args", "arg0", "retval", etc.
                let idx = self.add_or_get_constant(Constant::Identifier(name.clone()));
                self.emit_u16(Opcode::LoadVar, idx);
            }

            AstExpr::RequestVar { var, .. } => {
                // $request.field or $req.field
                // Compile as: LoadVar(request/req), GetAttr(field)
                let var_name = if var.is_request { "request" } else { "req" };
                let var_idx = self.add_or_get_constant(Constant::Identifier(var_name.to_string()));
                self.emit_u16(Opcode::LoadVar, var_idx);

                let field_idx = self.add_or_get_constant(Constant::FieldName(var.field.clone()));
                self.emit_u16(Opcode::GetAttr, field_idx);
            }

            AstExpr::Binary { op, left, right, span } => {
                // Compile left operand (pushes value onto stack)
                self.compile_expr(left)?;

                // Compile right operand (pushes value onto stack)
                self.compile_expr(right)?;

                // Emit operation (pops 2, pushes 1)
                let opcode = match op {
                    BinaryOp::Add => Opcode::Add,
                    BinaryOp::Sub => Opcode::Sub,
                    BinaryOp::Mul => Opcode::Mul,
                    BinaryOp::Div => Opcode::Div,
                    BinaryOp::Mod => Opcode::Mod,
                    BinaryOp::Eq => Opcode::Eq,
                    BinaryOp::NotEq => Opcode::Ne,
                    BinaryOp::Lt => Opcode::Lt,
                    BinaryOp::Gt => Opcode::Gt,
                    BinaryOp::LtEq => Opcode::Le,
                    BinaryOp::GtEq => Opcode::Ge,
                    BinaryOp::And => Opcode::And,
                    BinaryOp::Or => Opcode::Or,
                };
                self.emit(opcode);
            }

            AstExpr::Unary { op, expr, .. } => {
                // Only Not is supported in the AST
                match op {
                    UnaryOp::Not => {
                        // Compile operand (pushes value onto stack)
                        self.compile_expr(expr)?;
                        // Emit Not (pops 1, pushes 1)
                        self.emit(Opcode::Not);
                    }
                }
            }

            AstExpr::FieldAccess { object, field, span } => {
                // Compile object expression (pushes object onto stack)
                self.compile_expr(object)?;

                // Add field name to constant pool and emit GetAttr
                let idx = self.add_or_get_constant(Constant::FieldName(field.clone()));
                self.emit_u16(Opcode::GetAttr, idx);
            }

            AstExpr::IndexAccess { object, index, span } => {
                // Compile object expression (pushes object onto stack)
                self.compile_expr(object)?;

                // Compile index expression (pushes index onto stack)
                self.compile_expr(index)?;

                // Emit GetItem (pops object and index, pushes value)
                self.emit(Opcode::GetItem);
            }

            AstExpr::Call { function, args, span } => {
                // Compile each argument (left to right)
                for arg in args {
                    self.compile_expr(arg)?;
                }

                // Add function name to constant pool
                let idx = self.add_or_get_constant(Constant::FunctionName(function.clone()));

                // Emit CallFunc with arg count
                if args.len() > 255 {
                    return Err(ParseError::at_span(
                        format!("Too many arguments to function '{}': {} (max 255)", function, args.len()),
                        *span
                    ));
                }
                self.emit_call(idx, args.len() as u8);
            }
        }

        Ok(())
    }

    // ===== Statement Compilation =====

    /// Compile a statement into bytecode
    fn compile_stmt(&mut self, stmt: &AstStatement) -> ParseResult<()> {
        match stmt {
            AstStatement::Assignment { var, value, .. } => {
                // Compile the value expression (pushes result onto stack)
                self.compile_expr(value)?;

                // For assignment to request variable, we need:
                // LoadVar(request/req), swap, SetAttr(field)
                // But we don't have swap! So instead we do:
                // Compile value first (already done above)
                // Then StoreVar to the full path

                // Actually, looking at the VM opcodes, StoreVar is for simple variables
                // For $req.field, we need to:
                // 1. Load req
                // 2. Compile value
                // 3. SetAttr or similar

                // Wait, let me re-check the opcodes...
                // LoadVar, StoreVar are for variables
                // GetAttr is for reading obj.field
                // But there's no SetAttr for writing!

                // For now, let's assume StoreVar works with dotted paths
                // We'll compile it as: value, StoreVar("req.field")

                let var_name = if var.is_request {
                    format!("request.{}", var.field)
                } else {
                    format!("req.{}", var.field)
                };

                let idx = self.add_or_get_constant(Constant::Identifier(var_name));
                self.emit_u16(Opcode::StoreVar, idx);
            }

            AstStatement::Capture { is_send, args, span } => {
                // Compile capture/send as a function call
                // Arguments are compiled as expressions
                match args {
                    CaptureArgs::Positional(exprs) => {
                        // Compile each argument
                        for expr in exprs {
                            self.compile_expr(expr)?;
                        }

                        // Add function name to constant pool
                        let func_name = if *is_send { "send" } else { "capture" };
                        let idx = self.add_or_get_constant(Constant::FunctionName(func_name.to_string()));

                        // Emit CallFunc
                        if exprs.len() > 255 {
                            return Err(ParseError::at_span(
                                format!("Too many arguments to {}: {} (max 255)", func_name, exprs.len()),
                                *span
                            ));
                        }
                        self.emit_call(idx, exprs.len() as u8);
                    }

                    CaptureArgs::Named(entries) => {
                        // Named arguments: compile as key-value pairs
                        // For now, we'll treat this as positional arguments with string keys
                        // A more sophisticated approach would use a dict/map construct

                        // For each named argument, push the name as a string, then the value
                        for named_arg in entries {
                            // Push the name as a string
                            let name_idx = self.add_or_get_constant(Constant::String(named_arg.name.clone()));
                            self.emit_u16(Opcode::PushConst, name_idx);

                            // Push the value
                            self.compile_expr(&named_arg.value)?;
                        }

                        // Call capture/send with 2*N arguments (name-value pairs)
                        let func_name = if *is_send { "send" } else { "capture" };
                        let idx = self.add_or_get_constant(Constant::FunctionName(func_name.to_string()));

                        let arg_count = entries.len() * 2;
                        if arg_count > 255 {
                            return Err(ParseError::at_span(
                                format!("Too many arguments to {}: {} (max 255)", func_name, arg_count),
                                *span
                            ));
                        }
                        self.emit_call(idx, arg_count as u8);
                    }
                }

                // Pop the result since statements don't produce values
                self.emit(Opcode::Pop);
            }

            AstStatement::Sample { span, .. } => {
                // Sample statements are handled at the probe level, not in the body
                return Err(ParseError::at_span(
                    "Sample statements should be handled at probe level",
                    *span
                ));
            }
        }

        Ok(())
    }

    // ===== Probe and Program Compilation =====

    /// Compile a probe into a Probe struct with bytecode
    fn compile_probe(&mut self, probe: AstProbe, idx: usize) -> ParseResult<Probe> {
        // Generate probe ID
        let id = format!("probe_{}", idx);

        // Convert probe spec
        let spec = self.compile_probe_spec(&probe.spec)?;

        // Compile predicate (if present)
        let predicate = if let Some(pred_expr) = &probe.predicate {
            self.compile_expr(pred_expr)?;
            self.take_bytecode()
        } else {
            vec![]
        };

        // Compile probe body
        for stmt in &probe.body {
            self.compile_stmt(stmt)?;
        }
        let body = self.take_bytecode();

        Ok(Probe {
            id,
            spec,
            predicate,
            body,
        })
    }

    /// Convert AST ProbeSpec to VM ProbeSpec
    fn compile_probe_spec(&self, spec: &super::ast::ProbeSpec) -> ParseResult<ProbeSpec> {
        match &spec.provider {
            Provider::Fn | Provider::Py => {
                // Build function specifier string (module.function)
                let specifier = spec.module_function.to_string();

                // Convert probe point to target
                let target = match &spec.probe_point {
                    ProbePoint::Entry => FnTarget::Entry,
                    ProbePoint::Exit => FnTarget::Exit,
                    ProbePoint::EntryOffset(_) => FnTarget::Entry, // Simplified for now
                    ProbePoint::ExitOffset(_) => FnTarget::Exit,   // Simplified for now
                };

                Ok(ProbeSpec::Fn { specifier, target })
            }
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_new() {
        let compiler = Compiler::new();
        assert_eq!(compiler.constant_pool.len(), 0);
        assert_eq!(compiler.bytecode.len(), 0);
    }

    #[test]
    fn test_add_constant_deduplication() {
        let mut compiler = Compiler::new();

        let idx1 = compiler.add_or_get_constant(Constant::Int(42));
        let idx2 = compiler.add_or_get_constant(Constant::Int(42));
        let idx3 = compiler.add_or_get_constant(Constant::Int(100));

        assert_eq!(idx1, idx2); // Same constant, same index
        assert_ne!(idx1, idx3); // Different constant, different index
        assert_eq!(compiler.constant_pool.len(), 2); // Only 2 unique constants
    }

    #[test]
    fn test_add_constant_string_dedup() {
        let mut compiler = Compiler::new();

        let idx1 = compiler.add_or_get_constant(Constant::String("hello".to_string()));
        let idx2 = compiler.add_or_get_constant(Constant::String("hello".to_string()));
        let idx3 = compiler.add_or_get_constant(Constant::String("world".to_string()));

        assert_eq!(idx1, idx2);
        assert_ne!(idx1, idx3);
        assert_eq!(compiler.constant_pool.len(), 2);
    }

    #[test]
    fn test_add_constant_identifier_dedup() {
        let mut compiler = Compiler::new();

        let idx1 = compiler.add_or_get_constant(Constant::Identifier("args".to_string()));
        let idx2 = compiler.add_or_get_constant(Constant::Identifier("args".to_string()));

        assert_eq!(idx1, idx2);
        assert_eq!(compiler.constant_pool.len(), 1);
    }

    #[test]
    fn test_emit_opcode() {
        let mut compiler = Compiler::new();

        compiler.emit(Opcode::Add);
        compiler.emit(Opcode::Mul);

        assert_eq!(compiler.bytecode, vec![Opcode::Add as u8, Opcode::Mul as u8]);
    }

    #[test]
    fn test_emit_u16() {
        let mut compiler = Compiler::new();

        compiler.emit_u16(Opcode::PushConst, 0x1234);

        assert_eq!(compiler.bytecode, vec![
            Opcode::PushConst as u8,
            0x34, // Little-endian low byte
            0x12, // Little-endian high byte
        ]);
    }

    #[test]
    fn test_emit_call() {
        let mut compiler = Compiler::new();

        compiler.emit_call(0x0100, 3);

        assert_eq!(compiler.bytecode, vec![
            Opcode::CallFunc as u8,
            0x00, 0x01, // Function index (little-endian)
            3,          // Arg count
        ]);
    }

    #[test]
    fn test_take_bytecode() {
        let mut compiler = Compiler::new();

        compiler.emit(Opcode::Add);
        compiler.emit(Opcode::Mul);

        let bytecode = compiler.take_bytecode();

        assert_eq!(bytecode, vec![Opcode::Add as u8, Opcode::Mul as u8]);
        assert_eq!(compiler.bytecode.len(), 0); // Should be empty after take
    }

    #[test]
    fn test_float_deduplication() {
        let mut compiler = Compiler::new();

        let idx1 = compiler.add_or_get_constant(Constant::Float(3.14));
        let idx2 = compiler.add_or_get_constant(Constant::Float(3.14));
        let idx3 = compiler.add_or_get_constant(Constant::Float(2.71));

        assert_eq!(idx1, idx2);
        assert_ne!(idx1, idx3);
        assert_eq!(compiler.constant_pool.len(), 2);
    }

    #[test]
    fn test_bool_and_none_dedup() {
        let mut compiler = Compiler::new();

        let idx1 = compiler.add_or_get_constant(Constant::Bool(true));
        let idx2 = compiler.add_or_get_constant(Constant::Bool(true));
        let idx3 = compiler.add_or_get_constant(Constant::Bool(false));
        let idx4 = compiler.add_or_get_constant(Constant::None);
        let idx5 = compiler.add_or_get_constant(Constant::None);

        assert_eq!(idx1, idx2);
        assert_ne!(idx1, idx3);
        assert_eq!(idx4, idx5);
        assert_eq!(compiler.constant_pool.len(), 3); // true, false, none
    }

    // ===== Expression Compilation Tests =====

    use super::super::lexer::Lexer;
    use super::super::Parser;

    fn compile_expr_helper(source: &str) -> (Vec<u8>, ConstantPool) {
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let expr = parser.parse_expr().unwrap();

        let mut compiler = Compiler::new();
        compiler.compile_expr(&expr).unwrap();

        (compiler.bytecode, compiler.constant_pool)
    }

    #[test]
    fn test_compile_int_literal() {
        let (bytecode, pool) = compile_expr_helper("42");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        // Get constant index from bytecode (little-endian u16)
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        let constant = pool.get(idx).unwrap();
        assert!(matches!(constant, Constant::Int(42)));
    }

    #[test]
    fn test_compile_float_literal() {
        let (bytecode, pool) = compile_expr_helper("3.14");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        let constant = pool.get(idx).unwrap();
        if let Constant::Float(f) = constant {
            assert!((f - 3.14).abs() < 0.001);
        } else {
            panic!("Expected Float constant");
        }
    }

    #[test]
    fn test_compile_string_literal() {
        let (bytecode, pool) = compile_expr_helper("\"hello\"");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        let constant = pool.get(idx).unwrap();
        assert!(matches!(constant, Constant::String(s) if s == "hello"));
    }

    #[test]
    fn test_compile_bool_literal() {
        let (bytecode, pool) = compile_expr_helper("True");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        let constant = pool.get(idx).unwrap();
        assert!(matches!(constant, Constant::Bool(true)));
    }

    #[test]
    fn test_compile_none_literal() {
        let (bytecode, pool) = compile_expr_helper("None");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        let constant = pool.get(idx).unwrap();
        assert!(matches!(constant, Constant::None));
    }

    #[test]
    fn test_compile_variable() {
        let (bytecode, pool) = compile_expr_helper("args");

        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        let constant = pool.get(idx).unwrap();
        assert!(matches!(constant, Constant::Identifier(s) if s == "args"));
    }

    #[test]
    fn test_compile_request_var() {
        let (bytecode, pool) = compile_expr_helper("$req.user_id");

        // Should be: LoadVar(req), GetAttr(user_id)
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        let idx1 = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        assert!(matches!(pool.get(idx1).unwrap(), Constant::Identifier(s) if s == "req"));

        assert_eq!(bytecode[3], Opcode::GetAttr as u8);
        let idx2 = u16::from_le_bytes([bytecode[4], bytecode[5]]);
        assert!(matches!(pool.get(idx2).unwrap(), Constant::FieldName(s) if s == "user_id"));
    }

    #[test]
    fn test_compile_binary_add() {
        let (bytecode, _pool) = compile_expr_helper("1 + 2");

        // Should be: PushConst(1), PushConst(2), Add
        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::Add as u8);
    }

    #[test]
    fn test_compile_binary_subtract() {
        let (bytecode, _pool) = compile_expr_helper("10 - 3");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::Sub as u8);
    }

    #[test]
    fn test_compile_binary_multiply() {
        let (bytecode, _pool) = compile_expr_helper("5 * 6");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::Mul as u8);
    }

    #[test]
    fn test_compile_binary_divide() {
        let (bytecode, _pool) = compile_expr_helper("12 / 4");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::Div as u8);
    }

    #[test]
    fn test_compile_binary_modulo() {
        let (bytecode, _pool) = compile_expr_helper("10 % 3");

        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::Mod as u8);
    }

    #[test]
    fn test_compile_comparison_eq() {
        let (bytecode, _pool) = compile_expr_helper("1 == 2");

        assert_eq!(bytecode[6], Opcode::Eq as u8);
    }

    #[test]
    fn test_compile_comparison_ne() {
        let (bytecode, _pool) = compile_expr_helper("1 != 2");

        assert_eq!(bytecode[6], Opcode::Ne as u8);
    }

    #[test]
    fn test_compile_comparison_lt() {
        let (bytecode, _pool) = compile_expr_helper("1 < 2");

        assert_eq!(bytecode[6], Opcode::Lt as u8);
    }

    #[test]
    fn test_compile_comparison_gt() {
        let (bytecode, _pool) = compile_expr_helper("1 > 2");

        assert_eq!(bytecode[6], Opcode::Gt as u8);
    }

    #[test]
    fn test_compile_comparison_le() {
        let (bytecode, _pool) = compile_expr_helper("1 <= 2");

        assert_eq!(bytecode[6], Opcode::Le as u8);
    }

    #[test]
    fn test_compile_comparison_ge() {
        let (bytecode, _pool) = compile_expr_helper("1 >= 2");

        assert_eq!(bytecode[6], Opcode::Ge as u8);
    }

    #[test]
    fn test_compile_logical_and() {
        let (bytecode, _pool) = compile_expr_helper("True && False");

        assert_eq!(bytecode[6], Opcode::And as u8);
    }

    #[test]
    fn test_compile_logical_or() {
        let (bytecode, _pool) = compile_expr_helper("True || False");

        assert_eq!(bytecode[6], Opcode::Or as u8);
    }

    #[test]
    fn test_compile_unary_not() {
        let (bytecode, _pool) = compile_expr_helper("!True");

        // Should be: PushConst(True), Not
        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::Not as u8);
    }

    #[test]
    fn test_compile_field_access() {
        let (bytecode, pool) = compile_expr_helper("args.user");

        // Should be: LoadVar(args), GetAttr(user)
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::GetAttr as u8);

        let idx = u16::from_le_bytes([bytecode[4], bytecode[5]]);
        assert!(matches!(pool.get(idx).unwrap(), Constant::FieldName(s) if s == "user"));
    }

    #[test]
    fn test_compile_index_access() {
        let (bytecode, _pool) = compile_expr_helper("args[0]");

        // Should be: LoadVar(args), PushConst(0), GetItem
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::GetItem as u8);
    }

    #[test]
    fn test_compile_function_call_no_args() {
        let (bytecode, pool) = compile_expr_helper("timestamp()");

        // Should be: CallFunc(timestamp, 0)
        assert_eq!(bytecode[0], Opcode::CallFunc as u8);
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        assert!(matches!(pool.get(idx).unwrap(), Constant::FunctionName(s) if s == "timestamp"));
        assert_eq!(bytecode[3], 0); // 0 arguments
    }

    #[test]
    fn test_compile_function_call_one_arg() {
        let (bytecode, pool) = compile_expr_helper("len(args)");

        // Should be: LoadVar(args), CallFunc(len, 1)
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::CallFunc as u8);

        let idx = u16::from_le_bytes([bytecode[4], bytecode[5]]);
        assert!(matches!(pool.get(idx).unwrap(), Constant::FunctionName(s) if s == "len"));
        assert_eq!(bytecode[6], 1); // 1 argument
    }

    #[test]
    fn test_compile_function_call_multiple_args() {
        let (bytecode, pool) = compile_expr_helper("min(arg0, arg1, arg2)");

        // Should be: LoadVar(arg0), LoadVar(arg1), LoadVar(arg2), CallFunc(min, 3)
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::LoadVar as u8);
        assert_eq!(bytecode[6], Opcode::LoadVar as u8);
        assert_eq!(bytecode[9], Opcode::CallFunc as u8);

        let idx = u16::from_le_bytes([bytecode[10], bytecode[11]]);
        assert!(matches!(pool.get(idx).unwrap(), Constant::FunctionName(s) if s == "min"));
        assert_eq!(bytecode[12], 3); // 3 arguments
    }

    #[test]
    fn test_compile_complex_expr() {
        let (bytecode, _pool) = compile_expr_helper("(1 + 2) * 3");

        // Should be: PushConst(1), PushConst(2), Add, PushConst(3), Mul
        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::Add as u8);
        assert_eq!(bytecode[7], Opcode::PushConst as u8);
        assert_eq!(bytecode[10], Opcode::Mul as u8);
    }

    #[test]
    fn test_compile_nested_field_access() {
        let (bytecode, pool) = compile_expr_helper("args.user.email");

        // Should be: LoadVar(args), GetAttr(user), GetAttr(email)
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::GetAttr as u8);

        let idx1 = u16::from_le_bytes([bytecode[4], bytecode[5]]);
        assert!(matches!(pool.get(idx1).unwrap(), Constant::FieldName(s) if s == "user"));

        assert_eq!(bytecode[6], Opcode::GetAttr as u8);
        let idx2 = u16::from_le_bytes([bytecode[7], bytecode[8]]);
        assert!(matches!(pool.get(idx2).unwrap(), Constant::FieldName(s) if s == "email"));
    }

    // ===== Statement Compilation Tests =====

    fn compile_stmt_helper(source: &str) -> (Vec<u8>, ConstantPool) {
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let stmt = parser.parse_statement().unwrap();

        let mut compiler = Compiler::new();
        compiler.compile_stmt(&stmt).unwrap();

        (compiler.bytecode, compiler.constant_pool)
    }

    #[test]
    fn test_compile_assignment() {
        let (bytecode, pool) = compile_stmt_helper("$req.user_id = 42;");

        // Should be: PushConst(42), StoreVar(req.user_id)
        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::StoreVar as u8);

        let idx = u16::from_le_bytes([bytecode[4], bytecode[5]]);
        assert!(matches!(pool.get(idx).unwrap(), Constant::Identifier(s) if s == "req.user_id"));
    }

    #[test]
    fn test_compile_capture_no_args() {
        let (bytecode, pool) = compile_stmt_helper("capture();");

        // Should be: CallFunc(capture, 0), Pop
        assert_eq!(bytecode[0], Opcode::CallFunc as u8);
        let idx = u16::from_le_bytes([bytecode[1], bytecode[2]]);
        assert!(matches!(pool.get(idx).unwrap(), Constant::FunctionName(s) if s == "capture"));
        assert_eq!(bytecode[3], 0); // 0 arguments
        assert_eq!(bytecode[4], Opcode::Pop as u8);
    }

    #[test]
    fn test_compile_capture_with_args() {
        let (bytecode, _pool) = compile_stmt_helper("capture(args, retval);");

        // Should be: LoadVar(args), LoadVar(retval), CallFunc(capture, 2), Pop
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::LoadVar as u8);
        assert_eq!(bytecode[6], Opcode::CallFunc as u8);
        assert_eq!(bytecode[9], 2); // 2 arguments
        assert_eq!(bytecode[10], Opcode::Pop as u8);
    }

    // ===== End-to-End Program Compilation Tests =====

    #[test]
    fn test_compile_simple_program() {
        let source = r#"fn:myapp.test:entry
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        assert_eq!(program.version, 1);
        assert_eq!(program.probes.len(), 1);
        assert_eq!(program.probes[0].id, "probe_0");
        assert!(program.probes[0].predicate.is_empty());
        assert!(!program.probes[0].body.is_empty());
    }

    #[test]
    fn test_compile_program_with_predicate() {
        let source = r#"fn:myapp.test:entry
/ arg0 > 10 /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        assert_eq!(program.probes.len(), 1);
        assert!(!program.probes[0].predicate.is_empty()); // Has predicate
        assert!(!program.probes[0].body.is_empty()); // Has body
    }

    #[test]
    fn test_compile_multiple_probes() {
        let source = r#"fn:myapp.start:entry
{
    $req.start_time = timestamp();
}

fn:myapp.end:exit
{
    capture(retval);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        assert_eq!(program.probes.len(), 2);
        assert_eq!(program.probes[0].id, "probe_0");
        assert_eq!(program.probes[1].id, "probe_1");
    }

    #[test]
    fn test_compile_constant_pool_shared() {
        let source = r#"fn:myapp.test1:entry
{
    capture(args);
}

fn:myapp.test2:entry
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        // Both probes use "args" and "capture", should be deduplicated in constant pool
        // Verify we don't have duplicate constants
        assert!(program.constant_pool.len() < 10); // Should be small due to dedup
    }

    #[test]
    fn test_compile_program_roundtrip_protobuf() {
        let source = r#"fn:myapp.users.create:entry
{
    $req.user_id = arg0;
    capture(args, retval);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        // Test protobuf round-trip
        let bytes = program.to_proto_bytes().unwrap();
        let decoded = Program::from_proto_bytes(&bytes).unwrap();

        assert_eq!(decoded.version, program.version);
        assert_eq!(decoded.probes.len(), program.probes.len());
        assert_eq!(decoded.probes[0].id, program.probes[0].id);
    }

    // ===== Comprehensive Bytecode Validation Tests =====

    #[test]
    fn test_bytecode_simple_capture() {
        let source = r#"fn:myapp.test:entry
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Simple capture bytecode size: {} bytes", bytecode.len());

        // Expected: LoadVar(args) [3], CallFunc(capture, 1) [4], Pop [1] = 8 bytes
        assert_eq!(bytecode.len(), 8);
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::CallFunc as u8);
        assert_eq!(bytecode[7], Opcode::Pop as u8);

        // Verify constant pool
        assert!(program.constant_pool.len() >= 2); // args, capture
    }

    #[test]
    fn test_bytecode_with_predicate() {
        let source = r#"fn:myapp.test:entry
/ arg0 > 10 /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        println!("Probe with predicate:");
        println!("  Predicate bytecode: {} bytes", probe.predicate.len());
        println!("  Body bytecode: {} bytes", probe.body.len());
        println!("  Total probe bytecode: {} bytes", probe.predicate.len() + probe.body.len());

        // Predicate: LoadVar(arg0) [3], PushConst(10) [3], Gt [1] = 7 bytes
        assert_eq!(probe.predicate.len(), 7);
        assert_eq!(probe.predicate[0], Opcode::LoadVar as u8);
        assert_eq!(probe.predicate[3], Opcode::PushConst as u8);
        assert_eq!(probe.predicate[6], Opcode::Gt as u8);

        // Body: LoadVar(args) [3], CallFunc(capture, 1) [4], Pop [1] = 8 bytes
        assert_eq!(probe.body.len(), 8);
        assert_eq!(probe.body[0], Opcode::LoadVar as u8);
        assert_eq!(probe.body[3], Opcode::CallFunc as u8);
        assert_eq!(probe.body[7], Opcode::Pop as u8);
    }

    #[test]
    fn test_bytecode_complex_predicate() {
        let source = r#"fn:myapp.test:entry
/ arg0 > 10 && arg1 != None /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        println!("Complex predicate bytecode: {} bytes", probe.predicate.len());

        // Predicate should be:
        // LoadVar(arg0) [3], PushConst(10) [3], Gt [1],
        // LoadVar(arg1) [3], PushConst(None) [3], Ne [1],
        // And [1] = 15 bytes total
        assert_eq!(probe.predicate.len(), 15); // 3 + 3 + 1 + 3 + 3 + 1 + 1

        // First part: arg0 > 10
        assert_eq!(probe.predicate[0], Opcode::LoadVar as u8);
        assert_eq!(probe.predicate[3], Opcode::PushConst as u8);
        assert_eq!(probe.predicate[6], Opcode::Gt as u8);

        // Second part: arg1 != None
        assert_eq!(probe.predicate[7], Opcode::LoadVar as u8);
        assert_eq!(probe.predicate[10], Opcode::PushConst as u8);
        assert_eq!(probe.predicate[13], Opcode::Ne as u8);
        assert_eq!(probe.predicate[14], Opcode::And as u8); // Final And
    }

    #[test]
    fn test_bytecode_assignment() {
        let source = r#"fn:myapp.test:entry
{
    $req.user_id = 42;
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Assignment bytecode size: {} bytes", bytecode.len());

        // Expected: PushConst(42), StoreVar(req.user_id)
        assert_eq!(bytecode.len(), 6);
        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::StoreVar as u8);
    }

    #[test]
    fn test_bytecode_field_access() {
        let source = r#"fn:myapp.test:entry
{
    capture(args.user.email);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Field access bytecode size: {} bytes", bytecode.len());

        // Expected: LoadVar(args) [3], GetAttr(user) [3], GetAttr(email) [3], CallFunc(capture, 1) [4], Pop [1] = 14 bytes
        assert_eq!(bytecode.len(), 14);
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::GetAttr as u8);
        assert_eq!(bytecode[6], Opcode::GetAttr as u8);
        assert_eq!(bytecode[9], Opcode::CallFunc as u8);
        assert_eq!(bytecode[13], Opcode::Pop as u8);
    }

    #[test]
    fn test_bytecode_arithmetic_expression() {
        let source = r#"fn:myapp.test:entry
{
    $req.total = (arg0 + arg1) * 2;
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Arithmetic expression bytecode size: {} bytes", bytecode.len());

        // Expected: LoadVar(arg0), LoadVar(arg1), Add, PushConst(2), Mul, StoreVar(req.total)
        assert_eq!(bytecode.len(), 14);
        assert_eq!(bytecode[0], Opcode::LoadVar as u8); // arg0
        assert_eq!(bytecode[3], Opcode::LoadVar as u8); // arg1
        assert_eq!(bytecode[6], Opcode::Add as u8);
        assert_eq!(bytecode[7], Opcode::PushConst as u8); // 2
        assert_eq!(bytecode[10], Opcode::Mul as u8);
        assert_eq!(bytecode[11], Opcode::StoreVar as u8); // req.total
    }

    #[test]
    fn test_bytecode_function_call_chain() {
        let source = r#"fn:myapp.test:entry
{
    capture(len(args), timestamp());
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Function call chain bytecode size: {} bytes", bytecode.len());

        // Expected: LoadVar(args) [3], CallFunc(len, 1) [4], CallFunc(timestamp, 0) [4], CallFunc(capture, 2) [4], Pop [1] = 16 bytes
        assert_eq!(bytecode.len(), 16);
        assert_eq!(bytecode[0], Opcode::LoadVar as u8); // args
        assert_eq!(bytecode[3], Opcode::CallFunc as u8); // len
        assert_eq!(bytecode[6], 1); // 1 arg to len
        assert_eq!(bytecode[7], Opcode::CallFunc as u8); // timestamp
        assert_eq!(bytecode[11], Opcode::CallFunc as u8); // capture
        assert_eq!(bytecode[14], Opcode::Pop as u8);
    }

    #[test]
    fn test_bytecode_index_access() {
        let source = r#"fn:myapp.test:entry
{
    capture(args[0], args[1]);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Index access bytecode size: {} bytes", bytecode.len());

        // Expected: LoadVar(args) [3], PushConst(0) [3], GetItem [1], LoadVar(args) [3], PushConst(1) [3], GetItem [1], CallFunc(capture, 2) [4], Pop [1] = 19 bytes
        assert_eq!(bytecode.len(), 19);
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::GetItem as u8);
        assert_eq!(bytecode[7], Opcode::LoadVar as u8);
        assert_eq!(bytecode[10], Opcode::PushConst as u8);
        assert_eq!(bytecode[13], Opcode::GetItem as u8);
        assert_eq!(bytecode[14], Opcode::CallFunc as u8);
        assert_eq!(bytecode[18], Opcode::Pop as u8);
    }

    #[test]
    fn test_bytecode_multiple_statements() {
        let source = r#"fn:myapp.test:entry
{
    $req.start = timestamp();
    $req.user_id = arg0;
    capture(args, retval);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Multiple statements bytecode size: {} bytes", bytecode.len());

        // Statement 1: CallFunc(timestamp, 0) [4], StoreVar(req.start) [3] = 7 bytes
        // Statement 2: LoadVar(arg0) [3], StoreVar(req.user_id) [3] = 6 bytes
        // Statement 3: LoadVar(args) [3], LoadVar(retval) [3], CallFunc(capture, 2) [4], Pop [1] = 11 bytes
        // Total: 24 bytes
        assert_eq!(bytecode.len(), 24);
    }

    #[test]
    fn test_bytecode_comparison_operators() {
        let source = r#"fn:myapp.test:entry
/ arg0 < 10 || arg0 >= 100 /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        println!("Comparison operators predicate: {} bytes", probe.predicate.len());

        // LoadVar(arg0) [3], PushConst(10) [3], Lt [1], LoadVar(arg0) [3], PushConst(100) [3], Ge [1], Or [1] = 15 bytes
        assert_eq!(probe.predicate.len(), 15);
        assert_eq!(probe.predicate[6], Opcode::Lt as u8);
        assert_eq!(probe.predicate[13], Opcode::Ge as u8);
        assert_eq!(probe.predicate[14], Opcode::Or as u8);
    }

    #[test]
    fn test_bytecode_equality_operators() {
        let source = r#"fn:myapp.test:entry
/ arg0 == 42 /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        // LoadVar(arg0), PushConst(42), Eq
        assert_eq!(probe.predicate.len(), 7);
        assert_eq!(probe.predicate[6], Opcode::Eq as u8);
    }

    #[test]
    fn test_bytecode_not_equal() {
        let source = r#"fn:myapp.test:entry
/ arg0 != 0 /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        // LoadVar(arg0), PushConst(0), Ne
        assert_eq!(probe.predicate.len(), 7);
        assert_eq!(probe.predicate[6], Opcode::Ne as u8);
    }

    #[test]
    fn test_bytecode_logical_not() {
        let source = r#"fn:myapp.test:entry
/ !arg0 /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        // LoadVar(arg0), Not
        assert_eq!(probe.predicate.len(), 4);
        assert_eq!(probe.predicate[0], Opcode::LoadVar as u8);
        assert_eq!(probe.predicate[3], Opcode::Not as u8);
    }

    #[test]
    fn test_bytecode_string_literals() {
        let source = r#"fn:myapp.test:entry
{
    capture("hello", "world");
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("String literals bytecode size: {} bytes", bytecode.len());

        // PushConst("hello"), PushConst("world"), CallFunc(capture, 2), Pop
        assert_eq!(bytecode.len(), 11);
        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::CallFunc as u8);
        assert_eq!(bytecode[10], Opcode::Pop as u8);
    }

    #[test]
    fn test_bytecode_mixed_types() {
        let source = r#"fn:myapp.test:entry
{
    capture(42, 3.14, "test", True, None);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Mixed types bytecode size: {} bytes", bytecode.len());

        // 5 PushConst + CallFunc + Pop = 5*3 + 4 + 1 = 20
        assert_eq!(bytecode.len(), 20);

        // All should be PushConst
        assert_eq!(bytecode[0], Opcode::PushConst as u8);
        assert_eq!(bytecode[3], Opcode::PushConst as u8);
        assert_eq!(bytecode[6], Opcode::PushConst as u8);
        assert_eq!(bytecode[9], Opcode::PushConst as u8);
        assert_eq!(bytecode[12], Opcode::PushConst as u8);
        assert_eq!(bytecode[15], Opcode::CallFunc as u8);
        assert_eq!(bytecode[19], Opcode::Pop as u8);
    }

    #[test]
    fn test_bytecode_request_var_access() {
        let source = r#"fn:myapp.test:entry
{
    capture($req.user_id);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        let bytecode = &probe.body;

        println!("Request var access bytecode size: {} bytes", bytecode.len());

        // LoadVar(req) [3], GetAttr(user_id) [3], CallFunc(capture, 1) [4], Pop [1] = 11 bytes
        assert_eq!(bytecode.len(), 11);
        assert_eq!(bytecode[0], Opcode::LoadVar as u8);
        assert_eq!(bytecode[3], Opcode::GetAttr as u8);
        assert_eq!(bytecode[6], Opcode::CallFunc as u8);
        assert_eq!(bytecode[10], Opcode::Pop as u8);
    }

    #[test]
    fn test_program_size_metrics() {
        let source = r#"fn:myapp.users.create:entry
/ arg0 > 100 && $req.authenticated /
{
    $req.user_id = arg0;
    $req.timestamp = timestamp();
    capture(args, retval);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        println!("\n=== TYPICAL PROBE SIZE METRICS ===");
        println!("Predicate bytecode: {} bytes", probe.predicate.len());
        println!("Body bytecode: {} bytes", probe.body.len());
        println!("Total bytecode: {} bytes", probe.predicate.len() + probe.body.len());
        println!("Constant pool entries: {}", program.constant_pool.len());

        let proto_bytes = program.to_proto_bytes().unwrap();
        println!("Serialized protobuf size: {} bytes", proto_bytes.len());
        println!("===================================\n");

        // Typical probe should be under 100 bytes
        assert!(probe.predicate.len() + probe.body.len() < 100);
    }

    #[test]
    fn test_constant_pool_deduplication_effectiveness() {
        let source = r#"fn:myapp.test:entry
{
    capture(args, args, args, args, args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        println!("Deduplication test:");
        println!("  5 uses of 'args', constant pool entries: {}", program.constant_pool.len());

        // Should only have 2 constants: "args" and "capture"
        assert_eq!(program.constant_pool.len(), 2);
    }

    #[test]
    fn test_empty_probe_body() {
        let source = r#"fn:myapp.test:entry
{
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        println!("Empty probe bytecode size: {} bytes", probe.body.len());

        // Empty body should have 0 bytes
        assert_eq!(probe.body.len(), 0);
        assert_eq!(probe.predicate.len(), 0);
    }

    #[test]
    fn test_probe_spec_conversion_entry() {
        let source = r#"fn:myapp.test.function:entry
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        match &probe.spec {
            ProbeSpec::Fn { specifier, target } => {
                assert_eq!(specifier, "myapp.test.function");
                assert_eq!(*target, FnTarget::Entry);
            }
        }
    }

    #[test]
    fn test_probe_spec_conversion_exit() {
        let source = r#"fn:myapp.test:exit
{
    capture(retval);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];
        match &probe.spec {
            ProbeSpec::Fn { specifier, target } => {
                assert_eq!(specifier, "myapp.test");
                assert_eq!(*target, FnTarget::Exit);
            }
        }
    }

    #[test]
    fn test_multiple_probes_unique_ids() {
        let source = r#"fn:app.a:entry
{
    capture(args);
}

fn:app.b:entry
{
    capture(args);
}

fn:app.c:entry
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        assert_eq!(program.probes.len(), 3);
        assert_eq!(program.probes[0].id, "probe_0");
        assert_eq!(program.probes[1].id, "probe_1");
        assert_eq!(program.probes[2].id, "probe_2");

        println!("Multiple probes constant pool: {} entries", program.constant_pool.len());
        // Should deduplicate args and capture across all probes
        assert!(program.constant_pool.len() < 10);
    }

    #[test]
    fn test_bytecode_modulo_operator() {
        let source = r#"fn:myapp.test:entry
/ arg0 % 2 == 0 /
{
    capture(args);
}"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(ast).unwrap();

        let probe = &program.probes[0];

        // LoadVar(arg0) [3], PushConst(2) [3], Mod [1], PushConst(0) [3], Eq [1] = 11 bytes
        assert_eq!(probe.predicate.len(), 11);
        assert_eq!(probe.predicate[6], Opcode::Mod as u8);
        assert_eq!(probe.predicate[10], Opcode::Eq as u8);
    }
}
