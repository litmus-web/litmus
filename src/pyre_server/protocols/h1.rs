// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use crate::pyre_server::abc::{ProtocolBuffers, BaseTransport};
use crate::pyre_server::switch::{Switchable, SwitchStatus};
use crate::pyre_server::transport::Transport;
use crate::pyre_server::py_callback::CallbackHandler;
use crate::pyre_server::responders::data_callback::{DataSender, SenderPayload};

use pyo3::PyResult;
use pyo3::exceptions::PyRuntimeError;

use std::mem;
use std::sync::Arc;

use bytes::{BytesMut, Bytes};

use crossbeam::channel::{Sender, Receiver, unbounded};

use httparse::{Status, parse_chunk_size, Header, Request};
use http::version::Version;


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

    /// The sender half for sending body chunks.
    tx: Sender<SenderPayload>,

    /// The receiver half for sending body chunks.
    rx: Receiver<SenderPayload>,
}

impl H1Protocol {
    /// Create a new H1Protocol instance.
    pub fn new(callback: CallbackHandler) -> Self {
        let (tx, rx) = unbounded();
        Self {
            maybe_transport: None,
            callback,
            tx,
            rx,
        }
    }

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
    fn data_received(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        // This should be fine as it is guaranteed to be initialised
        // before we use it, just waiting for the ability to use
        // MaybeUninit, till then here we are.
        let mut headers: [Header<'_>; MAX_HEADERS] = unsafe {
            mem::uninitialized()
        };

        let body = buffer.clone();

        let mut request = Request::new(&mut headers);
        let len = match request.parse(&body) {
            Ok(status) => status.unwrap(),
            Err(e) => return Err(PyRuntimeError::new_err(format!(
                "{:?}", e  // todo remove this, add custom http response.
            )))
        };

        let _ = buffer.split_to(len);

        self.on_request_parse(&mut request);

        self.transport()?.resume_writing()?;
        Ok(())
    }

    fn fill_write_buffer(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        while let Ok((_more_body, buff)) = self.rx.try_recv() {
            buffer.extend(buff);
        }

        Ok(())
    }

    fn writing_paused(&mut self) -> PyResult<()> {
        Ok(())
    }

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
    fn on_request_parse(&self, request: &mut Request) -> PyResult<()> {
        let method = request.method
            .expect("Value was None at complete parse");
        let path = request.path
            .expect("Value was None at complete parse");
        let version = request.version
            .expect("Value was None at complete parse");


        let responder = DataSender::new(self.tx.clone());
        self.callback.invoke((responder,))?;

        Ok(())
    }
}