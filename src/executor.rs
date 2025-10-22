use crate::constant_pool::ConstantPool;
use crate::dispatcher::{BinaryOp, ComparisonOp, Dispatcher};
use crate::opcodes::Opcode;
use crate::value::Value;

/// Bytecode executor for the HogTrace VM
///
/// This is a stack-based VM with no control flow. Execution proceeds linearly
/// from start to end, and the final value on the stack is the result.
///
/// The executor is generic over a Dispatcher, which handles all language-specific
/// operations (variable access, function calls, attribute access, etc.).
pub struct Executor<'a, D: Dispatcher> {
    /// Reference to the constant pool
    constant_pool: &'a ConstantPool,

    /// Value stack
    stack: Vec<Value>,

    /// Language-specific dispatcher
    dispatcher: &'a mut D,
}

impl<'a, D: Dispatcher> Executor<'a, D> {
    /// Create a new executor
    ///
    /// # Arguments
    /// * `constant_pool` - The constant pool for the program
    /// * `dispatcher` - Language-specific dispatcher for operations
    pub fn new(constant_pool: &'a ConstantPool, dispatcher: &'a mut D) -> Self {
        Executor {
            constant_pool,
            stack: Vec::with_capacity(32), // Pre-allocate for efficiency
            dispatcher,
        }
    }

    /// Execute bytecode and return the result
    ///
    /// For predicates: returns the top value on the stack (should be a boolean)
    /// For action bodies: executes all statements, result may be None if no value produced
    ///
    /// Returns Err if execution fails (invalid bytecode, runtime error, etc.)
    pub fn execute(&mut self, bytecode: &[u8]) -> Result<Value, String> {
        let mut i = 0;

        while i < bytecode.len() {
            let opcode_byte = bytecode[i];
            i += 1;

            let opcode = Opcode::from_u8(opcode_byte)?;

            match opcode {
                Opcode::PushConst => {
                    let index = self.read_u16(bytecode, &mut i)?;
                    let value = self.constant_pool.get_value(index)?;
                    self.stack.push(value);
                }

                Opcode::Pop => {
                    self.pop()?;
                }

                Opcode::Dup => {
                    // Note: Dup is currently not supported because Value doesn't implement Clone
                    // (due to Object variant containing non-cloneable trait objects)
                    // If needed in the future, we can handle primitives specially
                    return Err("DUP instruction not yet implemented".to_string());
                }

                Opcode::LoadVar => {
                    let index = self.read_u16(bytecode, &mut i)?;
                    let name = self.constant_pool.get_string(index)?;
                    let value = self.dispatcher.load_variable(name)?;
                    self.stack.push(value);
                }

                Opcode::StoreVar => {
                    let index = self.read_u16(bytecode, &mut i)?;
                    let name = self.constant_pool.get_string(index)?;
                    let value = self.pop()?;
                    self.dispatcher.store_variable(name, value)?;
                }

                // Arithmetic operations
                Opcode::Add => self.binary_op(BinaryOp::Add)?,
                Opcode::Sub => self.binary_op(BinaryOp::Sub)?,
                Opcode::Mul => self.binary_op(BinaryOp::Mul)?,
                Opcode::Div => self.binary_op(BinaryOp::Div)?,
                Opcode::Mod => self.binary_op(BinaryOp::Mod)?,

                // Comparison operations
                Opcode::Eq => self.comparison_op(ComparisonOp::Eq)?,
                Opcode::Ne => self.comparison_op(ComparisonOp::Ne)?,
                Opcode::Lt => self.comparison_op(ComparisonOp::Lt)?,
                Opcode::Gt => self.comparison_op(ComparisonOp::Gt)?,
                Opcode::Le => self.comparison_op(ComparisonOp::Le)?,
                Opcode::Ge => self.comparison_op(ComparisonOp::Ge)?,

                // Logical operations
                Opcode::And => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    let result = left.is_truthy() && right.is_truthy();
                    self.stack.push(Value::Bool(result));
                }

                Opcode::Or => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    let result = left.is_truthy() || right.is_truthy();
                    self.stack.push(Value::Bool(result));
                }

                Opcode::Not => {
                    let value = self.pop()?;
                    let result = !value.is_truthy();
                    self.stack.push(Value::Bool(result));
                }

                // Field/attribute access
                Opcode::GetAttr => {
                    let index = self.read_u16(bytecode, &mut i)?;
                    let attr_name = self.constant_pool.get_string(index)?;
                    let obj = self.pop()?;
                    let value = self.dispatcher.get_attribute(&obj, attr_name)?;
                    self.stack.push(value);
                }

                Opcode::SetAttr => {
                    let index = self.read_u16(bytecode, &mut i)?;
                    let attr_name = self.constant_pool.get_string(index)?;
                    let value = self.pop()?;
                    let obj = self.pop()?;
                    self.dispatcher.set_attribute(&obj, attr_name, value)?;
                }

                Opcode::GetItem => {
                    let key = self.pop()?;
                    let obj = self.pop()?;
                    let value = self.dispatcher.get_item(&obj, &key)?;
                    self.stack.push(value);
                }

                // Function calls
                Opcode::CallFunc => {
                    let name_index = self.read_u16(bytecode, &mut i)?;
                    let arg_count = self.read_u8(bytecode, &mut i)? as usize;
                    let func_name = self.constant_pool.get_string(name_index)?;

                    // Pop arguments from stack
                    if self.stack.len() < arg_count {
                        return Err(format!(
                            "Stack underflow: need {} args for {}(), but only {} on stack",
                            arg_count,
                            func_name,
                            self.stack.len()
                        ));
                    }

                    let args_start = self.stack.len() - arg_count;
                    let args: Vec<Value> = self.stack.drain(args_start..).collect();

                    let result = self.dispatcher.call_function(func_name, args)?;
                    self.stack.push(result);
                }
            }
        }

        // Pop the final result from the stack
        // If stack is empty, return None
        Ok(self.stack.pop().unwrap_or(Value::None))
    }

    /// Helper: read a u8 from bytecode and advance index
    #[inline]
    fn read_u8(&self, bytecode: &[u8], i: &mut usize) -> Result<u8, String> {
        if *i >= bytecode.len() {
            return Err("Unexpected end of bytecode while reading u8".to_string());
        }
        let value = bytecode[*i];
        *i += 1;
        Ok(value)
    }

    /// Helper: read a u16 from bytecode (little-endian) and advance index
    #[inline]
    fn read_u16(&self, bytecode: &[u8], i: &mut usize) -> Result<u16, String> {
        if *i + 2 > bytecode.len() {
            return Err("Unexpected end of bytecode while reading u16".to_string());
        }
        let value = u16::from_le_bytes([bytecode[*i], bytecode[*i + 1]]);
        *i += 2;
        Ok(value)
    }

    /// Helper: pop a value from the stack
    #[inline]
    fn pop(&mut self) -> Result<Value, String> {
        self.stack
            .pop()
            .ok_or_else(|| "Stack underflow".to_string())
    }

    /// Helper: peek at the top of the stack without popping
    #[inline]
    fn peek(&self) -> Result<&Value, String> {
        self.stack
            .last()
            .ok_or_else(|| "Stack is empty".to_string())
    }

    /// Helper: execute a binary operation
    fn binary_op(&mut self, op: BinaryOp) -> Result<(), String> {
        let right = self.pop()?;
        let left = self.pop()?;
        let result = self.dispatcher.binary_op(op, &left, &right)?;
        self.stack.push(result);
        Ok(())
    }

    /// Helper: execute a comparison operation
    fn comparison_op(&mut self, op: ComparisonOp) -> Result<(), String> {
        let right = self.pop()?;
        let left = self.pop()?;
        let result = self.dispatcher.comparison_op(op, &left, &right)?;
        self.stack.push(result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constant_pool::Constant;
    use crate::opcodes::Opcode;

    // Captured event data for testing
    #[derive(Debug)]
    struct CaptureEvent {
        data: std::collections::HashMap<String, Value>,
    }

    // Mock dispatcher for testing
    struct MockDispatcher {
        variables: std::collections::HashMap<String, Value>,
        captures: Vec<CaptureEvent>,
    }

    impl MockDispatcher {
        fn new() -> Self {
            let mut variables = std::collections::HashMap::new();
            variables.insert("test_var".to_string(), Value::Int(42));
            variables.insert("count".to_string(), Value::Int(100));
            variables.insert("price".to_string(), Value::Float(19.99));
            variables.insert("name".to_string(), Value::String("test".to_string()));
            variables.insert("is_active".to_string(), Value::Bool(true));
            variables.insert("is_enabled".to_string(), Value::Bool(false));
            MockDispatcher {
                variables,
                captures: Vec::new(),
            }
        }

        fn take_captures(&mut self) -> Vec<CaptureEvent> {
            std::mem::take(&mut self.captures)
        }
    }

    impl Dispatcher for MockDispatcher {
        fn load_variable(&mut self, name: &str) -> Result<Value, String> {
            self.variables
                .get(name)
                .cloned_manually()
                .ok_or_else(|| format!("Unknown variable: {}", name))
        }

        fn store_variable(&mut self, name: &str, value: Value) -> Result<(), String> {
            self.variables.insert(name.to_string(), value);
            Ok(())
        }

        fn get_attribute(&mut self, _obj: &Value, attr: &str) -> Result<Value, String> {
            match attr {
                "length" => Ok(Value::Int(10)),
                "status" => Ok(Value::String("ok".to_string())),
                _ => Ok(Value::None),
            }
        }

        fn set_attribute(&mut self, _obj: &Value, _attr: &str, _value: Value) -> Result<(), String> {
            // Mock implementation - just succeed
            Ok(())
        }

        fn get_item(&mut self, _obj: &Value, _key: &Value) -> Result<Value, String> {
            Ok(Value::Int(42))
        }

        fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
            match name {
                "capture" | "send" => {
                    // Accumulate capture events
                    let mut data = std::collections::HashMap::new();

                    // Check if this is named arguments (even count, odd indices are strings)
                    let is_named = args.len() % 2 == 0
                        && args.iter().step_by(2).all(|v| matches!(v, Value::String(_)));

                    if is_named {
                        // Named arguments: args = [name1, value1, name2, value2, ...]
                        for i in (0..args.len()).step_by(2) {
                            if let Value::String(key) = &args[i] {
                                let value = match &args[i + 1] {
                                    Value::Bool(b) => Value::Bool(*b),
                                    Value::Int(n) => Value::Int(*n),
                                    Value::Float(f) => Value::Float(*f),
                                    Value::String(s) => Value::String(s.clone()),
                                    Value::None => Value::None,
                                    Value::Object(_) => Value::None,
                                };
                                data.insert(key.clone(), value);
                            }
                        }
                    } else {
                        // Positional arguments: store as "arg0", "arg1", etc.
                        for (i, arg) in args.into_iter().enumerate() {
                            let value = match arg {
                                Value::Bool(b) => Value::Bool(b),
                                Value::Int(n) => Value::Int(n),
                                Value::Float(f) => Value::Float(f),
                                Value::String(s) => Value::String(s),
                                Value::None => Value::None,
                                Value::Object(_) => Value::None,
                            };
                            data.insert(format!("arg{}", i), value);
                        }
                    }

                    self.captures.push(CaptureEvent { data });
                    Ok(Value::None)
                }
                "test_func" => Ok(Value::Int(100)),
                "add" => {
                    if args.len() != 2 {
                        return Err("add() requires 2 arguments".to_string());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                        _ => Err("add() requires int arguments".to_string()),
                    }
                }
                _ => Err(format!("Unknown function: {}", name)),
            }
        }
    }

    // Helper trait to manually clone values (since Value doesn't implement Clone)
    trait CloneManually {
        fn cloned_manually(&self) -> Option<Value>;
    }

    impl CloneManually for Option<&Value> {
        fn cloned_manually(&self) -> Option<Value> {
            self.map(|v| match v {
                Value::Bool(b) => Value::Bool(*b),
                Value::Int(i) => Value::Int(*i),
                Value::Float(f) => Value::Float(*f),
                Value::String(s) => Value::String(s.clone()),
                Value::None => Value::None,
                Value::Object(_) => Value::None,
            })
        }
    }

    #[test]
    fn test_push_const_and_add() {
        // Bytecode: PUSH_CONST 0, PUSH_CONST 1, ADD
        // Constants: [42, 8]
        // Expected result: 50

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::Int(8));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // index 0 (42)
            Opcode::PushConst as u8,
            1,
            0, // index 1 (8)
            Opcode::Add as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(50)));
    }

    #[test]
    fn test_comparison() {
        // Bytecode: PUSH_CONST 0, PUSH_CONST 1, LT
        // Constants: [10, 20]
        // Expected result: true (10 < 20)

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(10));
        pool.add(Constant::Int(20));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // index 0 (10)
            Opcode::PushConst as u8,
            1,
            0, // index 1 (20)
            Opcode::Lt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_load_var_basic() {
        // Bytecode: LOAD_VAR 0 (loads "test_var" which is 42)
        // Constants: ["test_var"]
        // Expected result: 42

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("test_var")
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(42)));
    }

    #[test]
    fn test_load_var_arithmetic() {
        // Bytecode: LOAD_VAR 0, PUSH_CONST 1, ADD
        // Constants: ["test_var", 8]
        // Expression: test_var + 8 (42 + 8 = 50)
        // Expected result: 50

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(8));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("test_var")
            Opcode::PushConst as u8,
            1,
            0, // index 1 (8)
            Opcode::Add as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(50)));
    }

    #[test]
    fn test_load_var_comparison() {
        // Bytecode: LOAD_VAR 0, PUSH_CONST 1, GT
        // Constants: ["test_var", 30]
        // Expression: test_var > 30 (42 > 30 = true)
        // Expected result: true

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(30));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("test_var")
            Opcode::PushConst as u8,
            1,
            0, // index 1 (30)
            Opcode::Gt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_load_var_multiple() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, SUB
        // Constants: ["count", "test_var"]
        // Expression: count - test_var (100 - 42 = 58)
        // Expected result: 58

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::String("test_var".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("count")
            Opcode::LoadVar as u8,
            1,
            0, // index 1 ("test_var")
            Opcode::Sub as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(58)));
    }

    #[test]
    fn test_load_var_complex_expression() {
        // Bytecode: LOAD_VAR 0, PUSH_CONST 1, MUL, LOAD_VAR 2, ADD
        // Constants: ["test_var", 2, "count"]
        // Expression: test_var * 2 + count (42 * 2 + 100 = 184)
        // Expected result: 184

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(2));
        pool.add(Constant::String("count".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("test_var")
            Opcode::PushConst as u8,
            1,
            0, // index 1 (2)
            Opcode::Mul as u8,
            Opcode::LoadVar as u8,
            2,
            0, // index 2 ("count")
            Opcode::Add as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(184)));
    }

    #[test]
    fn test_load_var_logical_and() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, AND
        // Constants: ["is_active", "is_enabled"]
        // Expression: is_active && is_enabled (true && false = false)
        // Expected result: false

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_active".to_string()));
        pool.add(Constant::String("is_enabled".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("is_active")
            Opcode::LoadVar as u8,
            1,
            0, // index 1 ("is_enabled")
            Opcode::And as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(false)));
    }

    #[test]
    fn test_load_var_logical_or() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, OR
        // Constants: ["is_active", "is_enabled"]
        // Expression: is_active || is_enabled (true || false = true)
        // Expected result: true

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_active".to_string()));
        pool.add(Constant::String("is_enabled".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("is_active")
            Opcode::LoadVar as u8,
            1,
            0, // index 1 ("is_enabled")
            Opcode::Or as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_load_var_not() {
        // Bytecode: LOAD_VAR 0, NOT
        // Constants: ["is_enabled"]
        // Expression: !is_enabled (!false = true)
        // Expected result: true

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_enabled".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("is_enabled")
            Opcode::Not as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_load_var_with_get_attr() {
        // Bytecode: LOAD_VAR 0, GET_ATTR 1
        // Constants: ["name", "length"]
        // Expression: name.length
        // Expected result: 10 (from mock dispatcher)

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("name".to_string()));
        pool.add(Constant::String("length".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("name")
            Opcode::GetAttr as u8,
            1,
            0, // index 1 ("length")
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(10)));
    }

    #[test]
    fn test_load_var_with_function_call() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, CALL_FUNC 2 (2 args)
        // Constants: ["test_var", "count", "add"]
        // Expression: add(test_var, count) = add(42, 100) = 142
        // Expected result: 142

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::String("add".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("test_var")
            Opcode::LoadVar as u8,
            1,
            0, // index 1 ("count")
            Opcode::CallFunc as u8,
            2,
            0, // index 2 ("add")
            2, // 2 arguments
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(142)));
    }

    #[test]
    fn test_store_and_load_var() {
        // Bytecode: PUSH_CONST 0, STORE_VAR 1, LOAD_VAR 1
        // Constants: [99, "new_var"]
        // Expression: new_var = 99; new_var
        // Expected result: 99

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(99));
        pool.add(Constant::String("new_var".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // index 0 (99)
            Opcode::StoreVar as u8,
            1,
            0, // index 1 ("new_var")
            Opcode::LoadVar as u8,
            1,
            0, // index 1 ("new_var")
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(99)));
    }

    #[test]
    fn test_load_var_float_arithmetic() {
        // Bytecode: LOAD_VAR 0, PUSH_CONST 1, MUL
        // Constants: ["price", 2.0]
        // Expression: price * 2.0 (19.99 * 2.0 = 39.98)
        // Expected result: 39.98

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("price".to_string()));
        pool.add(Constant::Float(2.0));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("price")
            Opcode::PushConst as u8,
            1,
            0, // index 1 (2.0)
            Opcode::Mul as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        match result {
            Value::Float(f) => assert!((f - 39.98).abs() < 0.01),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_load_var_error_unknown_variable() {
        // Bytecode: LOAD_VAR 0
        // Constants: ["unknown_var"]
        // Expected: Error

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("unknown_var".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // index 0 ("unknown_var")
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Unknown variable: unknown_var"));
    }

    // ============================================================================
    // TYPE CHECKING TESTS - Arithmetic Operations
    // ============================================================================

    #[test]
    fn test_type_error_bool_add_int() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, ADD
        // Constants: ["is_active", "test_var"]
        // Expression: is_active + test_var (bool + int)
        // Expected: Error

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_active".to_string()));
        pool.add(Constant::String("test_var".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // is_active (bool)
            Opcode::LoadVar as u8,
            1,
            0, // test_var (int)
            Opcode::Add as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot perform"));
    }

    #[test]
    fn test_type_error_string_multiply_int() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, MUL
        // Constants: ["name", "test_var"]
        // Expression: name * test_var (string * int)
        // Expected: Error (string multiplication not supported)

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("name".to_string()));
        pool.add(Constant::String("test_var".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // name (string)
            Opcode::LoadVar as u8,
            1,
            0, // test_var (int)
            Opcode::Mul as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot perform"));
    }

    #[test]
    fn test_string_concatenation() {
        // Bytecode: LOAD_VAR 0, PUSH_CONST 1, ADD
        // Constants: ["name", " world"]
        // Expression: name + " world" ("test" + " world" = "test world")
        // Expected: "test world"

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("name".to_string()));
        pool.add(Constant::String(" world".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // name ("test")
            Opcode::PushConst as u8,
            1,
            0, // " world"
            Opcode::Add as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        match result {
            Value::String(s) => assert_eq!(s, "test world"),
            _ => panic!("Expected string result"),
        }
    }

    #[test]
    fn test_int_float_promotion_add() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, ADD
        // Constants: ["test_var", "price"]
        // Expression: test_var + price (42 + 19.99 = 61.99)
        // Expected: 61.99 (promotes int to float)

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("price".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (int 42)
            Opcode::LoadVar as u8,
            1,
            0, // price (float 19.99)
            Opcode::Add as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        match result {
            Value::Float(f) => assert!((f - 61.99).abs() < 0.01),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_division_by_zero_int() {
        // Bytecode: PUSH_CONST 0, PUSH_CONST 1, DIV
        // Constants: [42, 0]
        // Expression: 42 / 0
        // Expected: Error "Division by zero"

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::Int(0));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 42
            Opcode::PushConst as u8,
            1,
            0, // 0
            Opcode::Div as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_modulo_by_zero() {
        // Bytecode: PUSH_CONST 0, PUSH_CONST 1, MOD
        // Constants: [42, 0]
        // Expression: 42 % 0
        // Expected: Error "Modulo by zero"

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::Int(0));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 42
            Opcode::PushConst as u8,
            1,
            0, // 0
            Opcode::Mod as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Modulo by zero"));
    }

    // ============================================================================
    // TYPE CHECKING TESTS - Comparison Operations
    // ============================================================================

    #[test]
    fn test_type_error_bool_less_than() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, LT
        // Constants: ["is_active", "is_enabled"]
        // Expression: is_active < is_enabled (bool < bool)
        // Expected: Error (bools can't be ordered)

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_active".to_string()));
        pool.add(Constant::String("is_enabled".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Lt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot compare bools"));
    }

    #[test]
    fn test_bool_equality() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, EQ
        // Constants: ["is_active", "is_enabled"]
        // Expression: is_active == is_enabled (true == false)
        // Expected: false

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_active".to_string()));
        pool.add(Constant::String("is_enabled".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Eq as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(false)));
    }

    #[test]
    fn test_int_float_comparison() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, GT
        // Constants: ["test_var", "price"]
        // Expression: test_var > price (42 > 19.99 = true)
        // Expected: true (promotes int to float for comparison)

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("price".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Gt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_string_comparison() {
        // Bytecode: PUSH_CONST 0, PUSH_CONST 1, LT
        // Constants: ["apple", "banana"]
        // Expression: "apple" < "banana"
        // Expected: true (lexicographic ordering)

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("apple".to_string()));
        pool.add(Constant::String("banana".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0,
            Opcode::PushConst as u8,
            1,
            0,
            Opcode::Lt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_type_error_cross_type_comparison() {
        // Bytecode: LOAD_VAR 0, LOAD_VAR 1, LT
        // Constants: ["test_var", "name"]
        // Expression: test_var < name (int < string)
        // Expected: Error

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("name".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Lt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot compare"));
    }

    // ============================================================================
    // OPERATOR PRECEDENCE TESTS
    // ============================================================================

    #[test]
    fn test_operator_precedence_mul_before_add() {
        // Bytecode for: 10 + 5 * 2 (should be 10 + (5 * 2) = 20)
        // Manual bytecode: PUSH 10, PUSH 5, PUSH 2, MUL, ADD
        // Constants: [10, 5, 2]

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(10));
        pool.add(Constant::Int(5));
        pool.add(Constant::Int(2));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 10
            Opcode::PushConst as u8,
            1,
            0, // 5
            Opcode::PushConst as u8,
            2,
            0, // 2
            Opcode::Mul as u8, // 5 * 2 = 10
            Opcode::Add as u8, // 10 + 10 = 20
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(20)));
    }

    #[test]
    fn test_operator_precedence_complex() {
        // Bytecode for: 2 * 3 + 4 * 5 (should be (2 * 3) + (4 * 5) = 6 + 20 = 26)
        // Manual bytecode: PUSH 2, PUSH 3, MUL, PUSH 4, PUSH 5, MUL, ADD

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(2));
        pool.add(Constant::Int(3));
        pool.add(Constant::Int(4));
        pool.add(Constant::Int(5));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 2
            Opcode::PushConst as u8,
            1,
            0, // 3
            Opcode::Mul as u8, // 2 * 3 = 6
            Opcode::PushConst as u8,
            2,
            0, // 4
            Opcode::PushConst as u8,
            3,
            0, // 5
            Opcode::Mul as u8, // 4 * 5 = 20
            Opcode::Add as u8, // 6 + 20 = 26
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(26)));
    }

    #[test]
    fn test_operator_precedence_with_variables() {
        // Bytecode for: count + test_var * 2 (should be 100 + (42 * 2) = 184)
        // Already tested in test_load_var_complex_expression but here explicitly for precedence

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(2));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // count (100)
            Opcode::LoadVar as u8,
            1,
            0, // test_var (42)
            Opcode::PushConst as u8,
            2,
            0, // 2
            Opcode::Mul as u8, // 42 * 2 = 84
            Opcode::Add as u8, // 100 + 84 = 184
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(184)));
    }

    #[test]
    fn test_comparison_after_arithmetic() {
        // Bytecode for: (test_var * 2) > count (should be (42 * 2) > 100 = false)
        // Constants: ["test_var", 2, "count"]

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(2));
        pool.add(Constant::String("count".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::PushConst as u8,
            1,
            0, // 2
            Opcode::Mul as u8, // 42 * 2 = 84
            Opcode::LoadVar as u8,
            2,
            0, // count (100)
            Opcode::Gt as u8, // 84 > 100 = false
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(false)));
    }

    // ============================================================================
    // PREDICATE CONTEXT TESTS
    // ============================================================================

    #[test]
    fn test_predicate_simple_comparison() {
        // Simulates a predicate: test_var > 30
        // This would be the bytecode for the predicate of a probe

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(30));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::PushConst as u8,
            1,
            0,
            Opcode::Gt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        // Predicate should evaluate to true (42 > 30)
        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_predicate_complex_condition() {
        // Simulates: (test_var > 30) && (count < 200)
        // Bytecode: LOAD test_var, PUSH 30, GT, LOAD count, PUSH 200, LT, AND

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(30));
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::Int(200));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var
            Opcode::PushConst as u8,
            1,
            0, // 30
            Opcode::Gt as u8, // test_var > 30 (true)
            Opcode::LoadVar as u8,
            2,
            0, // count
            Opcode::PushConst as u8,
            3,
            0, // 200
            Opcode::Lt as u8, // count < 200 (true)
            Opcode::And as u8, // true && true = true
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_predicate_with_negation() {
        // Simulates: !(is_enabled) && is_active
        // Bytecode: LOAD is_enabled, NOT, LOAD is_active, AND

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_enabled".to_string()));
        pool.add(Constant::String("is_active".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // is_enabled (false)
            Opcode::Not as u8, // !false = true
            Opcode::LoadVar as u8,
            1,
            0, // is_active (true)
            Opcode::And as u8, // true && true = true
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_predicate_arithmetic_in_comparison() {
        // Simulates: (test_var + 10) >= count
        // (42 + 10) >= 100 = 52 >= 100 = false

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(10));
        pool.add(Constant::String("count".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::PushConst as u8,
            1,
            0, // 10
            Opcode::Add as u8, // 42 + 10 = 52
            Opcode::LoadVar as u8,
            2,
            0, // count (100)
            Opcode::Ge as u8, // 52 >= 100 = false
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(false)));
    }

    // ============================================================================
    // BODY CONTEXT TESTS (Action Bodies with Multiple Statements)
    // ============================================================================

    #[test]
    fn test_body_variable_assignment_and_use() {
        // Simulates action body: result = test_var * 2; result + 10
        // Bytecode: LOAD test_var, PUSH 2, MUL, STORE result, LOAD result, PUSH 10, ADD

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(2));
        pool.add(Constant::String("result".to_string()));
        pool.add(Constant::Int(10));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::PushConst as u8,
            1,
            0, // 2
            Opcode::Mul as u8, // 42 * 2 = 84
            Opcode::StoreVar as u8,
            2,
            0, // store to "result"
            Opcode::LoadVar as u8,
            2,
            0, // load "result"
            Opcode::PushConst as u8,
            3,
            0, // 10
            Opcode::Add as u8, // 84 + 10 = 94
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(94)));
    }

    #[test]
    fn test_body_multiple_operations_sequence() {
        // Simulates: temp1 = count / 2; temp2 = test_var + temp1; temp2
        // count / 2 = 100 / 2 = 50
        // test_var + temp1 = 42 + 50 = 92

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::Int(2));
        pool.add(Constant::String("temp1".to_string()));
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("temp2".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // count (100)
            Opcode::PushConst as u8,
            1,
            0, // 2
            Opcode::Div as u8, // 100 / 2 = 50
            Opcode::StoreVar as u8,
            2,
            0, // store to temp1
            Opcode::LoadVar as u8,
            3,
            0, // test_var (42)
            Opcode::LoadVar as u8,
            2,
            0, // temp1 (50)
            Opcode::Add as u8, // 42 + 50 = 92
            Opcode::StoreVar as u8,
            4,
            0, // store to temp2
            Opcode::LoadVar as u8,
            4,
            0, // load temp2
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(92)));
    }

    #[test]
    fn test_body_accumulator_pattern() {
        // Simulates: sum = 0; sum = sum + test_var; sum = sum + count; sum
        // sum = 0 + 42 + 100 = 142

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(0));
        pool.add(Constant::String("sum".to_string()));
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("count".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 0
            Opcode::StoreVar as u8,
            1,
            0, // sum = 0
            Opcode::LoadVar as u8,
            1,
            0, // sum
            Opcode::LoadVar as u8,
            2,
            0, // test_var
            Opcode::Add as u8, // sum + test_var
            Opcode::StoreVar as u8,
            1,
            0, // sum = sum + test_var (42)
            Opcode::LoadVar as u8,
            1,
            0, // sum
            Opcode::LoadVar as u8,
            3,
            0, // count
            Opcode::Add as u8, // sum + count
            Opcode::StoreVar as u8,
            1,
            0, // sum = sum + count (142)
            Opcode::LoadVar as u8,
            1,
            0, // load final sum
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(142)));
    }

    // ============================================================================
    // MIXED PREDICATE + BODY SCENARIOS
    // ============================================================================

    #[test]
    fn test_predicate_passes_body_executes() {
        // Simulate: if (test_var > 30) { test_var + count }
        // This is split into two bytecode sequences in practice,
        // but we can test the body assuming predicate passed

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("count".to_string()));

        // Body bytecode (would only execute if predicate is true)
        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::LoadVar as u8,
            1,
            0, // count (100)
            Opcode::Add as u8, // 42 + 100 = 142
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(142)));
    }

    // ============================================================================
    // ADDITIONAL TYPE CHECKING EDGE CASES
    // ============================================================================

    #[test]
    fn test_type_error_bool_subtract() {
        // bool - bool should error
        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_active".to_string()));
        pool.add(Constant::String("is_enabled".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Sub as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
    }

    #[test]
    fn test_type_error_string_subtract() {
        // string - string should error (only + is supported)
        let mut pool = ConstantPool::new();
        pool.add(Constant::String("hello".to_string()));
        pool.add(Constant::String("world".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0,
            Opcode::PushConst as u8,
            1,
            0,
            Opcode::Sub as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot perform"));
    }

    #[test]
    fn test_type_mixed_int_string_add() {
        // int + string should error
        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("name".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (int)
            Opcode::LoadVar as u8,
            1,
            0, // name (string)
            Opcode::Add as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot perform"));
    }

    #[test]
    fn test_all_arithmetic_ops_on_ints() {
        // Test all arithmetic operations: (test_var + 8) * 2 - 10 / 2
        // (42 + 8) * 2 - 10 / 2 = 50 * 2 - 5 = 100 - 5 = 95

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(8));
        pool.add(Constant::Int(2));
        pool.add(Constant::Int(10));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::PushConst as u8,
            1,
            0, // 8
            Opcode::Add as u8, // 42 + 8 = 50
            Opcode::PushConst as u8,
            2,
            0, // 2
            Opcode::Mul as u8, // 50 * 2 = 100
            Opcode::PushConst as u8,
            3,
            0, // 10
            Opcode::PushConst as u8,
            2,
            0, // 2
            Opcode::Div as u8, // 10 / 2 = 5
            Opcode::Sub as u8, // 100 - 5 = 95
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Int(95)));
    }

    #[test]
    fn test_all_comparison_ops() {
        // Test all six comparison operators with variables
        // We'll test them individually to ensure they all work

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string())); // 42
        pool.add(Constant::String("count".to_string())); // 100

        // Test LT: 42 < 100 = true
        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Lt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        assert!(matches!(
            executor.execute(&bytecode).unwrap(),
            Value::Bool(true)
        ));

        // Test LE: 42 <= 100 = true
        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Le as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        assert!(matches!(
            executor.execute(&bytecode).unwrap(),
            Value::Bool(true)
        ));

        // Test GT: 42 > 100 = false
        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Gt as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        assert!(matches!(
            executor.execute(&bytecode).unwrap(),
            Value::Bool(false)
        ));

        // Test GE: 42 >= 100 = false
        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Ge as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        assert!(matches!(
            executor.execute(&bytecode).unwrap(),
            Value::Bool(false)
        ));

        // Test EQ: 42 == 100 = false
        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Eq as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        assert!(matches!(
            executor.execute(&bytecode).unwrap(),
            Value::Bool(false)
        ));

        // Test NE: 42 != 100 = true
        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0,
            Opcode::LoadVar as u8,
            1,
            0,
            Opcode::Ne as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        assert!(matches!(
            executor.execute(&bytecode).unwrap(),
            Value::Bool(true)
        ));
    }

    #[test]
    fn test_complex_boolean_expression() {
        // Test: (test_var > 30) && (count < 200) || is_enabled
        // (42 > 30) && (100 < 200) || false
        // true && true || false = true

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(30));
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::Int(200));
        pool.add(Constant::String("is_enabled".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var
            Opcode::PushConst as u8,
            1,
            0, // 30
            Opcode::Gt as u8, // test_var > 30 = true
            Opcode::LoadVar as u8,
            2,
            0, // count
            Opcode::PushConst as u8,
            3,
            0, // 200
            Opcode::Lt as u8, // count < 200 = true
            Opcode::And as u8, // true && true = true
            Opcode::LoadVar as u8,
            4,
            0, // is_enabled (false)
            Opcode::Or as u8, // true || false = true
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_division_by_zero_float() {
        // Test float division by zero
        let mut pool = ConstantPool::new();
        pool.add(Constant::Float(42.5));
        pool.add(Constant::Float(0.0));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0,
            Opcode::PushConst as u8,
            1,
            0,
            Opcode::Div as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_modulo_with_floats() {
        // Test modulo with floats: 10.5 % 3.0 = 1.5
        let mut pool = ConstantPool::new();
        pool.add(Constant::Float(10.5));
        pool.add(Constant::Float(3.0));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0,
            Opcode::PushConst as u8,
            1,
            0,
            Opcode::Mod as u8,
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        match result {
            Value::Float(f) => assert!((f - 1.5).abs() < 0.01),
            _ => panic!("Expected float result"),
        }
    }

    #[test]
    fn test_nested_arithmetic_and_comparison() {
        // Test: ((test_var * 2) + (count / 10)) > 100
        // ((42 * 2) + (100 / 10)) > 100
        // (84 + 10) > 100 = 94 > 100 = false

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(2));
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::Int(10));
        pool.add(Constant::Int(100));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::PushConst as u8,
            1,
            0, // 2
            Opcode::Mul as u8, // 42 * 2 = 84
            Opcode::LoadVar as u8,
            2,
            0, // count (100)
            Opcode::PushConst as u8,
            3,
            0, // 10
            Opcode::Div as u8, // 100 / 10 = 10
            Opcode::Add as u8, // 84 + 10 = 94
            Opcode::PushConst as u8,
            4,
            0, // 100
            Opcode::Gt as u8, // 94 > 100 = false
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(false)));
    }

    // ============================================================================
    // CAPTURE TESTS - Positional Arguments
    // ============================================================================

    #[test]
    fn test_capture_positional_single_constant() {
        // Bytecode: PUSH_CONST 0, CALL_FUNC capture (1 arg)
        // Expression: capture(42)
        // Expected capture: {"arg0": 42}

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 42
            Opcode::CallFunc as u8,
            1,
            0, // "capture"
            1, // 1 argument
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("arg0"), Some(Value::Int(42))));
    }

    #[test]
    fn test_capture_positional_multiple_constants() {
        // Bytecode: PUSH 42, PUSH "hello", PUSH true, CALL_FUNC capture (3 args)
        // Expression: capture(42, "hello", true)
        // Expected capture: {"arg0": 42, "arg1": "hello", "arg2": true}

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::String("hello".to_string()));
        pool.add(Constant::Bool(true));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 42
            Opcode::PushConst as u8,
            1,
            0, // "hello"
            Opcode::PushConst as u8,
            2,
            0, // true
            Opcode::CallFunc as u8,
            3,
            0, // "capture"
            3, // 3 arguments
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("arg0"), Some(Value::Int(42))));
        assert!(matches!(
            captures[0].data.get("arg1"),
            Some(Value::String(s)) if s == "hello"
        ));
        assert!(matches!(captures[0].data.get("arg2"), Some(Value::Bool(true))));
    }

    #[test]
    fn test_capture_positional_with_variable() {
        // Bytecode: LOAD_VAR test_var, CALL_FUNC capture (1 arg)
        // Expression: capture(test_var)
        // Expected capture: {"arg0": 42}

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::CallFunc as u8,
            1,
            0, // "capture"
            1, // 1 argument
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("arg0"), Some(Value::Int(42))));
    }

    #[test]
    fn test_capture_positional_with_expression() {
        // Bytecode: LOAD test_var, PUSH 10, ADD, CALL_FUNC capture (1 arg)
        // Expression: capture(test_var + 10)
        // Expected capture: {"arg0": 52} (42 + 10)

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(10));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::PushConst as u8,
            1,
            0, // 10
            Opcode::Add as u8, // 42 + 10 = 52
            Opcode::CallFunc as u8,
            2,
            0, // "capture"
            1, // 1 argument (the result of the expression)
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("arg0"), Some(Value::Int(52))));
    }

    #[test]
    fn test_capture_positional_multiple_variables() {
        // Expression: capture(test_var, count, name)
        // Expected capture: {"arg0": 42, "arg1": 100, "arg2": "test"}

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::String("name".to_string()));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::LoadVar as u8,
            0,
            0, // test_var
            Opcode::LoadVar as u8,
            1,
            0, // count
            Opcode::LoadVar as u8,
            2,
            0, // name
            Opcode::CallFunc as u8,
            3,
            0, // "capture"
            3, // 3 arguments
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("arg0"), Some(Value::Int(42))));
        assert!(matches!(captures[0].data.get("arg1"), Some(Value::Int(100))));
        assert!(matches!(
            captures[0].data.get("arg2"),
            Some(Value::String(s)) if s == "test"
        ));
    }

    // ============================================================================
    // CAPTURE TESTS - Named Arguments
    // ============================================================================

    #[test]
    fn test_capture_named_single() {
        // Bytecode: PUSH "user_id", PUSH 42, CALL_FUNC capture (2 args - name/value pair)
        // Expression: capture(user_id=42)
        // Expected capture: {"user_id": 42}

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("user_id".to_string()));
        pool.add(Constant::Int(42));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // "user_id"
            Opcode::PushConst as u8,
            1,
            0, // 42
            Opcode::CallFunc as u8,
            2,
            0, // "capture"
            2, // 2 arguments (1 name-value pair)
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("user_id"), Some(Value::Int(42))));
        assert!(!captures[0].data.contains_key("arg0"));
    }

    #[test]
    fn test_capture_named_multiple() {
        // Expression: capture(user_id=42, event="login", success=true)
        // Expected capture: {"user_id": 42, "event": "login", "success": true}

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("user_id".to_string()));
        pool.add(Constant::Int(42));
        pool.add(Constant::String("event".to_string()));
        pool.add(Constant::String("login".to_string()));
        pool.add(Constant::String("success".to_string()));
        pool.add(Constant::Bool(true));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // "user_id"
            Opcode::PushConst as u8,
            1,
            0, // 42
            Opcode::PushConst as u8,
            2,
            0, // "event"
            Opcode::PushConst as u8,
            3,
            0, // "login"
            Opcode::PushConst as u8,
            4,
            0, // "success"
            Opcode::PushConst as u8,
            5,
            0, // true
            Opcode::CallFunc as u8,
            6,
            0, // "capture"
            6, // 6 arguments (3 name-value pairs)
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("user_id"), Some(Value::Int(42))));
        assert!(matches!(
            captures[0].data.get("event"),
            Some(Value::String(s)) if s == "login"
        ));
        assert!(matches!(
            captures[0].data.get("success"),
            Some(Value::Bool(true))
        ));
    }

    #[test]
    fn test_capture_named_with_variables() {
        // Expression: capture(user=test_var, total=count)
        // Expected capture: {"user": 42, "total": 100}

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("user".to_string()));
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("total".to_string()));
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // "user"
            Opcode::LoadVar as u8,
            1,
            0, // test_var (42)
            Opcode::PushConst as u8,
            2,
            0, // "total"
            Opcode::LoadVar as u8,
            3,
            0, // count (100)
            Opcode::CallFunc as u8,
            4,
            0, // "capture"
            4, // 4 arguments (2 name-value pairs)
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("user"), Some(Value::Int(42))));
        assert!(matches!(captures[0].data.get("total"), Some(Value::Int(100))));
    }

    #[test]
    fn test_capture_named_with_expressions() {
        // Expression: capture(sum=test_var + count, double=test_var * 2)
        // Expected capture: {"sum": 142, "double": 84}

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("sum".to_string()));
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("count".to_string()));
        pool.add(Constant::String("double".to_string()));
        pool.add(Constant::Int(2));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // "sum"
            Opcode::LoadVar as u8,
            1,
            0, // test_var (42)
            Opcode::LoadVar as u8,
            2,
            0, // count (100)
            Opcode::Add as u8, // 42 + 100 = 142
            Opcode::PushConst as u8,
            3,
            0, // "double"
            Opcode::LoadVar as u8,
            1,
            0, // test_var (42)
            Opcode::PushConst as u8,
            4,
            0, // 2
            Opcode::Mul as u8, // 42 * 2 = 84
            Opcode::CallFunc as u8,
            5,
            0, // "capture"
            4, // 4 arguments (2 name-value pairs)
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("sum"), Some(Value::Int(142))));
        assert!(matches!(captures[0].data.get("double"), Some(Value::Int(84))));
    }

    // ============================================================================
    // CAPTURE TESTS - Multiple Captures in Sequence
    // ============================================================================

    #[test]
    fn test_multiple_captures_in_sequence() {
        // Execute two captures in sequence
        // capture(42); capture(100);
        // Expected: 2 capture events

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::FunctionName("capture".to_string()));
        pool.add(Constant::Int(100));

        let bytecode = vec![
            // First capture(42)
            Opcode::PushConst as u8,
            0,
            0, // 42
            Opcode::CallFunc as u8,
            1,
            0, // "capture"
            1, // 1 argument
            Opcode::Pop as u8, // Pop the None result
            // Second capture(100)
            Opcode::PushConst as u8,
            2,
            0, // 100
            Opcode::CallFunc as u8,
            1,
            0, // "capture"
            1, // 1 argument
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 2);
        assert!(matches!(captures[0].data.get("arg0"), Some(Value::Int(42))));
        assert!(matches!(captures[1].data.get("arg0"), Some(Value::Int(100))));
    }

    #[test]
    fn test_capture_with_variables_changing() {
        // temp = test_var; capture(value=temp); temp = count; capture(value=temp);
        // Expected: 2 captures with different values

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::String("temp".to_string()));
        pool.add(Constant::String("value".to_string()));
        pool.add(Constant::FunctionName("capture".to_string()));
        pool.add(Constant::String("count".to_string()));

        let bytecode = vec![
            // temp = test_var
            Opcode::LoadVar as u8,
            0,
            0, // test_var (42)
            Opcode::StoreVar as u8,
            1,
            0, // temp
            // capture(value=temp)
            Opcode::PushConst as u8,
            2,
            0, // "value"
            Opcode::LoadVar as u8,
            1,
            0, // temp (42)
            Opcode::CallFunc as u8,
            3,
            0, // "capture"
            2, // 2 arguments
            Opcode::Pop as u8, // Pop None
            // temp = count
            Opcode::LoadVar as u8,
            4,
            0, // count (100)
            Opcode::StoreVar as u8,
            1,
            0, // temp
            // capture(value=temp)
            Opcode::PushConst as u8,
            2,
            0, // "value"
            Opcode::LoadVar as u8,
            1,
            0, // temp (100)
            Opcode::CallFunc as u8,
            3,
            0, // "capture"
            2, // 2 arguments
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 2);
        assert!(matches!(captures[0].data.get("value"), Some(Value::Int(42))));
        assert!(matches!(captures[1].data.get("value"), Some(Value::Int(100))));
    }

    #[test]
    fn test_capture_empty_positional() {
        // Expression: capture()
        // Expected capture: empty data

        let mut pool = ConstantPool::new();
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::CallFunc as u8,
            0,
            0, // "capture"
            0, // 0 arguments
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].data.len(), 0);
    }

    #[test]
    fn test_capture_with_comparison_result() {
        // Expression: capture(is_large=test_var > 30)
        // Expected capture: {"is_large": true}

        let mut pool = ConstantPool::new();
        pool.add(Constant::String("is_large".to_string()));
        pool.add(Constant::String("test_var".to_string()));
        pool.add(Constant::Int(30));
        pool.add(Constant::FunctionName("capture".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // "is_large"
            Opcode::LoadVar as u8,
            1,
            0, // test_var (42)
            Opcode::PushConst as u8,
            2,
            0, // 30
            Opcode::Gt as u8, // 42 > 30 = true
            Opcode::CallFunc as u8,
            3,
            0, // "capture"
            2, // 2 arguments
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(
            captures[0].data.get("is_large"),
            Some(Value::Bool(true))
        ));
    }

    #[test]
    fn test_send_alias_works() {
        // Test that send() is an alias for capture()
        // Expression: send(42)

        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::FunctionName("send".to_string()));

        let bytecode = vec![
            Opcode::PushConst as u8,
            0,
            0, // 42
            Opcode::CallFunc as u8,
            1,
            0, // "send"
            1, // 1 argument
        ];

        let mut dispatcher = MockDispatcher::new();
        let mut executor = Executor::new(&pool, &mut dispatcher);
        executor.execute(&bytecode).unwrap();

        let captures = dispatcher.take_captures();
        assert_eq!(captures.len(), 1);
        assert!(matches!(captures[0].data.get("arg0"), Some(Value::Int(42))));
    }
}
