use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyBlockingIOError};

use std::sync::Arc;
use crossbeam::queue::SegQueue;
use crossbeam::channel::{
    Sender,
    Receiver,
    bounded,
    TryRecvError,
    TrySendError,
};


use crate::pyre_server::responders::{Payload, WakerQueue};


/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataReceiver {
    /// The receiver half for receiving the client body chunks.
    rx: Receiver<Payload>,

    /// A queue of waiting events to invoke before the body
    /// can be read from the receiver again.
    waiter_queue: WakerQueue,
}

impl DataReceiver {
    /// Create a new handler with the given sender.
    pub fn new(rx: Receiver<Payload>, waiter_queue: WakerQueue) -> Self {
        Self { rx, waiter_queue }
    }
}

#[pymethods]
impl DataReceiver {
    /// Receives a chunk of data from the socket without blocking.
    ///
    /// Invoked by python passing more_body which represents if there
    /// is any more body to expect or not, and the body itself.
    ///
    /// Returns:
    ///     A tuple containing a boolean and a set of bytes, the boolean signals
    ///     if there is more data to be read from the socket or not and the
    ///     bytes returned are what contain the actual data.
    ///
    /// Raises:
    ///     RuntimeError:
    ///         If the channel the receiver uses to communicate with the main
    ///         socket handler is closed.
    ///
    ///     BlockingIOError:
    ///         The receiver is empty and would block waiting for data to be
    ///         sent to the receiver. In the event that this error is raised
    ///         the handler should set a waker in order to be notified when
    ///         data is available.
    #[call]
    fn __call__(&self) -> PyResult<(bool, Vec<u8>)> {
        let resp = self.rx.try_recv();

        return match resp {
            Ok(values) => {
                Ok(values)
            },
            Err(TryRecvError::Disconnected) => {
                Err(PyRuntimeError::new_err(
                    "receiving channel was unexpectedly closed."
                ))
            },
            Err(TryRecvError::Empty) => {
                Err(PyBlockingIOError::new_err(()))
            }
        }
    }
}


/// A factory / manager for receiver handles sending data from the server
/// handler to the Python callbacks.
pub struct ReceiverFactory {
    /// The sender half for sending the client body chunks.
    receiver_tx: Sender<Payload>,

    /// The receiver half for receiving the client body chunks.
    receiver_rx: Receiver<Payload>,

    /// A queue of waiting events to invoke before the body
    /// can be read from the receiver again.
    waiter_queue: WakerQueue,
}

impl ReceiverFactory {
    /// Constructs a new factory.
    pub fn new() -> Self {
        let (tx, rx) = bounded(10);
        let queue = Arc::new(SegQueue::new());

        Self {
            receiver_tx: tx,
            receiver_rx: rx,
            waiter_queue: queue,
        }
    }

    /// Makes a new sending handle with the given factory channels and queue.
    pub fn make_handle(&self) -> DataReceiver {
        DataReceiver::new(
            self.receiver_rx.clone(),
            self.waiter_queue.clone(),
        )
    }

    /// Sends the given payload to the handler channel.
    ///
    /// This implicitly wakes up any waiters waiting for a chunk of data
    /// from the handler, unlike the receiver version of this responder
    /// this will only pop one waiter from the queue and pass it the chunk
    /// of data vs waking all waiters.
    pub fn send(&self, data: Payload) -> Result<(), TrySendError<Payload>> {
        if self.waiter_queue.len() > 0 {
            Python::with_gil(|py| {
                if let Some(waker) = self.waiter_queue.pop() {
                    // The waker should not affect the writer
                    let _ = waker.call1(py, data);
                }
            });
            Ok(())
        } else {
            self.receiver_tx.try_send(data)
        }
    }
}