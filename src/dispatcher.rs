use crate::value::Value;

/// Dispatcher trait for language-specific operations
///
/// This trait abstracts all language-specific behavior, allowing the VM core
/// to remain generic. Implementations of this trait handle:
/// - Variable access (args, arg0, retval, exception, kwargs, self, etc.)
/// - Variable assignment (request-scoped variables like $req.user_id)
/// - Function calls (timestamp, rand, capture, len, etc.)
/// - Attribute/field access (obj.field)
/// - Item access (obj[key])
///
/// The VM executor takes a reference to a Dispatcher and delegates all
/// language-specific operations to it.
pub trait Dispatcher {
    /// Load a variable by name
    ///
    /// Variables include:
    /// - Entry probes: args, arg0, arg1, ..., kwargs, self, $req.*
    /// - Exit probes: retval, exception (plus all entry variables)
    /// - Special: locals, globals
    ///
    /// Returns Err if the variable doesn't exist or can't be accessed.
    fn load_variable(&mut self, name: &str) -> Result<Value, String>;

    /// Store a value to a variable (typically request-scoped like $req.user_id)
    ///
    /// The name will be the full variable name, e.g., "$req.user_id" or "$request.start_time"
    fn store_variable(&mut self, name: &str, value: Value) -> Result<(), String>;

    /// Get an attribute from an object: obj.field
    ///
    /// Example: arg0.user.email
    /// - obj: the object (could be a language-specific Object value)
    /// - attr: the attribute name
    fn get_attribute(&mut self, obj: &Value, attr: &str) -> Result<Value, String>;

    /// Set an attribute on an object: obj.field = value
    ///
    /// Example: $req.user_id = 123
    /// - obj: the object (typically the request store proxy for $req.*)
    /// - attr: the attribute name
    /// - value: the value to set
    fn set_attribute(&mut self, obj: &Value, attr: &str, value: Value) -> Result<(), String>;

    /// Get an item from an object: obj[key]
    ///
    /// Example: args[0], retval["status"], data[idx]
    /// - obj: the object (could be list, dict, etc.)
    /// - key: the key (could be int for list index, string for dict key)
    fn get_item(&mut self, obj: &Value, key: &Value) -> Result<Value, String>;

    /// Call a function with arguments
    ///
    /// Built-in functions include:
    /// - timestamp() -> float (current Unix timestamp)
    /// - rand() -> float (random 0.0-1.0)
    /// - len(obj) -> int (length of object)
    /// - str(obj), int(obj), float(obj) (type conversions)
    /// - capture(...) / send(...) (send data to PostHog)
    ///
    /// Language-specific implementations can also support additional functions.
    ///
    /// Arguments are passed in order: [arg0, arg1, arg2, ...]
    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String>;

    /// Perform a binary arithmetic operation
    ///
    /// This is provided as a hook to allow language-specific numeric coercion.
    /// The default implementation performs Rust-native arithmetic on primitives.
    ///
    /// Operations: Add, Sub, Mul, Div, Mod
    fn binary_op(&mut self, op: BinaryOp, left: &Value, right: &Value) -> Result<Value, String> {
        binary_op_default(op, left, right)
    }

    /// Perform a comparison operation
    ///
    /// This is provided as a hook to allow language-specific comparison semantics.
    /// The default implementation performs Rust-native comparisons on primitives.
    ///
    /// Operations: Eq, Ne, Lt, Gt, Le, Ge
    fn comparison_op(
        &mut self,
        op: ComparisonOp,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        comparison_op_default(op, left, right)
    }
}

/// Binary arithmetic operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

/// Comparison operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

/// Default implementation for binary operations (used by default in trait)
pub fn binary_op_default(op: BinaryOp, left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        // Integer operations
        (Value::Int(l), Value::Int(r)) => match op {
            BinaryOp::Add => Ok(Value::Int(l + r)),
            BinaryOp::Sub => Ok(Value::Int(l - r)),
            BinaryOp::Mul => Ok(Value::Int(l * r)),
            BinaryOp::Div => {
                if *r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Int(l / r))
                }
            }
            BinaryOp::Mod => {
                if *r == 0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Value::Int(l % r))
                }
            }
        },

        // Float operations (promote int to float if needed)
        (Value::Float(_), Value::Float(_))
        | (Value::Float(_), Value::Int(_))
        | (Value::Int(_), Value::Float(_)) => {
            let lf = match left {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => unreachable!(),
            };
            let rf = match right {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => unreachable!(),
            };

            match op {
                BinaryOp::Add => Ok(Value::Float(lf + rf)),
                BinaryOp::Sub => Ok(Value::Float(lf - rf)),
                BinaryOp::Mul => Ok(Value::Float(lf * rf)),
                BinaryOp::Div => {
                    if rf == 0.0 {
                        Err("Division by zero".to_string())
                    } else {
                        Ok(Value::Float(lf / rf))
                    }
                }
                BinaryOp::Mod => Ok(Value::Float(lf % rf)),
            }
        }

        // String concatenation for Add
        (Value::String(l), Value::String(r)) if matches!(op, BinaryOp::Add) => {
            Ok(Value::String(format!("{}{}", l, r)))
        }

        _ => Err(format!(
            "Cannot perform {:?} on {} and {}",
            op,
            left.type_name(),
            right.type_name()
        )),
    }
}

/// Default implementation for comparison operations
pub fn comparison_op_default(
    op: ComparisonOp,
    left: &Value,
    right: &Value,
) -> Result<Value, String> {
    let result = match (left, right) {
        // Bool comparisons
        (Value::Bool(l), Value::Bool(r)) => match op {
            ComparisonOp::Eq => l == r,
            ComparisonOp::Ne => l != r,
            _ => return Err(format!("Cannot compare bools with {:?}", op)),
        },

        // Integer comparisons
        (Value::Int(l), Value::Int(r)) => match op {
            ComparisonOp::Eq => l == r,
            ComparisonOp::Ne => l != r,
            ComparisonOp::Lt => l < r,
            ComparisonOp::Gt => l > r,
            ComparisonOp::Le => l <= r,
            ComparisonOp::Ge => l >= r,
        },

        // Float comparisons (promote int to float)
        (Value::Float(_), Value::Float(_))
        | (Value::Float(_), Value::Int(_))
        | (Value::Int(_), Value::Float(_)) => {
            let lf = match left {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => unreachable!(),
            };
            let rf = match right {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => unreachable!(),
            };

            match op {
                ComparisonOp::Eq => lf == rf,
                ComparisonOp::Ne => lf != rf,
                ComparisonOp::Lt => lf < rf,
                ComparisonOp::Gt => lf > rf,
                ComparisonOp::Le => lf <= rf,
                ComparisonOp::Ge => lf >= rf,
            }
        }

        // String comparisons
        (Value::String(l), Value::String(r)) => match op {
            ComparisonOp::Eq => l == r,
            ComparisonOp::Ne => l != r,
            ComparisonOp::Lt => l < r,
            ComparisonOp::Gt => l > r,
            ComparisonOp::Le => l <= r,
            ComparisonOp::Ge => l >= r,
        },

        // None comparisons
        (Value::None, Value::None) => match op {
            ComparisonOp::Eq => true,
            ComparisonOp::Ne => false,
            _ => return Err("Cannot order-compare None values".to_string()),
        },

        (Value::None, _) | (_, Value::None) => match op {
            ComparisonOp::Eq => false,
            ComparisonOp::Ne => true,
            _ => return Err("Cannot order-compare with None".to_string()),
        },

        _ => {
            return Err(format!(
                "Cannot compare {} and {} with {:?}",
                left.type_name(),
                right.type_name(),
                op
            ))
        }
    };

    Ok(Value::Bool(result))
}
