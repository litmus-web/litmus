use pyo3::prelude::*;
use pyo3::types::PyBytes;

use std::collections::HashMap;
use std::sync::{Arc, mpsc};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use bytes::{Bytes, BytesMut};
use pyo3::{PyAsyncProtocol, PyIterProtocol};
use pyo3::iter::IterNextOutput;

use crate::server::flow_control::FlowControl;
use crate::http;
use crate::asyncio;

const HTTP_DISCONNECT_TYPE: &'static str = "http.disconnect";
const HTTP_BODY_TYPE: &'static str = "http.request";


#[pyclass]
pub struct ASGIRunner {
    callback: PyObject,
    future: PyObject,  // the actual awaited task

    method: String,
    raw_path: Bytes,
    headers: HashMap<Bytes, Bytes>,
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
            future,

            // Parsed details
            method,
            raw_path,
            headers,
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

    status: u16,
    headers: Vec<(Vec<u8>, Vec<u8>)>,
    body: BytesMut,
    more_body: bool,

    start_complete: bool,

    state: usize,
}

impl SendStart {
    fn new(
        flow_control: Arc<FlowControl>,
        transport: Arc<PyObject>,
    ) -> Self {
        SendStart {
            flow_control,
            transport,

            status: 0,
            headers: Vec::new(),
            body: BytesMut::new(),
            more_body: true,
            start_complete: false,

            state: 0
        }
    }
}

#[pymethods]
impl SendStart {
    #[call]
    fn __call__(
        mut slf: PyRefMut<Self>,
        status: u16,
        headers: Vec<(Vec<u8>, Vec<u8>)>,
        data: Vec<u8>,
        more_body: bool,
    ) -> PyResult<PyRefMut<Self>> {

        slf.status = status;
        slf.headers = headers;
        slf.body.extend(data);
        slf.more_body = more_body;

        Ok(slf)
    }
}

#[pyproto]
impl PyAsyncProtocol for SendStart {
    fn __await__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
}

#[pyproto]
impl PyIterProtocol for SendStart {
    fn __next__(
        mut slf: PyRefMut<Self>
    ) -> PyResult<IterNextOutput<Option<PyObject>, Option<PyObject>>> {

        if (slf.status != 0) & !slf.start_complete {
            let body = http::format_response_start(
                slf.status.clone(),
                slf.headers.clone(),
            )?;

            asyncio::write_transport(
                slf.py(),
                &slf.transport,
                body.as_ref()
            )?;

            slf.start_complete = true;
            return Ok(IterNextOutput::Return(None))
        }

        asyncio::write_transport(
            slf.py(),
            &slf.transport,
            slf.body.as_ref()
        )?;

        asyncio::close_transport(slf.py(), &slf.transport)?;
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

    response_complete: bool,
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
            response_complete: false,
        }
    }
}

#[pymethods]
impl Receive {
    #[call]
    fn __call__(slf: &PyCell<Self>) -> &PyCell<Self> {
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

        // If the client has disconnected or we've completed a response todo: Add response check
        if slf.flow_control.disconnected.load(Relaxed) | slf.response_complete {
            let py_bytes = PyBytes::new(slf.py(), "".as_bytes());
            return Ok(IterNextOutput::Return((
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

        Ok(IterNextOutput::Return((
            HTTP_BODY_TYPE,
            Py::from(PyBytes::new(slf.py(), body.as_ref())),
            slf.more_body.load(Relaxed)
        )))
    }
}