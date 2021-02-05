// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use pyo3::{PyResult, Python, Py};
use pyo3::types::PyBytes;
use pyo3::exceptions::PyRuntimeError;

use std::mem;
use std::sync::Arc;
use std::str;

use bytes::{BytesMut, Bytes};

use crossbeam::channel::{Sender, Receiver, unbounded};

use httparse::{Status, parse_chunk_size, Header, Request};
use http::version::Version;
use http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};

use crate::pyre_server::abc::{ProtocolBuffers, BaseTransport};
use crate::pyre_server::switch::{Switchable, SwitchStatus};
use crate::pyre_server::transport::Transport;
use crate::pyre_server::py_callback::CallbackHandler;
use crate::pyre_server::responders::sender::SenderHandler;
use crate::pyre_server::responders::receiver::ReceiverHandler;


/// The max headers allowed in a single request.
const MAX_HEADERS: usize = 100;


/// The protocol to add handling for the HTTP/1.x protocol.
pub struct H1Protocol {
    /// A possible Transport struct, this can be None if the protocol
    /// is not initialised before it starts handling interactions but this
    /// should never happen.
    maybe_transport: Option<Transport>,

    /// The python callback handler.
    callback: CallbackHandler,

    /// The sender half handler for ASGI callbacks.
    sender: SenderHandler,

    /// The receiver half handler for ASGI callbacks.
    receiver: ReceiverHandler,

    /// The length of the body either as a chunk or the whole
    /// length depending on if the request uses chunked encoding or not.
    expected_content_length: usize,

    /// If the request uses chunked encoding for it's body.
    chunked_encoding: bool,
}

impl H1Protocol {
    /// Create a new H1Protocol instance.
    pub fn new(callback: CallbackHandler) -> Self {
        let sender = SenderHandler::new();
        let receiver = ReceiverHandler::new();

        Self {
            maybe_transport: None,
            callback,
            sender,
            receiver,

            expected_content_length: 0,
            chunked_encoding: false,
        }
    }

    /// Get the set transport or raise an error.
    ///
    /// This is mostly just a helper function to stop so many .unwrap()'s
    /// it also allows the user to be alerted in a more readable fashion
    /// instead of a panic! which to the user will be fairly unreadable and
    /// confusing.
    #[inline]
    fn transport(&self) -> PyResult<&Transport> {
        return if let Some(t) = self.maybe_transport.as_ref() {
            Ok(t)
        } else {
            Err(PyRuntimeError::new_err(
                "Transport was None upon being called."
            ))
        }
    }
}

impl H1Protocol {
    /// Called when the protocol is in charge of a new socket / handle,
    /// the `Transport` can be used to pause and resume reading from this
    /// socket.
    pub fn new_connection(&mut self, transport: Transport) -> PyResult<()> {
        self.maybe_transport = Some(transport);
        Ok(())
    }

    /// Called when the connection is lost from the protocol in order to
    /// properly reset state.
    pub fn lost_connection(&mut self) -> PyResult<()> {
        Ok(())
    }
}

impl ProtocolBuffers for H1Protocol {
    /// Parses data received from the given buffer and separates the content
    /// accordingly.
    ///
    /// This callback is invoked just after the auto protocol's buffer has
    /// been acquired and filled. It's not guaranteed to be filled but it is
    /// guaranteed to have at least been filled up with 1 or more bytes of data.
    ///
    /// Upon no data being read signalling a EOF the eof_received callback is
    /// invoked and handled instead.
    fn data_received(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        // This should be fine as it is guaranteed to be initialised
        // before we use it, just waiting for the ability to use
        // MaybeUninit, till then here we are.
        let mut headers: [Header<'_>; MAX_HEADERS] = unsafe {
            mem::uninitialized()
        };

        let body = buffer.clone();

        let mut request = Request::new(&mut headers);
        let status = match request.parse(&body) {
            Ok(status) => status,
            Err(e) => return Err(PyRuntimeError::new_err(format!(
                "{:?}", e  // todo remove this, add custom http response.
            )))
        };

        let len= if status.is_partial() {
            return Ok(())
        } else {
            status.unwrap()
        };

        let _ = buffer.split_to(len);

        self.on_request_parse(&mut request)?;

        self.transport()?.resume_writing()?;
        Ok(())
    }

    /// Fills the passed buffer with any messages enqueued to be sent.
    ///
    /// Todo: This needs to have some flot control added.
    fn fill_write_buffer(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        while let Ok((_more_body, buff)) = self.sender.recv() {
            buffer.extend(buff);
        }

        Ok(())
    }

    /// Pauses writing removing the event listeners to close the socket.
    fn eof_received(&mut self) -> PyResult<()> {
        self.transport()?.pause_reading()
    }
}

impl Switchable for H1Protocol {
    /// Determines what the protocol should be switched to if it is
    /// necessary called just after reading has completed to allow
    /// for upgrading.
    fn switch_protocol(&mut self) -> PyResult<SwitchStatus> {
        // ignore for now
        Ok(SwitchStatus::NoSwitch)
    }
}

impl H1Protocol {
    /// Turns all the headers into Python type objects and invokes the
    /// python callback.
    fn on_request_parse(&mut self, request: &mut Request) -> PyResult<()> {
        let method = request.method
            .expect("Method was None at complete parse");
        let path = request.path
            .expect("Path was None at complete parse");
        let version = request.version
            .expect("Version was None at complete parse");


        let headers_new = Python::with_gil(|py| {
            let mut parsed_vec = Vec::with_capacity(request.headers.len());
            for header in request.headers.iter() {
                self.check_header(&header);

                let converted: Py<PyBytes> = Py::from(PyBytes::new(py, header.value));
                parsed_vec.push((header.name, converted))
            }

            parsed_vec
        });


        let sender = self.sender.make_handle();
        let receiver = self.receiver.make_handle();
        self.callback.invoke((
            sender,
            receiver,
            headers_new,
            method,
            path,
            version,
        ))?;

        Ok(())
    }

    /// Checks a given header to see if it is to do with the request's
    /// body size and type, e.g. Chunked encoding.
    fn check_header(&mut self, header: &Header) {
        if header.name == CONTENT_LENGTH {
            self.expected_content_length = str::from_utf8(header.value)
                .map(|v| v.parse::<usize>().unwrap_or(0))
                .unwrap_or(0)
        } else if header.name == TRANSFER_ENCODING {
            let lowered = header.value.to_ascii_lowercase();
            self.chunked_encoding = str::from_utf8(lowered.as_ref())
                .map(|v| v.contains("chunked"))
                .unwrap_or(false)
        }
    }
}