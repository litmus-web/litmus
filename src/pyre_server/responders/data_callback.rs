use pyo3::prelude::*;

use crossbeam::channel::Sender;
use bytes::Bytes;
use pyo3::exceptions::PyRuntimeError;


pub type SenderPayload = (bool, Vec<u8>);


#[pyclass]
pub struct DataSender {
    tx: Sender<SenderPayload>,
}
impl DataSender {
    pub fn new(tx: Sender<SenderPayload>) -> Self {
        Self { tx }
    }
}

#[pymethods]
impl DataSender {
    #[call]
    fn __call__(&self, more_body: bool, body: Vec<u8>) -> PyResult<()> {
        if let Err(e) = self.tx.send((more_body, body)) {
            return Err(PyRuntimeError::new_err(format!("{:?}", e)))
        }

        Ok(())
    }
}