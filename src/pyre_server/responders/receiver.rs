use pyo3::prelude::*;
use crossbeam::channel::{Sender, Receiver, bounded, TrySendError};

use crate::pyre_server::responders::Payload;


/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataReceiver {
    rx: Receiver<Payload>,
}

impl DataReceiver {
    /// Create a new handler with the given sender.
    pub fn new(rx: Receiver<Payload>) -> Self {
        Self { rx }
    }
}

#[pymethods]
impl DataReceiver {
    /// Invoked by python passing more_body which represents if there
    /// is any more body to expect or not, and the body itself.
    #[call]
    fn __call__(&self) -> PyResult<()> {

        Ok(())
    }
}


pub struct ReceiverHandler {
    /// The sender half for sending body chunks.
    receiver_tx: Sender<Payload>,

    /// The receiver half for sending body chunks.
    receiver_rx: Receiver<Payload>,
}

impl ReceiverHandler {
    pub fn new() -> Self {
        let (tx, rx) = bounded(10);
        Self {
            receiver_tx: tx,
            receiver_rx: rx,
        }
    }

    pub fn make_handle(&self) -> DataReceiver{
        DataReceiver::new(self.receiver_rx.clone())
    }

    pub fn send(&self, data: Payload) -> Result<(), TrySendError<Payload>> {
        self.receiver_tx.try_send(data)
    }
}