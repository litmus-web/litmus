use crate::pyre_server::{
    abc::{ProtocolBuffers, BaseTransport},
    switch::{Switchable, SwitchStatus},
    transport::Transport,
    parser::h1::extract_request,
};

use pyo3::PyResult;
use pyo3::exceptions::PyRuntimeError;
use bytes::BytesMut;
use crate::pyre_server::py_callback::CallbackHandler;


/// The protocol to add handling for the HTTP/1.x protocol.
pub struct H1Protocol {
    /// A possible Transport struct, this can be None if the protocol
    /// is not initialised before it starts handling interactions but this
    /// should never happen.
    maybe_transport: Option<Transport>,

    /// The python callback handler.
    callback: CallbackHandler,
}

impl H1Protocol {
    /// Create a new H1Protocol instance.
    pub fn new(callback: CallbackHandler) -> Self {
        Self {
            maybe_transport: None,
            callback,
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
        extract_request(buffer);
        self.callback.invoke((1,))?;
        buffer.clear();
        self.transport()?.resume_writing()?;
        Ok(())
    }

    fn fill_write_buffer(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        buffer.extend_from_slice("HTTP/1.1 200 OK\r\n\
        Content-Length: 13\r\n\
        Server: Pyre\r\n\r\nHello, World!".as_bytes());

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
