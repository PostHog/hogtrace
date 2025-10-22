use crate::dispatcher::Dispatcher;
use crate::value::Value;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyFrame, PyInt, PyString, PyTuple};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Marker type for the request store proxy
///
/// When LoadVar("$req") or LoadVar("$request") is executed, we return
/// Value::Object(Box::new(RequestStoreProxy)) to represent the request store.
/// This allows GetAttr and SetAttr operations to interact with the RequestLocalStore.
#[derive(Debug)]
struct RequestStoreProxy;

/// Captured event data from capture() calls
#[derive(Debug)]
pub struct CaptureEvent {
    /// Named arguments as key-value pairs
    pub data: HashMap<String, Value>,
}

/// Python-specific dispatcher implementation using PyO3
///
/// This dispatcher handles all Python-specific operations:
/// - Variable access (args, kwargs, retval, exception, etc.) from the Python frame
/// - Request-scoped variables ($req.*) via RequestLocalStore
/// - Attribute access on Python objects
/// - Item access (dict keys, list indices)
/// - Built-in function calls (timestamp, rand, capture, etc.)
pub struct PythonDispatcher<'py> {
    /// Python GIL token
    py: Python<'py>,

    /// The current Python frame being probed
    frame: Bound<'py, PyFrame>,

    /// Whether this is an entry or exit probe (determines which variables are available)
    is_exit: bool,

    /// Return value (only available in exit probes)
    retval: Option<Py<PyAny>>,

    /// Exception (only available in exit probes, None if no exception)
    exception: Option<Py<PyAny>>,

    /// Request-scoped variables storage (Python RequestLocalStore instance)
    store: Py<PyAny>,

    /// Accumulated capture events from capture() calls
    captures: Vec<CaptureEvent>,
}

impl<'py> PythonDispatcher<'py> {
    /// Create a new Python dispatcher for an entry probe
    pub fn new_entry(py: Python<'py>, frame: Bound<'py, PyFrame>, store: Py<PyAny>) -> Self {
        PythonDispatcher {
            py,
            frame,
            is_exit: false,
            retval: None,
            exception: None,
            store,
            captures: Vec::new(),
        }
    }

    /// Create a new Python dispatcher for an exit probe
    pub fn new_exit(
        py: Python<'py>,
        frame: Bound<'py, PyFrame>,
        retval: Option<Py<PyAny>>,
        exception: Option<Py<PyAny>>,
        store: Py<PyAny>,
    ) -> Self {
        PythonDispatcher {
            py,
            frame,
            is_exit: true,
            retval,
            exception,
            store,
            captures: Vec::new(),
        }
    }

    /// Get all accumulated capture events
    pub fn take_captures(&mut self) -> Vec<CaptureEvent> {
        std::mem::take(&mut self.captures)
    }

    /// Convert a CaptureEvent to a Python dictionary
    pub fn capture_to_py_dict(&self, capture: &CaptureEvent) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(self.py);
        for (key, value) in &capture.data {
            let py_value = self.value_to_py(value)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
            dict.set_item(key, py_value)?;
        }
        Ok(dict.into())
    }

    /// Helper: convert Python object to Value
    fn py_to_value(&self, obj: &Bound<'py, PyAny>) -> Result<Value, String> {
        // Try primitive conversions first
        if let Ok(b) = obj.extract::<bool>() {
            return Ok(Value::Bool(b));
        }
        if let Ok(i) = obj.extract::<i64>() {
            return Ok(Value::Int(i));
        }
        if let Ok(f) = obj.extract::<f64>() {
            return Ok(Value::Float(f));
        }
        if let Ok(s) = obj.extract::<String>() {
            return Ok(Value::String(s));
        }
        if obj.is_none() {
            return Ok(Value::None);
        }

        // Everything else is a language-specific object
        Ok(Value::Object(Box::new(obj.clone().unbind())))
    }

    /// Helper: convert Value to Python object
    fn value_to_py(&self, value: &Value) -> Result<Bound<'py, PyAny>, String> {
        match value {
            Value::Bool(b) => Ok(PyBool::new(self.py, *b).as_any().clone()),
            Value::Int(i) => Ok(PyInt::new(self.py, *i).as_any().clone()),
            Value::Float(f) => Ok(PyFloat::new(self.py, *f).as_any().clone()),
            Value::String(s) => Ok(PyString::new(self.py, s).as_any().clone()),
            Value::None => Ok(self.py.None().into_bound(self.py)),
            Value::Object(obj) => {
                // Downcast the boxed Any back to Py<PyAny>
                obj.downcast_ref::<Py<PyAny>>()
                    .ok_or_else(|| "Failed to downcast Object to Py<PyAny>".to_string())
                    .map(|py_obj| py_obj.bind(self.py).clone())
            }
        }
    }

    /// Get function arguments as a tuple
    fn get_args(&self) -> Result<Value, String> {
        // Access frame locals to get function arguments
        // This is a simplified implementation - in production, you'd need to
        // properly extract arguments from the frame based on the code object
        let locals = self.frame.getattr("f_locals")
            .map_err(|e| format!("Failed to get f_locals: {}", e))?;

        let args = locals
            .get_item("args")
            .ok()
            .unwrap_or_else(|| PyTuple::empty(self.py).into_any());

        self.py_to_value(&args)
    }

    /// Get a specific positional argument by index
    fn get_arg_n(&self, n: usize) -> Result<Value, String> {
        // Similar to get_args, but extract specific argument
        // This is simplified - proper implementation would inspect the code object
        let locals = self.frame.getattr("f_locals")
            .map_err(|e| format!("Failed to get f_locals: {}", e))?;

        // Try to get from args tuple
        if let Ok(args_obj) = locals.get_item("args") {
            if let Ok(args_tuple) = args_obj.downcast::<PyTuple>() {
                if let Ok(arg) = args_tuple.get_item(n) {
                    return self.py_to_value(&arg);
                }
            }
        }

        Err(format!("Argument {} not found", n))
    }

    /// Copy a Value (for primitives; Objects use reference counting)
    fn copy_value(&self, value: &Value) -> Result<Value, String> {
        match value {
            Value::Bool(b) => Ok(Value::Bool(*b)),
            Value::Int(i) => Ok(Value::Int(*i)),
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::String(s) => Ok(Value::String(s.clone())),
            Value::None => Ok(Value::None),
            Value::Object(obj) => {
                // For Py<PyAny> objects, we need to clone_ref with a Python token
                if let Some(py_obj) = obj.downcast_ref::<Py<PyAny>>() {
                    let cloned = py_obj.clone_ref(self.py);
                    Ok(Value::Object(Box::new(cloned)))
                } else {
                    Err("Cannot copy unknown object type".to_string())
                }
            }
        }
    }
}

impl<'py> Dispatcher for PythonDispatcher<'py> {
    fn load_variable(&mut self, name: &str) -> Result<Value, String> {
        // Handle request-scoped variables - return proxy for $req/$request
        if name == "$req" || name == "$request" {
            return Ok(Value::Object(Box::new(RequestStoreProxy)));
        }

        // Handle special variables
        match name {
            "args" => self.get_args(),
            "kwargs" => {
                // Get keyword arguments from frame
                let locals = self.frame.getattr("f_locals")
                    .map_err(|e| format!("Failed to get f_locals: {}", e))?;
                let kwargs = locals
                    .get_item("kwargs")
                    .ok()
                    .unwrap_or_else(|| PyDict::new(self.py).into_any());
                self.py_to_value(&kwargs)
            }
            "retval" => {
                if !self.is_exit {
                    return Err("retval only available in exit probes".to_string());
                }
                self.retval
                    .as_ref()
                    .ok_or_else(|| "No return value".to_string())
                    .and_then(|obj| self.py_to_value(&obj.bind(self.py)))
            }
            "exception" => {
                if !self.is_exit {
                    return Err("exception only available in exit probes".to_string());
                }
                match &self.exception {
                    Some(exc) => self.py_to_value(&exc.bind(self.py)),
                    None => Ok(Value::None),
                }
            }
            "self" => {
                // Get 'self' from locals (for method calls)
                let locals = self.frame.getattr("f_locals")
                    .map_err(|e| format!("Failed to get f_locals: {}", e))?;
                locals
                    .get_item("self")
                    .map_err(|e| format!("'self' not found: {}", e))
                    .and_then(|obj| self.py_to_value(&obj))
            }
            "locals" => {
                // Return all local variables as a dict
                let locals = self.frame.getattr("f_locals")
                    .map_err(|e| format!("Failed to get f_locals: {}", e))?;
                self.py_to_value(&locals)
            }
            "globals" => {
                // Return all global variables as a dict
                let globals = self.frame.getattr("f_globals")
                    .map_err(|e| format!("Failed to get f_globals: {}", e))?;
                self.py_to_value(&globals)
            }
            _ => {
                // Check if it's arg0, arg1, arg2, etc.
                if name.starts_with("arg") {
                    if let Ok(n) = name[3..].parse::<usize>() {
                        return self.get_arg_n(n);
                    }
                }

                // Try to get from frame locals
                let locals = self.frame.getattr("f_locals")
                    .map_err(|e| format!("Failed to get f_locals: {}", e))?;
                locals
                    .get_item(name)
                    .map_err(|e| format!("Variable {} not found: {}", name, e))
                    .and_then(|obj| self.py_to_value(&obj))
            }
        }
    }

    fn store_variable(&mut self, name: &str, value: Value) -> Result<(), String> {
        // StoreVar is no longer used for request variables (they use SetAttr now)
        // This method is kept for potential future use with other variable types
        Err(format!(
            "Cannot store to variable {} (use SetAttr for $req.* assignments)",
            name
        ))
    }

    fn get_attribute(&mut self, obj: &Value, attr: &str) -> Result<Value, String> {
        // Check if this is the request store proxy
        if let Value::Object(boxed) = obj {
            if boxed.downcast_ref::<RequestStoreProxy>().is_some() {
                // Call store.get(attr) which returns None if not set
                let result = self.store
                    .call_method1(self.py, "get", (attr,))
                    .map_err(|e| format!("Failed to get request variable {}: {}", attr, e))?;
                return self.py_to_value(&result.bind(self.py));
            }
        }

        // Regular attribute access on Python objects
        let py_obj = self.value_to_py(obj)?;
        py_obj
            .getattr(attr)
            .map_err(|e| format!("Failed to get attribute {}: {}", attr, e))
            .and_then(|result| self.py_to_value(&result))
    }

    fn set_attribute(&mut self, obj: &Value, attr: &str, value: Value) -> Result<(), String> {
        // Check if this is the request store proxy
        if let Value::Object(boxed) = obj {
            if boxed.downcast_ref::<RequestStoreProxy>().is_some() {
                // Convert value to Python and call store.set(attr, value)
                let py_value = self.value_to_py(&value)?;
                self.store
                    .call_method1(self.py, "set", (attr, py_value))
                    .map_err(|e| format!("Failed to set request variable {}: {}", attr, e))?;
                return Ok(());
            }
        }

        // Regular Python objects don't support attribute assignment through this interface
        Err(format!("Cannot set attribute {} on non-request object", attr))
    }

    fn get_item(&mut self, obj: &Value, key: &Value) -> Result<Value, String> {
        let py_obj = self.value_to_py(obj)?;
        let py_key = self.value_to_py(key)?;

        py_obj
            .get_item(&py_key)
            .map_err(|e| format!("Failed to get item: {}", e))
            .and_then(|result| self.py_to_value(&result))
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Handle built-in functions
        match name {
            "timestamp" => {
                // Return current Unix timestamp
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| format!("Failed to get timestamp: {}", e))?;
                Ok(Value::Float(now.as_secs_f64()))
            }

            "rand" => {
                // Return random float 0.0-1.0
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hash, Hasher};
                let mut hasher = RandomState::new().build_hasher();
                SystemTime::now().hash(&mut hasher);
                let hash = hasher.finish();
                let rand_f64 = (hash as f64) / (u64::MAX as f64);
                Ok(Value::Float(rand_f64))
            }

            "len" => {
                if args.len() != 1 {
                    return Err(format!("len() takes 1 argument, got {}", args.len()));
                }
                let obj = self.value_to_py(&args[0])?;
                let len = obj
                    .len()
                    .map_err(|e| format!("len() failed: {}", e))?;
                Ok(Value::Int(len as i64))
            }

            "str" | "int" | "float" => {
                if args.len() != 1 {
                    return Err(format!("{}() takes 1 argument, got {}", name, args.len()));
                }
                match name {
                    "str" => Ok(Value::String(format!("{}", args[0]))),
                    "int" => args[0].as_int().map(Value::Int),
                    "float" => args[0].as_float().map(Value::Float),
                    _ => unreachable!(),
                }
            }

            "capture" | "send" => {
                // Accumulate capture events instead of sending directly
                let mut data = HashMap::new();

                // Check if this is named arguments (even count, odd indices are strings)
                let is_named = args.len() % 2 == 0
                    && args.iter().step_by(2).all(|v| matches!(v, Value::String(_)));

                if is_named {
                    // Named arguments: args = [name1, value1, name2, value2, ...]
                    for i in (0..args.len()).step_by(2) {
                        if let Value::String(key) = &args[i] {
                            // Clone the value manually since Value doesn't implement Clone
                            let value = match &args[i + 1] {
                                Value::Bool(b) => Value::Bool(*b),
                                Value::Int(n) => Value::Int(*n),
                                Value::Float(f) => Value::Float(*f),
                                Value::String(s) => Value::String(s.clone()),
                                Value::None => Value::None,
                                Value::Object(_) => Value::None, // Can't clone objects
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

            _ => {
                // Try to call a Python function
                let builtins = self
                    .py
                    .import("builtins")
                    .map_err(|e| format!("Failed to import builtins: {}", e))?;

                let func = builtins
                    .getattr(name)
                    .map_err(|e| format!("Function {} not found: {}", name, e))?;

                // Convert args to Python objects
                let py_args: Result<Vec<Bound<'py, PyAny>>, String> = args
                    .iter()
                    .map(|v| self.value_to_py(v))
                    .collect();
                let py_args = py_args?;

                // Create tuple from args
                let args_tuple = PyTuple::new(self.py, &py_args)
                    .map_err(|e| format!("Failed to create args tuple: {}", e))?;

                let result = func
                    .call1(&args_tuple)
                    .map_err(|e| format!("Failed to call {}: {}", name, e))?;

                self.py_to_value(&result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_event_creation() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), Value::Int(42));
        data.insert("event".to_string(), Value::String("login".to_string()));

        let capture = CaptureEvent { data };

        assert_eq!(capture.data.len(), 2);
        assert!(matches!(capture.data.get("user_id"), Some(Value::Int(42))));
        assert!(matches!(
            capture.data.get("event"),
            Some(Value::String(s)) if s == "login"
        ));
    }
}
