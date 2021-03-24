// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use pyo3::{PyResult, Python, Py};
use pyo3::types::PyBytes;
use pyo3::exceptions::PyRuntimeError;

use std::mem;
use std::str;

use bytes::BytesMut;

use httparse::{Header, Request, Status, parse_chunk_size};
use http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use http::uri::Uri;

use crate::pyre_server::abc::{ProtocolBuffers, BaseTransport};
use crate::pyre_server::switch::{Switchable, SwitchStatus};
use crate::pyre_server::transport::Transport;
use crate::pyre_server::py_callback::CallbackHandler;
use crate::pyre_server::responders::sender::SenderFactory;
use crate::pyre_server::responders::receiver::ReceiverFactory;
use crate::pyre_server::settings::Settings;
use crate::pyre_server::psgi;

macro_rules! conv_err {
    ( $e:expr ) => ( $e.map_err(|e| PyRuntimeError::new_err(format!("{}", e))) )
}

/// The max headers allowed in a single request.
const MAX_HEADERS: usize = 100;

/// The minimum amount the buffer needs to be filled by before a body is sent.
const MIN_BUFF_SIZE: usize = 64 * 1024;

const FORGIVING_BUFFER_SIZE: usize = 128 * 1024;

/// The protocol to add handling for the HTTP/1.x protocol.
pub struct H1Protocol {
    /// A possible Transport struct, this can be None if the protocol
    /// is not initialised before it starts handling interactions but this
    /// should never happen.
    maybe_transport: Option<Transport>,

    /// The server configuration used to construct a ASGI scope.
    settings: Settings,

    /// The python callback handler.
    callback: CallbackHandler,

    /// The sender half handler for ASGI callbacks.
    sender: SenderFactory,

    /// The receiver half handler for ASGI callbacks.
    receiver: ReceiverFactory,

    /// The length of the body either as a chunk or the whole
    /// length depending on if the request uses chunked encoding or not.
    expected_content_length: usize,

    /// If the request uses chunked encoding for it's body.
    chunked_encoding: bool,
}

impl H1Protocol {
    /// Create a new H1Protocol instance.
    pub fn new(settings: Settings, callback: CallbackHandler) -> Self {
        let sender = SenderFactory::new();
        let receiver = ReceiverFactory::new();

        Self {
            maybe_transport: None,

            settings,
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
                "transport was None upon being called"
            ))
        }
    }
}

impl H1Protocol {
    /// Called when the protocol is in charge of a new socket / handle,
    /// the `Transport` can be used to pause and resume reading from this
    /// socket.
    pub fn new_connection(&mut self, transport: Transport) -> PyResult<()> {
        self.reset_state();
        self.maybe_transport = Some(transport);
        Ok(())
    }

    /// Called when the connection is lost from the protocol in order to
    /// properly reset state.
    pub fn lost_connection(&mut self) -> PyResult<()> {
        Ok(())
    }

    /// Resets the internal state of the protocol for handling a new
    /// connection.
    fn reset_state(&mut self) {
        self.expected_content_length = 0;
        self.chunked_encoding = false;

        self.sender = SenderFactory::new();
        self.receiver = ReceiverFactory::new();
    }
}

impl ProtocolBuffers for H1Protocol {
    /// Parses data received from the given buffer and separates the content
    /// accordingly.
    ///
    /// This callback is invoked just after the auto protocol's buffer
    /// has been acquired and filled. It's not guaranteed to be filled
    /// fully but it is guaranteed to have at least been filled up with
    /// one or more bytes of data.
    ///
    /// Upon no data being read signalling a EOF the eof_received callback is
    /// invoked and handled instead.
    fn data_received(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        if self.expected_content_length == 0 {
            self.parser_request(buffer)?;
        }

        if self.chunked_encoding {
            self.parse_chunked_body(buffer)?;
        } else if self.expected_content_length > 0 {
            self.parse_body(buffer)?;
        } else {
            let _ = self.receiver.send((false, Vec::new()));
        }

        self.transport()?.resume_writing()?;
        Ok(())
    }

    /// Fills the passed buffer with any messages enqueued to be sent.
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
    fn parser_request(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        // This should be fine as it is guaranteed to be initialised
        // before we use it, just waiting for the ability to use
        // MaybeUninit, till then here we are.
        let mut headers: [Header<'_>; MAX_HEADERS] = unsafe {
            mem::uninitialized()
        };

        let body = buffer.clone();

        let mut request = Request::new(&mut headers);
        let status = conv_err!(request.parse(&body))?;

        let len= if status.is_partial() {
            return Ok(())
        } else {
            status.unwrap()
        };

        let _ = buffer.split_to(len);

        self.on_request_parse(&mut request)?;

        Ok(())
    }

    fn parse_chunked_body(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        if let Some((more_body, data)) = self.drain_body_chunks(buffer)? {
            let _ = self.receiver.send((more_body, data.to_vec()));
        }

        Ok(())
    }

    fn drain_body_chunks(
        &mut self,
        buffer: &mut BytesMut,
    ) -> PyResult<Option<(bool, BytesMut)>> {

        let mut temp_buff = BytesMut::with_capacity(FORGIVING_BUFFER_SIZE);
        loop {
            let res = conv_err!(parse_chunk_size(&buffer))?;
            let (start, len) = match res {
                Status::Complete(info) => info,
                Status::Partial => {
                    return if temp_buff.len() <= MIN_BUFF_SIZE {
                        Ok(None)
                    } else {
                        Ok(Some((true, temp_buff)))
                    }

                },
            };

            if len == 0 {
                let _ = buffer.split_to(start + 4);
                return Ok(Some((false, temp_buff)))
            }

            let _ = buffer.split_to(start);
            let body = buffer.split_to(len as usize);
            let _ = buffer.split_to(2); // remove \r\n suffix

            temp_buff.extend(body);

            if temp_buff.len() >= MIN_BUFF_SIZE {
                return Ok(Some((true, temp_buff)))
            }
        }
    }

    fn parse_body(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        let (
            more_body,
            data,
        ) = if buffer.len() >= self.expected_content_length {
            let res = buffer.split_to(self.expected_content_length);
            (false, Some(res))
        } else if buffer.len() >= MIN_BUFF_SIZE {
            let res = buffer.clone();
            buffer.clear();
            self.expected_content_length -= res.len();
            (true, Some(res))
        } else {
            (true, None)
        };

        if let Some(data) = data {
            let _ = self.receiver.send((more_body, data.to_vec()));
        }

        Ok(())
    }

    /// Turns all the headers into Python type objects and invokes the
    /// python callback.
    fn on_request_parse(&mut self, request: &mut Request) -> PyResult<()> {
        let method = request.method
            .expect("Method was None at complete parse");
        let path = request.path
            .expect("Path was None at complete parse");
        let version = request.version
            .expect("Version was None at complete parse");

        let version = if version == 0 {
            psgi::HTTP_10
        } else if version == 1 {
            psgi::HTTP_11
        } else {
            unreachable!()
        };

        let uri = path.parse::<Uri>()
                .expect("failed to parse http url");

        let headers_new = Python::with_gil(|py| {
            let mut parsed_vec = Vec::with_capacity(request.headers.len());
            for header in request.headers.iter() {
                self.check_header(&header);

                let converted2: Py<PyBytes> = Py::from(PyBytes::new(
                    py,
                    header.value
                ));
                parsed_vec.push(( header.name, converted2))
            }

            parsed_vec
        });

        let server = (
            self.settings.server_addr.ip().to_string(),
            self.settings.server_addr.port(),
        );

        let client = (
            self.transport()?.client.ip().to_string(),
            self.transport()?.client.port(),
        );

        let scope: psgi::PSGIScope = (
            psgi::SCOPE_TYPE,
            version,
            method,
            self.settings.schema.as_str(),
            uri.path(),
            uri.query().unwrap_or(""),
            psgi::TEMP_ROOT_PATH,
            headers_new,
            server,
            client,
        );

        let sender = self.sender.make_handle();
        let receiver = self.receiver.make_handle();
        self.callback.invoke((
            scope,
            sender,
            receiver,
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