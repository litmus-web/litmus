use pyo3::{PyObject, Python, IntoPy, Py, PyResult};
use pyo3::types::PyTuple;

use std::sync::Arc;


/// A cheaply cloneable helper function that wraps a python callback.
#[derive(Clone)]
pub struct CallbackHandler {
    /// The python callback itself.
    cb: Arc<PyObject>,
}

impl CallbackHandler {
    /// Creates a new instance of this struct wrapping the PyObject in a
    /// arc to make for cheap clones.
    pub fn new(cb: PyObject) -> Self {
        Self { cb: Arc::new(cb) }
    }

    /// Invokes the callback by acquiring the gil internally.
    pub fn invoke(&self, args: impl IntoPy<Py<PyTuple>>) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
           let _ = self.cb.call1(py, args)?;
            Ok(())
        })
    }
}