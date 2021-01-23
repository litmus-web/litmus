use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use crossbeam::channel::Sender;
use bytes::Bytes;


/// The payload that gets sent to the receiver half of the channel.
pub type SenderPayload = (bool, Vec<u8>);


/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataSender {
    tx: Sender<SenderPayload>,
}

impl DataSender {
    /// Create a new handler with the given sender.
    pub fn new(tx: Sender<SenderPayload>) -> Self {
        Self { tx }
    }
}

#[pymethods]
impl DataSender {
    /// Invoked by python passing more_body which represents if there
    /// is any more body to expect or not, and the body itself.
    #[call]
    fn __call__(&self, more_body: bool, body: Vec<u8>) -> PyResult<()> {
        if let Err(e) = self.tx.send((more_body, body)) {
            return Err(PyRuntimeError::new_err(format!("{:?}", e)))
        }

        Ok(())
    }
}