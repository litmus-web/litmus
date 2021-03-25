use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyBlockingIOError};

use crossbeam::queue::SegQueue;
use crossbeam::channel::{
    Sender,
    Receiver,
    bounded,
    TryRecvError,
    TrySendError,
};

use std::sync::Arc;

use crate::pyre_server::responders::{Payload, WakerQueue};


/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataSender {
    /// The sending half of the channel used for sending data to the
    /// server handler.
    tx: Sender<Payload>,

    /// A queue of waiting events to invoke before the body
    /// can be written to again.
    waiter_queue: WakerQueue,
}

impl DataSender {
    /// Create a new handler with the given sender.
    pub fn new(tx: Sender<Payload>, waiter_queue: WakerQueue) -> Self {
        Self { tx, waiter_queue }
    }
}

#[pymethods]
impl DataSender {
    /// Sends the given body and more_body signal to the protocol handler.
    ///
    /// This raises a `BlockingIoError` if the queue / buffer is full, the
    /// invoker should wait till the queue / buffer is no longer full.
    ///
    /// This raises a `RuntimeError` if the channel receiver has been dropped.
    ///
    /// Args:
    ///     more_body:
    ///         A boolean to determine if the server should expect any more
    ///         chunks of body being sent or if the request is regarded as
    ///         being 'complete'.
    ///
    ///     body:
    ///         A chunk of bytes to be written to the socket.
    #[call]
    fn __call__(&self, more_body: bool, body: Vec<u8>) -> PyResult<()> {
        if let Err(e) = self.tx.try_send((more_body, body)) {
            if let TrySendError::Full(_) = e {
                return Err(PyBlockingIOError::new_err(()))
            }

            return Err(PyRuntimeError::new_err(format!("{:?}", e)))
        }

        Ok(())
    }

    /// Submits a given callback to the waiter queue.
    ///
    /// Any waiters in the queue when the socket is able to be written to will
    /// be taken out of the queue and invoked signalling the system's ability
    /// to be written to again.
    ///
    /// All waker callbacks are invoked with no parameters or key word
    /// arguments and are expected not to directly raise an error, in the case
    /// that a waker does raise an error the exception is ignored and implicitly
    /// silenced.
    ///
    /// Args:
    ///     waker:
    ///         A callback to be invoked when data can be written to the socket
    ///         without blocking.
    fn subscribe(&self, waker: PyObject) {
        self.waiter_queue.push(waker);
    }
}


pub struct SenderFactory {
    /// The sender half for sending body chunks.
    sender_tx: Sender<Payload>,

    /// The receiver half for receiving body chunks.
    sender_rx: Receiver<Payload>,

    /// A queue of waiting events to invoke before the body
    /// can be written to again.
    waiter_queue: WakerQueue,
}

impl SenderFactory {
    /// Constructs a new factory.
    pub fn new() -> Self {
        let (tx, rx) = bounded(2);
        let queue = Arc::new(SegQueue::new());

        Self {
            sender_tx: tx,
            sender_rx: rx,
            waiter_queue: queue
        }
    }

    /// Makes a new sending handle with the given factory channels and queue.
    pub fn make_handle(&self) -> DataSender {
        DataSender::new(
            self.sender_tx.clone(),
            self.waiter_queue.clone(),
        )
    }

    /// Receives data from any DataSenders that have submitted
    /// data to the channel.
    ///
    /// This also implicitly wakes up any waiters waiting on a notifying them
    /// that they can send to the handler again.
    pub fn recv(&self) -> Result<Payload, TryRecvError> {
        if self.waiter_queue.len() > 0 {
            Python::with_gil(|py| {
                while let Some(waker) = self.waiter_queue.pop() {
                    // The waker should not affect the reader
                    let _ = waker.call0(py);
                }
            });
        }
        self.sender_rx.try_recv()
    }
}