use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

use crossbeam::channel::{Sender, Receiver, bounded, TryRecvError};

use crate::pyre_server::responders::Payload;

const OP_SEND_NOTIFY: usize = 1;
const OP_SEND_BODY: usize = 2;
const OP_SEND_BODY_END: usize = 3;


/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataSender {
    tx: Sender<Payload>,
}

impl DataSender {
    /// Create a new handler with the given sender.
    pub fn new(tx: Sender<Payload>) -> Self {
        Self { tx }
    }
}

#[pymethods]
impl DataSender {
    /// Invoked by python passing an opcode and a body in the form of bytes.
    ///
    /// The `opcode` describes the nature of the call it can be one of the
    /// following opcodes:
    ///
    /// - `OP_SEND_NOTIFY = 1`
    /// - `OP_SEND_BODY = 2`
    /// - `OP_SEND_BODY_END = 3`
    ///
    /// Any other integers will raise a ValueError.
    ///
    /// Only `OP_SEND_BODY` & `OP_SEND_BODY_END` use the body buffer, in
    /// the case
    #[call]
    fn __call__(&self, opcode: usize, body: Vec<u8>) -> PyResult<()> {
        let (more_body, body) = match opcode {
            OP_SEND_NOTIFY => {
                return Ok(())
            }
            OP_SEND_BODY => {
                (true, body)
            },
            OP_SEND_BODY_END => {
                (false, body)
            },

            n => return Err(PyValueError::new_err(format!(
                "value {:?} is not a valid op code for sender", n
            )))
        };

        if let Err(e) = self.tx.send((more_body, body)) {
            return Err(PyRuntimeError::new_err(format!("{:?}", e)))
        }

        Ok(())
    }
}


pub struct SenderFactory {
    /// The sender half for sending body chunks.
    sender_tx: Sender<Payload>,

    /// The receiver half for sending body chunks.
    sender_rx: Receiver<Payload>,
}

impl SenderFactory {
    /// Constructs a new factory.
    pub fn new() -> Self {
        let (tx, rx) = bounded(10);
        Self {
            sender_tx: tx,
            sender_rx: rx,
        }
    }

    /// Makes a new sending handle with the given factory channels.
    pub fn make_handle(&self) -> DataSender {
        DataSender::new(self.sender_tx.clone())
    }

    /// Receives data from any DataSenders that have submitted
    /// data to the channel.
    pub fn recv(&self) -> Result<Payload, TryRecvError> {
        self.sender_rx.try_recv()
    }
}