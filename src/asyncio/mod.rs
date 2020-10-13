use pyo3::prelude::*;

use crate::RequestResponseCycle;


pub fn get_loop(py: Python) -> PyResult<PyObject> {
    let module = py.import("asyncio")?;
    Ok(module.call_method0("get_event_loop")?.into_py(py))
}

pub fn create_server_task(py: Python, task: RequestResponseCycle) -> PyResult<PyObject> {
    let module = py.import("asyncio")?;
    Ok(module.call_method1("ensure_future", (task,))?.into_py(py))
}

pub fn write_transport(py: Python, transport: &PyObject, data: &[u8]) -> PyResult<()> {
    transport.call_method1(py, "write", (data,))?;
    Ok(())
}

pub fn write_eof_transport(py: Python, transport: &PyObject) -> PyResult<()> {
    transport.call_method0(py, "write_eof")?;
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