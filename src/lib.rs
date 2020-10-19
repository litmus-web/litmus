pub mod utils;
pub mod asyncio;
pub mod http;
pub mod framework;
pub mod asgi;
pub mod protocol;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;


#[pyfunction]
fn setup(callback: PyObject) {
    asgi::setup(callback)
}


///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn _pyre(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<protocol::RustProtocol>()?;
    m.add_class::<asgi::ASGIRunner>()?;
    m.add_function(wrap_pyfunction!(setup, m)?)?;
    Ok(())
}
