use crate::pyre_server::{
    abc::{ProtocolBuffers, BaseTransport},
    switch::{Switchable, SwitchStatus},
    transport::Transport,
};

use pyo3::PyResult;
use pyo3::exceptions::PyRuntimeError;
use bytes::BytesMut;


/// The protocol to add handling for the HTTP/1.x protocol.
pub struct H1Protocol {
    maybe_transport: Option<Transport>,
}

impl H1Protocol {
    /// Create a new H1Protocol instance.
    pub fn new() -> Self {

        Self {
            maybe_transport: None,
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
    pub fn new_connection(&mut self, transport: Transport) -> PyResult<()> {
        println!("Got transport");
        self.maybe_transport = Some(transport);
        Ok(())
    }

    pub fn lost_connection(&mut self) -> PyResult<()> {
        unimplemented!()
    }
}

impl ProtocolBuffers for H1Protocol {
    fn data_received(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        unimplemented!()
    }

    fn fill_write_buffer(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        unimplemented!()
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
