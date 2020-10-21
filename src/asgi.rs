use pyo3::prelude::*;
use pyo3::PyAsyncProtocol;

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::asyncio;
use crate::protocol;
use crate::http;


const HIGH_WATER_LIMIT: usize = 64 * 1024;  // 64KiB
static CALLBACK: OnceCell<PyObject> = OnceCell::new();


/// A necessary setup function designed to save us constantly passing a callback
/// between Python and Rust which can be a rather expensive operation compared to running
/// the expensive operation once and then saving the callback to a static cell.
pub fn setup(callback: PyObject) {
    let _: &PyObject = CALLBACK.get_or_init(|| {
        callback
    });
}

/// The ASGIRunner struct and methods are what actually interact with the ASGI system
/// because Pyre is designed to work both as a standalone server as well as a framework system
/// we need a simple way of handling the two.
///
/// The ASGIRunner is created *after* all parsing has completed this means that the ASGI
/// interface may never be interacted with or ran when a request is taken in and rejected
/// by the server protocol itself.
#[pyclass]
pub struct ASGIRunner {
    method: String,
    path: String,
    headers: HashMap<String, Vec<u8>>,

    transport: Arc<PyObject>,
    fc: Arc<protocol::FlowControl>,
}

impl ASGIRunner {
    pub fn new(
        method: String,
        path: String,
        headers: HashMap<String, Vec<u8>>,
        transport: Arc<PyObject>,
        fc: Arc<protocol::FlowControl>,
    ) -> Self {
        //println!("initiated the runner!");  // todo remove

        ASGIRunner {
            method,
            path,
            headers,

            transport,
            fc,
        }
    }
}

#[pymethods]
impl ASGIRunner {

    fn can_write(&self) -> bool {
         self.fc.can_write()
    }

    fn close_conn(&mut self, py: Python) -> PyResult<()> {
        self.transport.call_method0(py, "close")?;
        Ok(())
    }

    /// The public function responsible for formatting and sending the status line and headers
    /// to the transport buffer.
    ///
    /// WARNING:
    ///     - This does *not* account for flow control and should only be sent
    ///       providing that the server has been given the go ahead from flow
    ///       control by using `can_write()`.
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

    /// The exposed Python function responsible for sending any content to the transport buffer.
    ///
    /// WARNING:
    ///     - This does *not* account for flow control and should only be sent
    ///       providing that the server has been given the go ahead from flow
    ///       control by using `can_write()`.
    fn send_body(&mut self, py: Python, body: &[u8]) -> PyResult<()> {
        // todo if this is called from python its really slow????

        Ok(asyncio::write_transport(
            py,
            &self.transport,
            body,
        )?)
    }

    /// The exposed Python function responsible signaling end of transmission,
    /// this is the equivalent of h11's eof which is a single `b''`, this is used
    /// in place of the transport's write_eof.
    ///
    /// WARNING:
    ///     - This does *not* account for flow control and should only be sent
    ///       providing that the server has been given the go ahead from flow
    ///       control by using `can_write()`.
    fn send_end(&mut self, py: Python) -> PyResult<()> {
        Ok(asyncio::write_eof_transport(
            py,
            &self.transport,
        )?)
    }
}

#[pyproto]
impl PyAsyncProtocol for ASGIRunner {
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

