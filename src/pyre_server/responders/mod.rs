use std::sync::Arc;
use pyo3::{PyObject, Py};
use pyo3::types::PyBytes;
use crossbeam::queue::SegQueue;

pub mod sender;
pub mod receiver;

/// The payload that gets sent to the receiver half of the channel.
///
/// Types equate to: more_body, keep_alive, body.
pub type SenderPayload = (bool, bool, Vec<u8>);

/// The payload that gets sent to the receiver half of the channel.
pub type ReceiverPayload = (bool, Py<PyBytes>);

/// The queue of Python waiters to be woken up on a given event.
pub(crate) type WakerQueue = Arc<SegQueue<PyObject>>;