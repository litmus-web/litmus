use pyo3::prelude::*;
use pyo3::PyAsyncProtocol;

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::asyncio;
use crate::protocol;
use pyo3::types::PyDict;


const HIGH_WATER_LIMIT: usize = 64 * 1024;  // 64KiB
static FRAMEWORK_CALLBACK: OnceCell<PyObject> = OnceCell::new();


pub fn setup(callback: PyObject) {
    let _: &PyObject = FRAMEWORK_CALLBACK.get_or_init(|| {
        callback
    });
}


#[pyclass]
pub struct RequestResponseCycle {
    method: String,
    path: String,
    headers: HashMap<String, Vec<u8>>,

    transport: Arc<PyObject>,
    fc: Arc<protocol::FlowControl>,
}

impl RequestResponseCycle {
    pub fn new(
        method: String,
        path: String,
        headers: HashMap<String, Vec<u8>>,
        transport: Arc<PyObject>,
        fc: Arc<protocol::FlowControl>,
    ) -> Self {
        RequestResponseCycle {
            method,
            path,
            headers,

            transport,
            fc,
        }
    }

    fn send_body(&mut self, py: Python, body: &[u8]) -> PyResult<()> {
        Ok(asyncio::write_transport(
            py,
            &self.transport,
            body,
        )?)
    }


    fn send_end(&mut self, py: Python) -> PyResult<()> {
        Ok(asyncio::write_eof_transport(
            py,
            &self.transport,
        )?)
    }
}

#[pyproto]
impl PyAsyncProtocol for RequestResponseCycle {
    fn __await__(slf: PyRef<Self>) -> PyResult<()> {
        let callback: &PyObject = FRAMEWORK_CALLBACK
            .get()
            .unwrap();


        let gil = Python::acquire_gil();
        let py = gil.python();
        let fut = callback.call0(py)?;


        Ok(())
    }
}

/// ASGISend is the the 'send' coroutine of standard ASGI
/// servers, this is implemented as a class because it has to be
/// a coroutine object.
#[pyclass]
struct ASGISend {
    transport: Arc<PyObject>,
    fc: Arc<protocol::FlowControl>,
}

impl ASGISend {
    fn new(transport: Arc<PyObject>, fc: Arc<protocol::FlowControl>) -> Self {
        ASGISend{
            transport,
            fc,
        }
    }
}

#[pymethods]
impl ASGISend {
    #[call]
    fn __call__(&mut self, data: PyDict) {
        data.get_item("");
        self
    }
}


#[pyclass]
struct ASGIReceive {
    transport: Arc<PyObject>,
    fc: Arc<protocol::FlowControl>,
}

impl ASGIReceive {
    fn new(transport: Arc<PyObject>, fc: Arc<protocol::FlowControl>) -> Self {
        ASGIReceive{
            transport,
            fc,
        }
    }
}

#[pymethods]
impl ASGIReceive {
    #[call]
    fn __call__(&mut self, data: PyDict) {
        data.get_item("");
    }
}