use std::any::Any;
use std::fmt;

/// Runtime value in the HogTrace VM
///
/// Values can be primitives (int, float, string, bool, none) or language-specific objects.
/// Language-specific objects are stored as boxed trait objects and accessed via the dispatcher.
pub enum Value {
    /// Boolean value
    Bool(bool),

    /// 64-bit signed integer
    Int(i64),

    /// 64-bit floating point number
    Float(f64),

    /// UTF-8 string
    String(String),

    /// None/null value
    None,

    /// Language-specific object (e.g., Python PyObject, JavaScript value, etc.)
    /// Stored as Box<dyn Any> for maximum flexibility.
    /// The dispatcher is responsible for downcasting and interpreting these objects.
    Object(Box<dyn Any + Send>),
}

impl Value {
    /// Check if this value is truthy (for boolean coercion in predicates)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::None => false,
            Value::Object(_) => true, // Objects are truthy by default
        }
    }

    /// Try to extract an integer from this value
    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Int(i) => Ok(*i),
            Value::Float(f) => Ok(*f as i64),
            Value::Bool(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err(format!("Cannot convert {:?} to int", self)),
        }
    }

    /// Try to extract a float from this value
    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as f64),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err(format!("Cannot convert {:?} to float", self)),
        }
    }

    /// Try to extract a string from this value
    pub fn as_string(&self) -> Result<&str, String> {
        match self {
            Value::String(s) => Ok(s),
            _ => Err(format!("Cannot convert {:?} to string", self)),
        }
    }

    /// Try to extract a boolean from this value
    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(format!("Cannot convert {:?} to bool", self)),
        }
    }

    /// Get the type name of this value for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::None => "none",
            Value::Object(_) => "object",
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "Bool({})", b),
            Value::Int(i) => write!(f, "Int({})", i),
            Value::Float(fl) => write!(f, "Float({})", fl),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::None => write!(f, "None"),
            Value::Object(_) => write!(f, "Object(<opaque>)"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::None => write!(f, "None"),
            Value::Object(_) => write!(f, "<object>"),
        }
    }
}

// Implement From for easy construction
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

/// Value does not implement `Clone` for the following reasons:
/// 1. The `Object` variant contains `Box<dyn Any + Send>`, which does not implement `Clone`
/// 2. Python objects (`Py<PyAny>`) require a Python GIL token to clone via `clone_ref()`
/// 3. Explicit copy methods are provided where needed to make ownership semantics clear
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Float(1.5).is_truthy());
        assert!(!Value::Float(0.0).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(!Value::None.is_truthy());
    }

    #[test]
    fn test_conversions() {
        let v = Value::Int(42);
        assert_eq!(v.as_int().unwrap(), 42);
        assert_eq!(v.as_float().unwrap(), 42.0);

        let v = Value::String("hello".to_string());
        assert_eq!(v.as_string().unwrap(), "hello");
    }
}
