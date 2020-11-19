use pyo3::prelude::*;

use std::sync::Arc;

use crate::server::flow_control::FlowControl;
use bytes::Bytes;

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
    fn data_received(&self, data: &[u8]) {
        if !self.parse_complete {
            self.parse(data)?;
        } else {
            self.on_body(data)?;
        }

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
    fn parse(&self, data: &[u8]) -> PyResult<()> {

        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut request = httparse::Request::new(&mut headers);

        match request.parse(data) {
            Ok(s) => s,
            Err(e) => return Ok(())
        }

        Ok(())
    }

    fn on_parse_complete(&self) {

    }

    fn on_body(&self, body: &[u8]) -> PyResult<()> {

        Ok(())
    }
}







