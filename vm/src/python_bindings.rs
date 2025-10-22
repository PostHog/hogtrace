//! Python bindings for the HogTrace VM
//!
//! This module exposes the Rust VM to Python using PyO3.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyFrame};
use pyo3::exceptions::{PyValueError, PyRuntimeError};

use crate::parser;
use crate::program::{Program, Probe, ProbeSpec, FnTarget};
use crate::executor::Executor;
use crate::python_dispatcher::PythonDispatcher;

/// Compile HogTrace source code into a Program with bytecode
///
/// Args:
///     source (str): HogTrace source code
///
/// Returns:
///     Program: Compiled program with bytecode ready for execution
///
/// Raises:
///     ValueError: If compilation fails
///
/// Example:
///     >>> program = compile("fn:myapp.users.*:entry { capture(args); }")
///     >>> print(len(program.probes))
///     1
#[pyfunction]
fn compile(source: &str) -> PyResult<PyProgram> {
    parser::parse(source)
        .map(|program| PyProgram { inner: program })
        .map_err(|e| PyValueError::new_err(format!("Compilation error: {}", e)))
}

/// A compiled HogTrace program
///
/// Contains bytecode for all probes and a shared constant pool.
#[pyclass(name = "Program")]
#[derive(Clone)]
struct PyProgram {
    inner: Program,
}

#[pymethods]
impl PyProgram {
    /// Get the number of probes in this program
    #[getter]
    fn probes(&self) -> Vec<PyProbe> {
        self.inner
            .probes
            .iter()
            .map(|probe| PyProbe {
                inner: probe.clone(),
            })
            .collect()
    }

    /// Get the bytecode format version
    #[getter]
    fn version(&self) -> u32 {
        self.inner.version
    }

    /// Get the global sampling rate
    #[getter]
    fn sampling(&self) -> f32 {
        self.inner.sampling
    }

    /// Serialize the program to protobuf bytes
    ///
    /// Returns:
    ///     bytes: Serialized program
    ///
    /// Example:
    ///     >>> program = parse("fn:test:entry {}")
    ///     >>> data = program.to_bytes()
    ///     >>> loaded = Program.from_bytes(data)
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.inner
            .to_proto_bytes()
            .map(|bytes| PyBytes::new(py, &bytes))
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization error: {}", e)))
    }

    /// Deserialize a program from protobuf bytes
    ///
    /// Args:
    ///     data (bytes): Serialized program data
    ///
    /// Returns:
    ///     Program: Deserialized program
    ///
    /// Raises:
    ///     RuntimeError: If deserialization fails
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<PyProgram> {
        Program::from_proto_bytes(data)
            .map(|program| PyProgram { inner: program })
            .map_err(|e| PyRuntimeError::new_err(format!("Deserialization error: {}", e)))
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "<Program version={} probes={} sampling={}>",
            self.inner.version,
            self.inner.probes.len(),
            self.inner.sampling
        )
    }
}

/// A single probe with its specification and bytecode
#[pyclass(name = "Probe")]
#[derive(Clone)]
struct PyProbe {
    inner: Probe,
}

#[pymethods]
impl PyProbe {
    /// Get the probe ID
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Get the probe specification
    #[getter]
    fn spec(&self) -> PyProbeSpec {
        PyProbeSpec {
            inner: self.inner.spec.clone(),
        }
    }

    /// Get the predicate bytecode (empty if no predicate)
    #[getter]
    fn predicate<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.predicate)
    }

    /// Get the action body bytecode
    #[getter]
    fn body<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.body)
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "<Probe id='{}' spec={:?} predicate_len={} body_len={}>",
            self.inner.id,
            self.inner.spec,
            self.inner.predicate.len(),
            self.inner.body.len()
        )
    }
}

/// Probe specification defining where the probe is installed
#[pyclass(name = "ProbeSpec")]
struct PyProbeSpec {
    inner: ProbeSpec,
}

#[pymethods]
impl PyProbeSpec {
    /// Get the probe specifier (e.g., "myapp.users.create")
    #[getter]
    fn specifier(&self) -> String {
        match &self.inner {
            ProbeSpec::Fn { specifier, .. } => specifier.clone(),
        }
    }

    /// Get the probe target ("entry" or "exit")
    #[getter]
    fn target(&self) -> &str {
        match &self.inner {
            ProbeSpec::Fn { target, .. } => match target {
                FnTarget::Entry => "entry",
                FnTarget::Exit => "exit",
            },
        }
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("fn:{}:{}", self.specifier(), self.target())
    }
}

/// Execute a probe against a Python frame
///
/// Args:
///     probe: The probe to execute (must be from a compiled Program)
///     frame: Python frame object
///     retval: Optional return value (for exit probes)
///     exception: Optional exception (for exit probes)
///
/// Returns:
///     dict or None: Dictionary of captured data, or None if predicate failed
///
/// Example:
///     >>> import sys
///     >>> program = compile("fn:test:entry { capture(arg0=args[0]); }")
///     >>> probe = program.probes[0]
///     >>> frame = sys._getframe()
///     >>> result = execute_probe(program, probe, frame)
#[pyfunction]
#[pyo3(signature = (program, probe, frame, retval=None, exception=None))]
fn execute_probe<'py>(
    py: Python<'py>,
    program: &PyProgram,
    probe: &PyProbe,
    frame: Bound<'py, PyFrame>,
    retval: Option<Bound<'py, PyAny>>,
    exception: Option<Bound<'py, PyAny>>,
) -> PyResult<Option<Bound<'py, PyDict>>> {
    // Create dispatcher
    let retval_owned = retval.map(|r| r.unbind());
    let exception_owned = exception.map(|e| e.unbind());

    let mut dispatcher = if retval_owned.is_some() || exception_owned.is_some() {
        PythonDispatcher::new_exit(py, frame, retval_owned, exception_owned)
    } else {
        PythonDispatcher::new_entry(py, frame)
    };

    // Create executor
    let mut executor = Executor::new(&program.inner.constant_pool, &mut dispatcher);

    // Execute predicate if present
    if !probe.inner.predicate.is_empty() {
        match executor.execute(&probe.inner.predicate) {
            Ok(crate::value::Value::Bool(true)) => {
                // Predicate passed, continue to body
            }
            Ok(crate::value::Value::Bool(false)) => {
                // Predicate failed
                return Ok(None);
            }
            Ok(_) => {
                return Err(PyRuntimeError::new_err(
                    "Predicate must return a boolean value",
                ));
            }
            Err(e) => {
                return Err(PyRuntimeError::new_err(format!("Predicate error: {}", e)));
            }
        }
    }

    // Execute probe body
    if let Err(e) = executor.execute(&probe.inner.body) {
        return Err(PyRuntimeError::new_err(format!("Execution error: {}", e)));
    }

    // Get captured events
    let captures = dispatcher.take_captures();
    if captures.is_empty() {
        return Ok(None);
    }

    // Convert first capture to Python dict
    // (For now, we only return the first capture. Could be changed to return a list)
    let capture = &captures[0];
    let dict = dispatcher.capture_to_py_dict(capture)?;

    Ok(Some(dict.bind(py).clone()))
}

/// Probe executor class for executing probes against Python frames
///
/// Example:
///     >>> program = compile("fn:test:entry { capture(args); }")
///     >>> executor = ProbeExecutor(program, program.probes[0])
///     >>> result = executor.execute(frame)
#[pyclass(name = "ProbeExecutor")]
struct PyProbeExecutor {
    program: PyProgram,
    probe: PyProbe,
}

#[pymethods]
impl PyProbeExecutor {
    /// Create a new probe executor
    #[new]
    fn new(program: PyProgram, probe: PyProbe) -> Self {
        PyProbeExecutor { program, probe }
    }

    /// Execute the probe against a Python frame
    ///
    /// Args:
    ///     frame: Python frame object
    ///     retval: Optional return value (for exit probes)
    ///     exception: Optional exception (for exit probes)
    ///
    /// Returns:
    ///     dict or None: Captured data or None if predicate failed
    #[pyo3(signature = (frame, retval=None, exception=None))]
    fn execute<'py>(
        &self,
        py: Python<'py>,
        frame: Bound<'py, PyFrame>,
        retval: Option<Bound<'py, PyAny>>,
        exception: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        execute_probe(py, &self.program, &self.probe, frame, retval, exception)
    }

    fn __repr__(&self) -> String {
        format!("<ProbeExecutor probe_id='{}'>", self.probe.inner.id)
    }
}

/// Python module definition
#[pymodule]
fn _vm_rust(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
    // Add functions
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(execute_probe, m)?)?;

    // Add classes
    m.add_class::<PyProgram>()?;
    m.add_class::<PyProbe>()?;
    m.add_class::<PyProbeSpec>()?;
    m.add_class::<PyProbeExecutor>()?;

    // Add version constant
    m.add("BYTECODE_VERSION", crate::BYTECODE_VERSION)?;

    Ok(())
}
