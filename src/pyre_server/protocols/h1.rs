use crate::pyre_server::{
    abc::{SocketCommunicator, BaseTransport},
    switch::{Switchable, SwitchStatus},
    transport::Transport,
};

use pyo3::PyResult;
use pyo3::exceptions::PyRuntimeError;
use bytes::{BytesMut, BufMut};

const MAX_BUFFER_LIMIT: usize = 256 * 1024;


/// The protocol to add handling for the HTTP/1.x protocol.
pub struct H1Protocol {
    maybe_transport: Option<Transport>,

    writer_buffer: BytesMut,
    reader_buffer: BytesMut,
}

impl H1Protocol {
    /// Create a new H1Protocol instance.
    pub fn new() -> Self {
        let buff1 = BytesMut::with_capacity(MAX_BUFFER_LIMIT);
        let buff2 = BytesMut::with_capacity(MAX_BUFFER_LIMIT);

        Self {
            maybe_transport: None,
            writer_buffer: buff1,
            reader_buffer: buff2,
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

impl SocketCommunicator for H1Protocol {
    fn read_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        return Ok(&mut self.reader_buffer)
    }

    fn read_buffer_filled(&mut self, amount: usize) -> PyResult<()> {
        if (amount >= MAX_BUFFER_LIMIT) | (amount == 0) {
            self.transport()?.pause_reading()?;
        }

        println!("Buffer filled, {}", amount);
        println!("{:?}", self.reader_buffer);

        self.transport()?.pause_reading()?;

        Ok(())
    }

    fn write_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        return Ok(&mut self.writer_buffer)
    }

    fn write_buffer_drained(&mut self, _amount: usize) -> PyResult<()> {
        if self.writer_buffer.len() == 0 {
            self.transport()?.pause_writing()?;
        }

        Ok(())
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
