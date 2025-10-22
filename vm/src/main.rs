use vm::{Constant, ConstantPool, Executor, FnTarget, Opcode, Probe, ProbeSpec, Program, Value};
use vm::dispatcher::Dispatcher;

/// Example minimal dispatcher for demonstration
struct SimpleDispatcher;

impl Dispatcher for SimpleDispatcher {
    fn load_variable(&mut self, name: &str) -> Result<Value, String> {
        // Simulate some variables
        match name {
            "args" => Ok(Value::Int(10)),
            "arg0" => Ok(Value::Int(5)),
            _ => Err(format!("Unknown variable: {}", name)),
        }
    }

    fn store_variable(&mut self, _name: &str, _value: Value) -> Result<(), String> {
        Ok(())
    }

    fn get_attribute(&mut self, _obj: &Value, _attr: &str) -> Result<Value, String> {
        Ok(Value::None)
    }

    fn get_item(&mut self, _obj: &Value, _key: &Value) -> Result<Value, String> {
        Ok(Value::None)
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "capture" => {
                println!("Capturing: {:?}", args);
                Ok(Value::None)
            }
            _ => Err(format!("Unknown function: {}", name)),
        }
    }
}

fn main() {
    println!("HogTrace Bytecode VM Example\n");

    // Build a simple program with constant pool
    let mut pool = ConstantPool::new();

    // Add constants
    let const_10 = pool.add(Constant::Int(10));
    let const_100 = pool.add(Constant::Int(100));
    let var_arg0 = pool.add(Constant::Identifier("arg0".to_string()));
    let func_capture = pool.add(Constant::FunctionName("capture".to_string()));

    println!("Constant Pool:");
    println!("  [{}] Int(10)", const_10);
    println!("  [{}] Int(100)", const_100);
    println!("  [{}] Identifier(\"arg0\")", var_arg0);
    println!("  [{}] FunctionName(\"capture\")", func_capture);
    println!();

    // Build predicate bytecode: arg0 > 10
    // Stack operations:
    //   LOAD_VAR "arg0"  -> [5]
    //   PUSH_CONST 10    -> [5, 10]
    //   GT               -> [false]
    let predicate = vec![
        Opcode::LoadVar as u8,
        (var_arg0 & 0xFF) as u8,
        ((var_arg0 >> 8) & 0xFF) as u8,
        Opcode::PushConst as u8,
        (const_10 & 0xFF) as u8,
        ((const_10 >> 8) & 0xFF) as u8,
        Opcode::Gt as u8,
    ];

    // Build body bytecode: capture(arg0)
    // Stack operations:
    //   LOAD_VAR "arg0"     -> [5]
    //   CALL_FUNC "capture", 1  -> [None]
    let body = vec![
        Opcode::LoadVar as u8,
        (var_arg0 & 0xFF) as u8,
        ((var_arg0 >> 8) & 0xFF) as u8,
        Opcode::CallFunc as u8,
        (func_capture & 0xFF) as u8,
        ((func_capture >> 8) & 0xFF) as u8,
        1, // arg count
    ];

    // Create a probe
    let probe = Probe {
        id: "example_probe".to_string(),
        spec: ProbeSpec::Fn {
            specifier: "myapp.users.create".to_string(),
            target: FnTarget::Entry,
        },
        predicate: predicate.clone(),
        body: body.clone(),
    };

    // Create a program
    let program = Program {
        version: vm::BYTECODE_VERSION,
        constant_pool: pool,
        probes: vec![probe],
        sampling: 1.0,
    };

    println!("Program:");
    println!("  Version: {}", program.version);
    println!("  Sampling: {}%", program.sampling * 100.0);
    println!("  Probes: {}", program.probes.len());
    println!();

    // Execute the probe
    println!("Executing probe:");
    println!("  Spec: fn:myapp.users.create:entry");
    println!();

    // Execute predicate
    let mut dispatcher = SimpleDispatcher;
    let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);

    println!("1. Evaluating predicate: arg0 > 10");
    match executor.execute(&predicate) {
        Ok(Value::Bool(result)) => {
            println!("   Result: {} (arg0=5, so 5 > 10 is false)", result);

            if result {
                println!("\n2. Predicate is true, executing body: capture(arg0)");
                let mut dispatcher = SimpleDispatcher;
                let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);
                if let Err(e) = executor.execute(&body) {
                    println!("   Error: {}", e);
                }
            } else {
                println!("\n2. Predicate is false, skipping body execution");
            }
        }
        Ok(other) => println!("   Unexpected result type: {:?}", other),
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n--- Testing Protobuf Serialization ---\n");

    // Serialize to protobuf
    match program.to_proto_bytes() {
        Ok(bytes) => {
            println!("Serialized program to {} bytes", bytes.len());

            // Deserialize back
            match Program::from_proto_bytes(&bytes) {
                Ok(decoded) => {
                    println!("Successfully deserialized program");
                    println!("  Version: {}", decoded.version);
                    println!("  Constant pool size: {}", decoded.constant_pool.len());
                    println!("  Probes: {}", decoded.probes.len());
                }
                Err(e) => println!("Deserialization error: {}", e),
            }
        }
        Err(e) => println!("Serialization error: {}", e),
    }
}
