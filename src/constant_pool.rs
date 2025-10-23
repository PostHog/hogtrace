use crate::value::Value;

/// A constant in the constant pool
///
/// Constants can be literals (int, float, string, bool, none) or identifiers/names
/// used for variable names, field names, and function names.
#[derive(Debug, Clone)]
pub enum Constant {
    /// Integer literal
    Int(i64),

    /// Float literal
    Float(f64),

    /// String literal
    String(String),

    /// Boolean literal
    Bool(bool),

    /// None/null literal
    None,

    /// Variable identifier (e.g., "args", "arg0", "retval", "exception")
    Identifier(String),

    /// Field/attribute name (e.g., "user", "email", "status")
    FieldName(String),

    /// Function name (e.g., "timestamp", "rand", "capture", "len")
    FunctionName(String),
}

impl Constant {
    /// Try to convert this constant to a Value (only works for literals)
    pub fn as_value(&self) -> Result<Value, String> {
        match self {
            Constant::Int(i) => Ok(Value::Int(*i)),
            Constant::Float(f) => Ok(Value::Float(*f)),
            Constant::String(s) => Ok(Value::String(s.clone())),
            Constant::Bool(b) => Ok(Value::Bool(*b)),
            Constant::None => Ok(Value::None),
            _ => Err(format!(
                "Cannot convert {:?} to Value (not a literal)",
                self
            )),
        }
    }

    /// Get the string value if this is an identifier, field name, or function name
    pub fn as_string(&self) -> Result<&str, String> {
        match self {
            Constant::Identifier(s) | Constant::FieldName(s) | Constant::FunctionName(s) => Ok(s),
            Constant::String(s) => Ok(s),
            _ => Err(format!("Constant {:?} is not a string", self)),
        }
    }

    /// Check if this is an identifier
    pub fn is_identifier(&self) -> bool {
        matches!(self, Constant::Identifier(_))
    }

    /// Check if this is a field name
    pub fn is_field_name(&self) -> bool {
        matches!(self, Constant::FieldName(_))
    }

    /// Check if this is a function name
    pub fn is_function_name(&self) -> bool {
        matches!(self, Constant::FunctionName(_))
    }
}

/// Constant pool holding all constants for a program
///
/// Constants are accessed by index (u16) from bytecode instructions.
#[derive(Debug, Clone)]
pub struct ConstantPool {
    constants: Vec<Constant>,
}

impl ConstantPool {
    /// Create a new empty constant pool
    pub fn new() -> Self {
        ConstantPool {
            constants: Vec::new(),
        }
    }

    /// Create a constant pool from a vector of constants
    pub fn from_vec(constants: Vec<Constant>) -> Self {
        ConstantPool { constants }
    }

    /// Add a constant to the pool and return its index
    pub fn add(&mut self, constant: Constant) -> u16 {
        let index = self.constants.len();
        if index >= u16::MAX as usize {
            panic!("Constant pool overflow: too many constants");
        }
        self.constants.push(constant);
        index as u16
    }

    /// Get a constant by index
    pub fn get(&self, index: u16) -> Result<&Constant, String> {
        self.constants
            .get(index as usize)
            .ok_or_else(|| format!("Constant pool index {} out of bounds", index))
    }

    /// Get the number of constants in the pool
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }

    /// Get a constant as a Value (only for literals)
    pub fn get_value(&self, index: u16) -> Result<Value, String> {
        let constant = self.get(index)?;
        constant.as_value()
    }

    /// Get a constant as a string (for identifiers, field names, function names)
    pub fn get_string(&self, index: u16) -> Result<&str, String> {
        let constant = self.get(index)?;
        constant.as_string()
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_pool_basic() {
        let mut pool = ConstantPool::new();

        let idx1 = pool.add(Constant::Int(42));
        let idx2 = pool.add(Constant::String("hello".to_string()));
        let idx3 = pool.add(Constant::Identifier("args".to_string()));

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 2);
        assert_eq!(pool.len(), 3);

        let v1 = pool.get_value(idx1).unwrap();
        assert!(matches!(v1, Value::Int(42)));

        let s = pool.get_string(idx3).unwrap();
        assert_eq!(s, "args");
    }

    #[test]
    fn test_constant_as_value() {
        let c = Constant::Int(42);
        let v = c.as_value().unwrap();
        assert!(matches!(v, Value::Int(42)));

        let c = Constant::Identifier("test".to_string());
        assert!(c.as_value().is_err());
    }

    #[test]
    fn test_constant_type_checks() {
        let id = Constant::Identifier("test".to_string());
        let field = Constant::FieldName("name".to_string());
        let func = Constant::FunctionName("timestamp".to_string());

        assert!(id.is_identifier());
        assert!(!id.is_field_name());
        assert!(!id.is_function_name());

        assert!(field.is_field_name());
        assert!(func.is_function_name());
    }
}
