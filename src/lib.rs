// Pyo3 experimental
use pyo3::prelude::*;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod server;

use server::net::listener;


#[pyfunction]
fn setup(loop_add_reader: PyObject, loop_remove_reader: PyObject) {
    listener::setup(loop_add_reader, loop_remove_reader);
}



///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn pyre(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<listener::PyreListener>()?;
    //m.add_function(wrap_pyfunction!(setup, m)?).unwrap();
    Ok(())
}
