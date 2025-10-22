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

    // Mock dispatcher for testing
    struct MockDispatcher;

    impl Dispatcher for MockDispatcher {
        fn load_variable(&mut self, name: &str) -> Result<Value, String> {
            match name {
                "test_var" => Ok(Value::Int(42)),
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

        fn call_function(&mut self, name: &str, _args: Vec<Value>) -> Result<Value, String> {
            match name {
                "test_func" => Ok(Value::Int(100)),
                _ => Err(format!("Unknown function: {}", name)),
            }
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

        let mut dispatcher = MockDispatcher;
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

        let mut dispatcher = MockDispatcher;
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }
}
