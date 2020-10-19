use pyo3::prelude::*;
use pyo3::PyAsyncProtocol;
use pyo3::types::PyDict;

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::asyncio;
use crate::protocol;
use crate::http;



const HIGH_WATER_LIMIT: usize = 64 * 1024;  // 64KiB
static CALLBACK: OnceCell<PyObject> = OnceCell::new();


pub fn setup(callback: PyObject) {
    let _: &PyObject = CALLBACK.get_or_init(|| {
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
}

#[pymethods]
impl RequestResponseCycle {
    fn send_start(
        &mut self,
        py: Python,
        status: u16,
        headers: Vec<(&[u8], &[u8])>,
    ) -> PyResult<()> {

        let body = http::format_response_start(
            status,
            headers,
        )?;

        let _ = self.send_body(
            py,
            body.as_slice(),
        )?;

        Ok(())
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
    fn __await__(slf: PyRef<Self>) -> PyResult<PyObject> {
        let callback: &PyObject = CALLBACK
            .get()
            .unwrap();


        let gil = Python::acquire_gil();
        let py = gil.python();
        let fut = callback.call1(py, (slf,))?;
        Ok(fut.call_method0(py, "__await__")?)
    }
}
