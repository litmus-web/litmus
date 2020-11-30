// Pyo3 experimental
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod server;

use server::net::listener;
use server::net::handler;


#[pyfunction]
fn setup(
    loop_create_task: PyObject,
    loop_remove_reader: PyObject,
    loop_remove_writer: PyObject,
) {
    handler::setup(
        loop_create_task,
        loop_remove_reader,
        loop_remove_writer,
    );
}


///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn pyre(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<listener::PyreListener>()?;
    m.add_class::<listener::PyreClientAddrPair>()?;
    m.add_function(wrap_pyfunction!(setup, m)?).unwrap();
    Ok(())
}
