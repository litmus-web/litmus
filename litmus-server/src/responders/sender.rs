use pyo3::exceptions::PyBlockingIOError;
use pyo3::prelude::*;

use crossbeam::channel::{bounded, Receiver, Sender, TryRecvError, TrySendError};
use crossbeam::queue::SegQueue;

use std::sync::Arc;

use super::{SenderPayload, WakerQueue};

const HEADER_SEPARATOR: &[u8] = ": ".as_bytes();
const LINE_SEPARATOR: &[u8] = "\r\n".as_bytes();
const SERVER_HEADER: &[u8] = "server: Litmus".as_bytes();

/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataSender {
    /// The sending half of the channel used for sending data to the
    /// server handler.
    tx: Sender<SenderPayload>,

    /// A queue of waiting events to invoke before the body
    /// can be written to again.
    waiter_queue: WakerQueue,

    /// If the response is using chunked encoding or not or not set.
    chunked_encoding: Option<bool>,

    /// If the client has defined a given content length of the body.
    expected_content_length: usize,
}

impl DataSender {
    /// Create a new handler with the given sender.
    pub fn new(tx: Sender<SenderPayload>, waiter_queue: WakerQueue) -> Self {
        let chunked_encoding = None; // We expect nothing yet.
        let expected_content_length: usize = 0; // We expect nothing yet.

        Self {
            tx,
            waiter_queue,
            chunked_encoding,
            expected_content_length,
        }
    }
}

#[pymethods]
impl DataSender {
    /// Sends the a chunk of the main body to the handler.
    ///
    /// This raises a `BlockingIoError` if the queue / buffer is full, the
    /// invoker should wait till the queue / buffer is no longer full.
    ///
    /// Args:
    ///     more_body:
    ///         A boolean to determine if the server should expect any more
    ///         chunks of body being sent or if the request is regarded as
    ///         being 'complete'.
    ///
    ///     body:
    ///         A chunk of bytes to be written to the socket.
    fn send_body(&self, more_body: bool, body: Vec<u8>) -> PyResult<()> {
        if self.expected_content_length == 0 {
            return Ok(());
        }

        let payload = (more_body, true, body);
        if let Err(e) = self.tx.try_send(payload) {
            if let TrySendError::Full(_) = e {
                return Err(PyBlockingIOError::new_err(()));
            }

            // The connection has been dropped, ignore.
            return Ok(());
        }

        Ok(())
    }

    /// Sends the start of the response body to the handler.
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
    fn send_start(&mut self, status_code: u16, resp_headers: Vec<(&[u8], &[u8])>) -> PyResult<()> {
        let mut keep_alive = true;
        let mut out = Vec::with_capacity(resp_headers.len() + 4);

        let status = match http::StatusCode::from_u16(status_code) {
            Ok(s) => s,
            Err(_) => panic!("invalid status code given"),
        };
        let status_block = format!(
            "HTTP/1.1 {} {}",
            status.as_str(),
            status.canonical_reason().unwrap_or_else(|| ""),
        )
        .as_bytes()
        .to_vec();
        out.push(status_block);

        for (name, value) in resp_headers {
            let name = match headers::HeaderName::from_bytes(name) {
                Ok(s) => s,
                Err(_) => panic!("invalid header name given"),
            };

            let value = match headers::HeaderValue::from_bytes(value) {
                Ok(s) => s,
                Err(_) => panic!("invalid status code given"),
            };

            match &name {
                &http::header::CONTENT_LENGTH => {
                    self.expected_content_length = value
                        .to_str()
                        .expect("content length header is not ASCII encodable")
                        .parse()
                        .expect("content length header contains invalid integer")
                }
                &http::header::TRANSFER_ENCODING => {
                    let temp_val = value.as_ref();
                    if temp_val.len() == 7 {
                        // This compares each explicit character,
                        // in this case a for loop is a lot of extra overhead
                        // for no reason and can lead to attacks.
                        if (temp_val[0] == 99) &   // c
                        (temp_val[1] == 104) &  // h
                        (temp_val[2] == 117) &  // u
                        (temp_val[3] == 110) &  // n
                        (temp_val[4] == 107) &  // k
                        (temp_val[5] == 101) &  // e
                        (temp_val[6] == 100)
                        // d
                        {
                            self.chunked_encoding = Some(true);
                        }
                    };
                }
                &http::header::CONNECTION => {
                    let temp_val = value.as_ref();
                    if temp_val.len() == 5 {
                        if (temp_val[0] == 99) &   // c
                        (temp_val[1] == 108) &  // l
                        (temp_val[2] == 111) &  // o
                        (temp_val[3] == 115) &  // s
                        (temp_val[4] == 101)
                        // e
                        {
                            keep_alive = false;
                        }
                    }
                }
                _ => {}
            }

            let res = [name.as_ref(), value.as_bytes()].join(HEADER_SEPARATOR);

            out.push(res);
        }

        let formatted_date_header = format!(
            "date: {}",
            httpdate::fmt_http_date(std::time::SystemTime::now()),
        )
        .as_bytes()
        .to_vec();
        out.push(formatted_date_header); // Date
        out.push(SERVER_HEADER.to_vec()); // Server
        out.push(LINE_SEPARATOR.to_vec()); // End of Headers

        // Joins all separate lines into a single block with \r\n joining them.
        let start_block = out.join(LINE_SEPARATOR);

        return match self.tx.try_send((true, keep_alive, start_block)) {
            Err(TrySendError::Full(_)) => Err(PyBlockingIOError::new_err(())),
            _ => Ok(()),
        };
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
    sender_tx: Sender<SenderPayload>,

    /// The receiver half for receiving body chunks.
    sender_rx: Receiver<SenderPayload>,

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
            waiter_queue: queue,
        }
    }

    /// Makes a new sending handle with the given factory channels and queue.
    pub fn make_handle(&self) -> DataSender {
        DataSender::new(self.sender_tx.clone(), self.waiter_queue.clone())
    }

    /// Receives data from any DataSenders that have submitted
    /// data to the channel.
    ///
    /// This also implicitly wakes up any waiters waiting on a notifying them
    /// that they can send to the handler again.
    pub fn recv(&self) -> Result<SenderPayload, TryRecvError> {
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
