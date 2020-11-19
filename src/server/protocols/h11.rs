use pyo3::prelude::*;

use std::sync::Arc;

use crate::server::flow_control::FlowControl;

const MAX_HEADERS: usize = 32;

/// 64KiB Chunk
const HIGH_WATER_LIMIT: usize = 64 * 1024;

/// Max amount of messages to buffer onto the channel
const CHANNEL_BUFFER_SIZE: usize = 10;

/// Standard Keep-Alive timeout
const KEEP_ALIVE_TIMEOUT: usize = 5;


#[pyclass]
pub struct PyreProtocol {
    callback: PyObject,

    transport: Option<Arc<PyObject>>,
    flow_control: Option<Arc<FlowControl>>,


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
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let parser = httparse::Request::new(&mut headers);
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







