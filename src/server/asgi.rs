///
/// The asgi.rs file contains everything needed for interacting with the ASGI
/// interface on Python's side communicating with the RustProtocols.
///
/// CHANGE_LOG:
///     29/10/2020 - Add ASGIRunner struct with `new()` factory.
///
///     29/10/2020 - Add `Send` and `Receive` structs for callbacks.
///
/// TO_ADD:
///     - All of the ASGI stuff.
///

use pyo3::prelude::*;
use pyo3::{PyAsyncProtocol, PyIterProtocol, exceptions};
use pyo3::iter::IterNextOutput;
use pyo3::types::PyBytes;

use std::sync::{mpsc, Arc};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use bytes::{BytesMut, Bytes};

use crate::server::flow_control::FlowControl;


const HTTP_DISCONNECT_TYPE: &'static str = "http.disconnect";
const HTTP_BODY_TYPE: &'static str = "http.request";


#[pyclass]
pub struct ASGIRunner {
    callback: PyObject,
    transport: Arc<PyObject>,
    future: PyObject,  // the actual awaited task

    method: String,
    raw_path: Bytes,
    headers: HashMap<Bytes, Bytes>,

    flow_control: Arc<FlowControl>,
}

impl ASGIRunner {
    pub fn new(
        py: Python,
        callback: PyObject,
        transport: Arc<PyObject>,

        method: String,
        raw_path: Bytes,
        headers: HashMap<Bytes, Bytes>,

        flow_control: Arc<FlowControl>,
        more_body: Arc<AtomicBool>,
        receiver: mpsc::Receiver<BytesMut>,
    ) -> PyResult<Self> {

        let send = SendStart::new(
            flow_control.clone(),
            transport.clone(),
        );
        let receive = Receive::new(
            flow_control.clone(),
            transport.clone(),
            receiver,
            more_body,
        );

        let future = callback.call1(py,(send, receive))?;

        Ok(ASGIRunner {
            // set systems
            callback,
            transport,
            future,

            // Parsed details
            method,
            raw_path,
            headers,

            // Body streamer
            flow_control,
        })
    }
}

#[pyproto]
impl PyAsyncProtocol for ASGIRunner {
    fn __await__(slf: PyRef<Self>) -> PyResult<PyObject> {
        let awaited = slf
            .future
            .call_method0(slf.py(), "__await__")?;

        Ok(awaited)
    }
}


/// The ASGI send() awaitable, this is what writes content to the socket
/// taking a dictionary that gets converted to a struct.
///
/// What we're essentially recreating is a Python instance method callback
/// where the struct is aware of previously defined parameters and can then
/// be called to handle the data it is given.
///
/// This is why we define both `SendStart::new()` and `SendStart.__call__()` as
/// a pymethod giving us the same effect as a instance method callback.
///
#[pyclass]
struct SendStart {
    flow_control: Arc<FlowControl>,
    transport: Arc<PyObject>,
    data: Option<PyObject>,
}

impl SendStart {
    fn new(
        flow_control: Arc<FlowControl>,
        transport: Arc<PyObject>,
    ) -> Self {
        SendStart {
            flow_control,
            transport,
            data: None,
        }
    }
}

#[pymethods]
impl SendStart {
    #[call]
    fn __call__(
        mut slf: PyRefMut<Self>,
        data: PyObject,
    ) -> PyResult<PyRefMut<Self>> {
        slf.data = Some(data);

        Ok(slf)
    }
}

#[pyproto]
impl PyAsyncProtocol for SendStart {
    fn __await__(mut slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
}

#[pyproto]
impl PyIterProtocol for SendStart {
    fn __next__(
        mut slf: PyRefMut<Self>
    ) -> PyResult<IterNextOutput<Option<PyObject>, Option<PyObject>>> {
        // Check so we dont screw everything if it's not initialised.
        if !slf.data.is_none() {
            return Err(exceptions::PyValueError::new_err(
                "parameter 'data' was type 'None' at point of iteration."
            ))
        }


        Ok(IterNextOutput::Return(None))
    }
}


/// The receive() awaitable that handles receiving data from the
/// server, this uses a channel and atomic bool to communicate
/// with the protocol handling the actual read.
///
/// What we're essentially recreating is a Python instance method callback;
/// where the struct is aware of previously defined parameters and can then
/// be called to handle actually reading the data stream (In this case we
/// use a set of non-blocking channel receives to interact with the protocol)
///
/// This is why we define both `Receive::new()` and `Receive.__call__()` as a
/// pymethod giving us the same effect as a instance method callback.
///
#[pyclass]
struct Receive {
    flow_control: Arc<FlowControl>,
    transport: Arc<PyObject>,
    receiver: mpsc::Receiver<BytesMut>,
    more_body: Arc<AtomicBool>,
    pending: bool,
}

impl Receive {
    fn new(
        flow_control: Arc<FlowControl>,
        transport: Arc<PyObject>,
        receiver: mpsc::Receiver<BytesMut>,
        more_body: Arc<AtomicBool>,
    ) -> Self {
        Receive {
            flow_control,
            transport,
            receiver,
            more_body,
            pending: false,
        }
    }
}

#[pymethods]
impl Receive {
    #[call]
    fn __call__(mut slf: &PyCell<Self>) -> &PyCell<Self> {
        slf
    }

}

#[pyproto]
impl PyAsyncProtocol for Receive {
    fn __await__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
}

#[pyproto]
impl PyIterProtocol for Receive {
    fn __next__(
        mut slf: PyRefMut<Self>
    ) -> PyResult<IterNextOutput<
        Option<PyObject>,
        (&'static str, Py<PyBytes>, bool)
    >> {
        // todo: This is iterating more than it should meaning
        // the body is being sent to the channel more than it should
        // i have a suspicion we should fill the buffer until it hits
        // high water however i am not sure so do check out.
        //
        // currently Pyre is timed at 70micros per iteration while
        // uvicorn is timed at 220micros which is 3x slower *nice*
        //
        // okay so we worked out what caused this -> We're being give 32KiB
        // chunks of data compared to Uvicorn's 256
        // yikes


        // If the client has disconnected or we've completed a response todo: Add response check
        if slf.flow_control.disconnected {
            let py_bytes = PyBytes::new(slf.py(), "".as_bytes());
            Ok(IterNextOutput::Return((
                HTTP_BODY_TYPE,
                Py::from(py_bytes),
                false,
            )))
        }

        if !slf.pending {
            slf.flow_control.resume_reading(slf.py())?;
            slf.pending = true;
        }


        let body = match slf.receiver.try_recv() {
            Ok(data) => data,
            Err(_) => return Ok(IterNextOutput::Yield(None)),
        };

        slf.pending = false;


        let py_bytes = PyBytes::new(slf.py(), body.as_ref());
        Ok(IterNextOutput::Return((
            HTTP_BODY_TYPE,
            Py::from(py_bytes),
            slf.more_body.load(Relaxed)
        )))
    }
}