use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyBytes;

use std::sync::Arc;
use std::sync::mpsc;

use bytes::{Bytes, BytesMut};

use arrayvec::ArrayVec;

use crate::server::flow_control::FlowControl;
use test::test_main;


const MAX_HEADERS: usize = 32;

/// 64KiB Chunk
const HIGH_WATER_LIMIT: usize = 64 * 1024;

/// Max amount of messages to buffer onto the channel
const CHANNEL_BUFFER_SIZE: usize = 10;

/// Standard Keep-Alive timeout
const KEEP_ALIVE_TIMEOUT: usize = 5;



#[pyclass]
pub struct PyreProtocol {
    // Python callbacks
    callback: PyObject,

    // transport management
    transport: Option<Arc<PyObject>>,
    flow_control: Option<Arc<FlowControl>>,

    // internal state
    parse_complete: bool,
    expected_length: usize,
    task_disconnected: bool,  // means the sender will fail on send if true

    // storage
    body_tx: Option<mpsc::SyncSender<BytesMut>>,
    body: BytesMut,
}

#[pymethods]
impl PyreProtocol {
    #[new]
    pub fn new(
        py: Python,
        callback: PyObject,
    ) -> PyResult<Self> {

        Ok(PyreProtocol {
            callback,

            transport: None,
            flow_control: None,

            parse_complete: false,
            expected_length: 0,
            task_disconnected: false,

            body_tx: None,
            body: BytesMut::new(),
        })
    }

    /// Called when the connection is first established
    fn connection_made(&mut self, transport: PyObject) {

        let transport = Arc::new(transport);
        let flow_control = Arc::new(FlowControl::new(
            transport.clone()
        ));

        self.transport = Some(transport);
        self.flow_control = Some(flow_control);

    }

    /// Called when the connection is closed
    fn connection_lost(&self, _exception: PyObject) {

    }

    /// Required but not used eof callback
    fn eof_received(&self) {

    }

    /// Received data from the socket
    fn data_received(&mut self, py: Python, data: &[u8]) -> PyResult<()> {
        if self.task_disconnected {
            return Err(PyRuntimeError::new_err(
                "The asgi task has ended while the server is still receiving data."
            ))
        }

        if !self.parse_complete {
            self.parse(py, data)?;
        } else {
            self.body.extend_from_slice(data);
            self.on_body()?;
        }

        Ok(())
    }

    /// called when the socket reaches the high water limit
    fn pause_writing(&self, py: Python) -> PyResult<()>{
        let flow_control = match self.flow_control.as_ref() {
            Some(fc) => fc,
            _ => return Ok(())
        };

        flow_control.pause_reading(py)?;

        Ok(())
    }

    /// called when the socket can start being written to again
    fn resume_writing(&self) {
        let flow_control = match self.flow_control.as_ref() {
            Some(fc) => fc,
            _ => return
        };

        flow_control.pause_writing();
    }
}

impl PyreProtocol {
    fn parse(&mut self, py: Python, data: &[u8]) -> PyResult<()> {

        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut request = httparse::Request::new(&mut headers);

        let status = match request.parse(data) {
            Ok(s) => s,
            Err(e) => return Err(PyRuntimeError::new_err(format!(
                "{:?}", e
            )))
        };

        if status.is_partial() {
            return Ok(())
        }
        self.parse_complete = true;

        // Converts and checks headers for content length specifiers
        let mut headers = {
            ArrayVec::<[(Py<PyBytes>, Py<PyBytes>); MAX_HEADERS]>::new()
        };
        for header in request.headers {
            if header.name == "content-length" {
                self.expected_length = String::from_utf8_lossy(&header.value)
                    .parse()
                    .unwrap_or_else(|_|0);
            }


            let name = Py::from(PyBytes::new(
                py,
                header.name.as_bytes()
            ));

            let value = Py::from(PyBytes::new(
                py,
                header.value
            ));

            headers.push((name, value))
        }

        // Get the path of the request
        let path = Py::from(PyBytes::new(
            py,
            request.path.unwrap_or("/").as_bytes()
        ));

        // This should never error default to the or values
        let method = String::from(request.method.unwrap_or("GET"));
        let version = request.version.unwrap_or(1);

        // Submit the complete callback
        self.on_parse_complete(
            py,
            version,
            path,
            method,
            headers,
        )?;

        // Handle the remaining body
        let (_, body) = data.split_at(status.unwrap());
        self.body.extend_from_slice(body);
        self.on_body()?;

        Ok(())
    }

    /// Called only once when the data has been parsed into headers, etc...
    /// this should always be in charge of creating the tasks and channels
    /// needed for the on_body callback to work.
    fn on_parse_complete(
        &self,
        py: Python,
        version: u8,
        path: Py<PyBytes>,
        method: String,
        headers: ArrayVec<[(Py<PyBytes>, Py<PyBytes>); MAX_HEADERS]>,
    ) -> PyResult<()> {

    }

    /// Called when ever data is received and the sending transmitter is
    /// able to send unless it is the first time calling it.
    ///
    /// This is invoked *after* on_parse_complete is called giving time
    /// to initialise the sender and receiver along with any tasks
    fn on_body(&mut self) -> PyResult<()> {
        // This should never reasonably error unless everything is one fire.
        let tx = match self.body_tx.as_ref() {
            Some(t) => t,
            _ => return Err(PyRuntimeError::new_err(
                "Unexpected NoneType found when unwrapping sender channel."
            ))
        };

        if let Err(_) = tx.send(self.body.clone()) {
            self.task_disconnected = true
        }

        Ok(())
    }
}







