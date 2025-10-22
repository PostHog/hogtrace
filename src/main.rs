use vm::{Constant, ConstantPool, Executor, FnTarget, Opcode, Probe, ProbeSpec, Program, Value};
use vm::dispatcher::Dispatcher;
use vm::parser::{Lexer, Parser, Compiler};

// Helper to clone a Value (since Value doesn't implement Clone due to Object variant)
fn clone_value(value: &Value) -> Value {
    match value {
        Value::Bool(b) => Value::Bool(*b),
        Value::Int(i) => Value::Int(*i),
        Value::Float(f) => Value::Float(*f),
        Value::String(s) => Value::String(s.clone()),
        Value::None => Value::None,
        Value::Object(_) => Value::None, // Can't clone objects, return None
    }
}

/// Example minimal dispatcher for demonstration
struct SimpleDispatcher {
    variables: std::collections::HashMap<String, Value>,
}

impl SimpleDispatcher {
    fn new() -> Self {
        let mut variables = std::collections::HashMap::new();
        // Pre-populate some test variables
        variables.insert("args".to_string(), Value::Int(42));
        variables.insert("arg0".to_string(), Value::Int(150));
        variables.insert("arg1".to_string(), Value::String("test@example.com".to_string()));
        variables.insert("arg2".to_string(), Value::Bool(true));

        Self { variables }
    }
}

impl Dispatcher for SimpleDispatcher {
    fn load_variable(&mut self, name: &str) -> Result<Value, String> {
        // Handle request-scoped variables
        if name.starts_with("req.") || name.starts_with("request.") {
            return self.variables
                .get(name)
                .map(|v| clone_value(v))
                .ok_or_else(|| format!("Request variable '{}' not set", name));
        }

        // Load regular variables
        self.variables
            .get(name)
            .map(|v| clone_value(v))
            .ok_or_else(|| format!("Unknown variable: {}", name))
    }

    fn store_variable(&mut self, name: &str, value: Value) -> Result<(), String> {
        self.variables.insert(name.to_string(), value);
        Ok(())
    }

    fn get_attribute(&mut self, obj: &Value, attr: &str) -> Result<Value, String> {
        // Simulate object attribute access
        match (obj, attr) {
            (Value::Object(_), "email") => Ok(Value::String("user@example.com".to_string())),
            (Value::Object(_), "id") => Ok(Value::Int(123)),
            (Value::Object(_), "active") => Ok(Value::Bool(true)),
            _ => Err(format!("Attribute '{}' not found", attr)),
        }
    }

    fn get_item(&mut self, obj: &Value, key: &Value) -> Result<Value, String> {
        match (obj, key) {
            (Value::String(s), Value::Int(i)) => {
                let idx = *i as usize;
                s.chars()
                    .nth(idx)
                    .map(|c| Value::String(c.to_string()))
                    .ok_or_else(|| format!("Index {} out of range", i))
            }
            _ => Err(format!("Cannot index {:?} with {:?}", obj, key)),
        }
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "capture" => {
                println!("    📸 Captured: {:?}", args);
                Ok(Value::None)
            }
            "send" => {
                println!("    📤 Sent: {:?}", args);
                Ok(Value::None)
            }
            "len" => {
                let len = args.len() as i64;
                Ok(Value::Int(len))
            }
            "timestamp" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                Ok(Value::Int(ts))
            }
            _ => Err(format!("Unknown function: {}", name)),
        }
    }
}

fn print_separator() {
    println!("\n{}", "=".repeat(80));
}

fn print_header(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!("  {}", title);
    println!("{}", "=".repeat(80));
}

fn example_manual_bytecode() {
    print_header("Example 1: Manual Bytecode Construction");

    println!("\nThis example shows low-level bytecode construction.");
    println!("Building a probe manually with predicate: arg0 > 10");

    let mut pool = ConstantPool::new();
    let const_10 = pool.add(Constant::Int(10));
    let var_arg0 = pool.add(Constant::Identifier("arg0".to_string()));
    let func_capture = pool.add(Constant::FunctionName("capture".to_string()));

    println!("\nConstant Pool:");
    println!("  [{}] Int(10)", const_10);
    println!("  [{}] Identifier(\"arg0\")", var_arg0);
    println!("  [{}] FunctionName(\"capture\")", func_capture);

    // Predicate: arg0 > 10
    let predicate = vec![
        Opcode::LoadVar as u8,
        (var_arg0 & 0xFF) as u8,
        ((var_arg0 >> 8) & 0xFF) as u8,
        Opcode::PushConst as u8,
        (const_10 & 0xFF) as u8,
        ((const_10 >> 8) & 0xFF) as u8,
        Opcode::Gt as u8,
    ];

    // Body: capture(arg0)
    let body = vec![
        Opcode::LoadVar as u8,
        (var_arg0 & 0xFF) as u8,
        ((var_arg0 >> 8) & 0xFF) as u8,
        Opcode::CallFunc as u8,
        (func_capture & 0xFF) as u8,
        ((func_capture >> 8) & 0xFF) as u8,
        1,
    ];

    let probe = Probe {
        id: "manual_probe".to_string(),
        spec: ProbeSpec::Fn {
            specifier: "myapp.handler".to_string(),
            target: FnTarget::Entry,
        },
        predicate: predicate.clone(),
        body: body.clone(),
    };

    let program = Program {
        version: vm::BYTECODE_VERSION,
        constant_pool: pool,
        probes: vec![probe],
        sampling: 1.0,
    };

    println!("\nBytecode (hex):");
    println!("  Predicate: {}", hex_dump(&predicate));
    println!("  Body: {}", hex_dump(&body));

    // Execute
    println!("\nExecuting probe...");
    let mut dispatcher = SimpleDispatcher::new();
    let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);

    match executor.execute(&predicate) {
        Ok(Value::Bool(result)) => {
            println!("  ✓ Predicate evaluated to: {} (arg0={}, 150 > 10)", result, 150);

            if result {
                let mut dispatcher = SimpleDispatcher::new();
                let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);
                let _ = executor.execute(&body);
            }
        }
        Ok(other) => println!("  ✗ Unexpected result: {:?}", other),
        Err(e) => println!("  ✗ Error: {}", e),
    }
}

fn example_parser_success() {
    print_header("Example 2: Parser - Successful Compilation");

    let source = r#"
fn:myapp.users.authenticate:entry
/ arg0 > 100 && arg1 != None /
{
    $req.user_id = arg0;
    $req.timestamp = timestamp();
    capture(user_id=$req.user_id, email=arg1);
}
"#;

    println!("\nSource code:");
    println!("{}", indent_code(source));

    println!("\nParsing...");
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    match parser.parse_program() {
        Ok(ast) => {
            println!("  ✓ Parsing successful!");
            println!("  Probes: {}", ast.probes.len());

            for (i, probe) in ast.probes.iter().enumerate() {
                println!("\n  Probe {}:", i);
                println!("    Spec: {}", probe.spec.module_function);
                println!("    Has predicate: {}", probe.predicate.is_some());
                println!("    Statements: {}", probe.body.len());
            }

            println!("\nCompiling to bytecode...");
            let mut compiler = Compiler::new();

            match compiler.compile(ast) {
                Ok(program) => {
                    println!("  ✓ Compilation successful!");
                    println!("  Bytecode version: {}", program.version);
                    println!("  Constant pool size: {}", program.constant_pool.len());
                    println!("  Total bytecode size: {} bytes",
                        program.probes.iter()
                            .map(|p| p.predicate.len() + p.body.len())
                            .sum::<usize>());

                    // Show constant pool
                    println!("\n  Constant Pool:");
                    for i in 0..program.constant_pool.len().min(10) {
                        if let Ok(constant) = program.constant_pool.get(i as u16) {
                            println!("    [{}] {:?}", i, constant);
                        }
                    }
                    if program.constant_pool.len() > 10 {
                        println!("    ... ({} more)", program.constant_pool.len() - 10);
                    }

                    // Execute the probe
                    println!("\nExecuting probe...");
                    if let Some(probe) = program.probes.first() {
                        let mut dispatcher = SimpleDispatcher::new();
                        let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);

                        // Execute predicate
                        if !probe.predicate.is_empty() {
                            match executor.execute(&probe.predicate) {
                                Ok(Value::Bool(true)) => {
                                    println!("  ✓ Predicate: true");

                                    // Execute body
                                    let mut dispatcher = SimpleDispatcher::new();
                                    let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);
                                    match executor.execute(&probe.body) {
                                        Ok(_) => println!("  ✓ Body executed successfully"),
                                        Err(e) => println!("  ✗ Body error: {}", e),
                                    }
                                }
                                Ok(Value::Bool(false)) => println!("  ✓ Predicate: false (body skipped)"),
                                Ok(other) => println!("  ✗ Predicate returned non-boolean: {:?}", other),
                                Err(e) => println!("  ✗ Predicate error: {}", e),
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("  ✗ Compilation failed:");
                    println!("{}", format_error(source, &e));
                }
            }
        }
        Err(e) => {
            println!("  ✗ Parsing failed:");
            println!("{}", format_error(source, &e));
        }
    }
}

fn example_parser_errors() {
    print_header("Example 3: Parser - Error Handling");

    let test_cases = vec![
        (
            "Missing colon in probe spec",
            "fn myapp.test entry { capture(args); }",
        ),
        (
            "Invalid probe point",
            "fn:myapp.test:entr { capture(args); }",
        ),
        (
            "Unclosed predicate",
            "fn:test:entry / arg0 > 10 { capture(args); }",
        ),
        (
            "Missing semicolon",
            "fn:test:entry { capture(args) }",
        ),
        (
            "Invalid assignment target",
            "fn:test:entry { arg0 = 10; }",
        ),
        (
            "Empty probe spec",
            "::: { capture(args); }",
        ),
    ];

    for (i, (description, source)) in test_cases.iter().enumerate() {
        println!("\n{}. {}", i + 1, description);
        println!("   Source: {}", source);

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);

        match parser.parse_program() {
            Ok(_) => println!("   ✓ Unexpectedly succeeded!"),
            Err(e) => {
                println!("   ✗ Error caught:");
                for line in format_error(source, &e).lines() {
                    println!("   {}", line);
                }
            }
        }
    }
}

fn example_complex_program() {
    print_header("Example 4: Complex Multi-Probe Program");

    let source = r#"
// Track user authentication
fn:auth.login:entry
/ len(args) > 0 && arg0 != None /
{
    $req.auth_start = timestamp();
    $req.user_email = arg0;
    capture(event="login_start", email=$req.user_email);
}

fn:auth.login:exit
{
    $req.auth_duration = timestamp() - $req.auth_start;
    capture(
        event="login_complete",
        duration=$req.auth_duration,
        success=retval
    );
}

// Monitor API endpoints with sampling
fn:api.*.handler:entry
{
    sample 10%;
    $req.api_start = timestamp();
    capture(endpoint=arg0, method=arg1);
}

fn:api.*.handler:exit
{
    send(
        endpoint=arg0,
        duration=timestamp() - $req.api_start,
        status=retval
    );
}
"#;

    println!("\nSource code:");
    println!("{}", indent_code(source));

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    match parser.parse_program() {
        Ok(ast) => {
            println!("\n✓ Parsed {} probes", ast.probes.len());

            let mut compiler = Compiler::new();
            match compiler.compile(ast) {
                Ok(program) => {
                    println!("✓ Compiled successfully");
                    println!("\nProgram Statistics:");
                    println!("  Probes: {}", program.probes.len());
                    println!("  Constants: {}", program.constant_pool.len());

                    let total_bytecode: usize = program.probes.iter()
                        .map(|p| p.predicate.len() + p.body.len())
                        .sum();
                    println!("  Total bytecode: {} bytes", total_bytecode);

                    // Breakdown by probe
                    println!("\n  Per-Probe Breakdown:");
                    for (i, probe) in program.probes.iter().enumerate() {
                        let total = probe.predicate.len() + probe.body.len();
                        println!("    Probe {}: {} bytes (pred: {}, body: {})",
                            i, total, probe.predicate.len(), probe.body.len());
                    }

                    // Test protobuf serialization
                    println!("\nProtobuf Serialization:");
                    match program.to_proto_bytes() {
                        Ok(bytes) => {
                            println!("  ✓ Serialized to {} bytes", bytes.len());

                            match Program::from_proto_bytes(&bytes) {
                                Ok(decoded) => {
                                    println!("  ✓ Deserialized successfully");

                                    // Verify integrity
                                    let matches = decoded.version == program.version
                                        && decoded.probes.len() == program.probes.len()
                                        && decoded.constant_pool.len() == program.constant_pool.len();

                                    if matches {
                                        println!("  ✓ Round-trip verification passed");
                                    } else {
                                        println!("  ✗ Round-trip verification failed");
                                    }
                                }
                                Err(e) => println!("  ✗ Deserialization error: {}", e),
                            }
                        }
                        Err(e) => println!("  ✗ Serialization error: {}", e),
                    }
                }
                Err(e) => {
                    println!("✗ Compilation failed:");
                    println!("{}", format_error(source, &e));
                }
            }
        }
        Err(e) => {
            println!("✗ Parsing failed:");
            println!("{}", format_error(source, &e));
        }
    }
}

fn example_edge_cases() {
    print_header("Example 5: Edge Cases and Advanced Features");

    let test_cases = vec![
        (
            "Unicode in strings",
            r#"fn:test:entry { capture("Hello 世界 🌍"); }"#,
        ),
        (
            "Deep nesting",
            r#"fn:test:entry { capture(a[b[c]].d.e[f].g); }"#,
        ),
        (
            "Complex arithmetic",
            r#"fn:test:entry { $req.result = (a + b) * c - d / e % f; }"#,
        ),
        (
            "Division in predicate",
            r#"fn:test:entry / (arg0 / 2) > 50 / { capture(args); }"#,
        ),
        (
            "Multiple logical operators",
            r#"fn:test:entry / a && b || c && !d || e / { capture(args); }"#,
        ),
        (
            "Wildcards in spec",
            r#"fn:myapp.*.handler:entry { capture(args); }"#,
        ),
        (
            "Empty body",
            r#"fn:test:entry { }"#,
        ),
    ];

    for (i, (description, source)) in test_cases.iter().enumerate() {
        println!("\n{}. {}", i + 1, description);

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);

        match parser.parse_program() {
            Ok(ast) => {
                let mut compiler = Compiler::new();
                match compiler.compile(ast) {
                    Ok(program) => {
                        let size: usize = program.probes.iter()
                            .map(|p| p.predicate.len() + p.body.len())
                            .sum();
                        println!("   ✓ Compiled successfully ({} bytes)", size);
                    }
                    Err(e) => println!("   ✗ Compilation error: {}", e),
                }
            }
            Err(e) => println!("   ✗ Parse error: {}", e),
        }
    }
}

// Helper functions

fn hex_dump(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn indent_code(code: &str) -> String {
    code.lines()
        .map(|line| format!("  {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_error(source: &str, error: &vm::parser::ParseError) -> String {
    let mut output = String::new();

    output.push_str(&format!("  Error: {}\n", error.message));

    // Show the line with the error
    let span = &error.span;
    let lines: Vec<&str> = source.lines().collect();
    let mut pos = 0;

    for (line_num, line) in lines.iter().enumerate() {
        let line_start = pos;
        let line_end = pos + line.len();

        if span.start.offset >= line_start && span.start.offset <= line_end {
            output.push_str(&format!("\n  {}| {}\n", line_num + 1, line));

            // Add caret indicator
            let column = span.start.offset - line_start;
            let length = (span.end.offset - span.start.offset).max(1).min(line.len() - column);

            output.push_str(&format!("  {}| {}{}\n",
                " ".repeat((line_num + 1).to_string().len()),
                " ".repeat(column),
                "^".repeat(length)));

            break;
        }

        pos = line_end + 1; // +1 for newline
    }

    if let Some(suggestion) = &error.suggestion {
        output.push_str(&format!("\n  Suggestion: {}\n", suggestion));
    }

    output
}

fn main() {
    println!("{}", "━".repeat(80));
    println!("  🔥 HogTrace VM - Complete Library Showcase");
    println!("{}", "━".repeat(80));

    example_manual_bytecode();
    example_parser_success();
    example_parser_errors();
    example_complex_program();
    example_edge_cases();

    print_separator();
    println!("\n✨ All examples completed!");
    println!("\nKey Features Demonstrated:");
    println!("  ✓ Manual bytecode construction");
    println!("  ✓ High-level parser (HogTrace → AST)");
    println!("  ✓ Compiler (AST → bytecode)");
    println!("  ✓ Rich error messages with source context");
    println!("  ✓ Bytecode execution with custom dispatcher");
    println!("  ✓ Protobuf serialization/deserialization");
    println!("  ✓ Multi-probe programs");
    println!("  ✓ Complex predicates and expressions");
    println!("  ✓ Edge case handling");
    println!("\n{}", "━".repeat(80));
}
