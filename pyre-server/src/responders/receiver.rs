use pyo3::exceptions::{PyBlockingIOError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use bytes::BytesMut;
use crossbeam::channel::{bounded, Receiver, Sender, TryRecvError, TrySendError};
use crossbeam::queue::SegQueue;
use std::sync::Arc;

use super::{ReceiverPayload, WakerQueue};

/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataReceiver {
    /// The receiver half for receiving the client body chunks.
    rx: Receiver<ReceiverPayload>,

    /// A queue of waiting events to invoke before the body
    /// can be read from the receiver again.
    waiter_queue: WakerQueue,
}

impl DataReceiver {
    /// Create a new handler with the given sender.
    pub fn new(rx: Receiver<ReceiverPayload>, waiter_queue: WakerQueue) -> Self {
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
    fn __call__(&self) -> PyResult<(bool, Py<PyBytes>)> {
        let resp = self.rx.try_recv();

        return match resp {
            Ok(values) => Ok(values),
            Err(TryRecvError::Disconnected) => Err(PyRuntimeError::new_err(
                "receiving channel was unexpectedly closed.",
            )),
            Err(TryRecvError::Empty) => Err(PyBlockingIOError::new_err(())),
        };
    }

    /// Submits a given callback to the waiter queue.
    ///
    /// Any waiters in the queue when the socket is able to be read from will
    /// be taken out of the queue and invoked signalling the system's ability
    /// to be read from again.
    ///
    /// All waker callbacks are invoked with no parameters or key word
    /// arguments and are expected not to directly raise an error, in the case
    /// that a waker does raise an error the exception is ignored and
    /// implicitly silenced.
    ///
    /// Args:
    ///     waker:
    ///         A callback to be invoked when data can be read from the socket
    ///         without blocking.
    fn subscribe(&self, waker: PyObject) {
        self.waiter_queue.push(waker);
    }
}

/// A factory / manager for receiver handles sending data from the server
/// handler to the Python callbacks.
pub struct ReceiverFactory {
    /// The sender half for sending the client body chunks.
    receiver_tx: Sender<ReceiverPayload>,

    /// The receiver half for receiving the client body chunks.
    receiver_rx: Receiver<ReceiverPayload>,

    /// A queue of waiting events to invoke before the body
    /// can be read from the receiver again.
    waiter_queue: WakerQueue,
}

impl ReceiverFactory {
    /// Constructs a new factory.
    pub fn new() -> Self {
        let (tx, rx) = bounded(2);
        let queue = Arc::new(SegQueue::new());

        Self {
            receiver_tx: tx,
            receiver_rx: rx,
            waiter_queue: queue,
        }
    }

    /// Makes a new sending handle with the given factory channels and queue.
    pub fn make_handle(&self) -> DataReceiver {
        DataReceiver::new(self.receiver_rx.clone(), self.waiter_queue.clone())
    }

    /// Sends the given payload to the handler channel.
    ///
    /// This implicitly wakes up any waiters waiting for a chunk of data
    /// from the handler, unlike the receiver version of this responder
    /// this will only pop one waiter from the queue and pass it the chunk
    /// of data vs waking all waiters.
    pub fn send(&self, data: (bool, BytesMut)) -> Result<(), TrySendError<ReceiverPayload>> {
        Python::with_gil(|py| {
            let bytes_body = unsafe { PyBytes::from_ptr(py, data.1.as_ptr(), data.1.len()) };
            let body = Py::from(bytes_body);

            if self.waiter_queue.len() > 0 {
                if let Some(waker) = self.waiter_queue.pop() {
                    // The waker should not affect the writer
                    let _ = waker.call1(py, (data.0, body));
                }
                Ok(())
            } else {
                self.receiver_tx.try_send((data.0, body))
            }
        })
    }
}
