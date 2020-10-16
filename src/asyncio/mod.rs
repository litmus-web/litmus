///
/// This crate is responsible for handling all interactions with AsyncIO in Python
/// as a set of helper functions to make the code more readable on the main file.
///
use pyo3::prelude::*;

use crate::asgi::RequestResponseCycle;

// Standardised helpers
pub fn get_loop(py: Python) -> PyResult<PyObject> {
    let module = py.import("asyncio")?;
    Ok(module.call_method0("get_event_loop")?.into_py(py))
}

// Task helpers
pub fn create_server_task(py: Python, task: RequestResponseCycle) -> PyResult<&PyAny> {
    let module = py.import("asyncio")?;
    let future = module.call_method1("ensure_future", (task,))?;
    Ok(future)
}

// Protocol Transport Helpers
pub fn write_transport(py: Python, transport: &PyObject, data: &[u8]) -> PyResult<()> {
    transport.call_method1(py, "write", (data,))?;
    Ok(())
}

pub fn write_eof_transport(py: Python, transport: &PyObject) -> PyResult<()> {
    let _ = write_transport(py, transport, b"")?;
    Ok(())
}

pub fn close_transport(py: Python, transport: &PyObject) -> PyResult<()> {
    transport.call_method0(py, "close")?;
    Ok(())
}

pub fn pause_reading_transport(py: Python, transport: &PyObject) -> PyResult<()> {
    transport.call_method0(py, "pause_reading")?;
    Ok(())
}

pub fn resume_reading_transport(py: Python, transport: &PyObject) -> PyResult<()> {
    transport.call_method0(py, "resume_reading")?;
    Ok(())
}