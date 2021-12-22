use std::sync::Arc;

use crossbeam::queue::SegQueue;
use pyo3::types::PyBytes;
use pyo3::{Py, PyObject};

mod receiver;
mod sender;

pub use receiver::{DataReceiver, ReceiverFactory};
pub use sender::{DataSender, SenderFactory};

/// The payload that gets sent to the receiver half of the channel.
///
/// Types equate to: more_body, keep_alive, body.
pub type SenderPayload = (bool, bool, Vec<u8>);

/// The payload that gets sent to the receiver half of the channel.
pub type ReceiverPayload = (bool, Py<PyBytes>);

/// The queue of Python waiters to be woken up on a given event.
pub(crate) type WakerQueue = Arc<SegQueue<PyObject>>;
