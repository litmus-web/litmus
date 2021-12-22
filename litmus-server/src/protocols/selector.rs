use bytes::BytesMut;
use pyo3::PyResult;

use super::H1Protocol;
use crate::server::CallbackHandler;
use crate::settings::Settings;
use crate::traits::{BaseTransport, BufferHandler, ProtocolBuffers, SocketState};
use crate::transport::Transport;

const BUFFER_SIZE: usize = 32 * 1024;

#[derive(Copy, Clone)]
pub(crate) enum Protocols {
    H1,
    // H2,
    // WS,
}

#[allow(unused)]
pub(crate) enum SwitchStatus {
    SwitchTo(Protocols),
    NoSwitch,
}

pub(crate) struct AutoProtocol {
    transport: Transport,

    selected: Protocols,
    h1: H1Protocol,

    writer_buffer: BytesMut,
    reader_buffer: BytesMut,
}

impl AutoProtocol {
    pub(crate) fn new(
        settings: Settings,
        selected: Protocols,
        transport: Transport,
        callback: CallbackHandler,
    ) -> Self {
        let mut h1 = H1Protocol::new(settings, callback);
        h1.new_connection(transport.clone());

        Self {
            selected,
            transport,
            h1,
            writer_buffer: BytesMut::with_capacity(BUFFER_SIZE),
            reader_buffer: BytesMut::with_capacity(BUFFER_SIZE),
        }
    }
}

impl AutoProtocol {
    /// Allows the chance to switch protocol just after reading has
    /// finished.
    pub(crate) fn maybe_switch(&mut self) -> PyResult<SwitchStatus> {
        match self.selected {
            Protocols::H1 => self.h1.maybe_switch(),
        }
    }

    /// Pauses reading from the event loop and notifies the protocol of
    /// the pause to allow the protocol to re-wake the state later on.
    fn pause_writing(&mut self) -> PyResult<()> {
        self.transport.pause_writing()?;
        Ok(())
    }
}

impl SocketState for AutoProtocol {
    fn new_connection(&mut self, transport: Transport) {
        self.transport = transport;

        match self.selected {
            Protocols::H1 => self.h1.new_connection(self.transport.clone()),
        }
    }

    fn connection_lost(&mut self) -> PyResult<()> {
        self.transport.pause_reading()?;
        self.transport.pause_writing()?;
        self.reader_buffer.clear();
        self.writer_buffer.clear();
        match self.selected {
            Protocols::H1 => self.h1.lost_connection(),
        }
    }

    /// The EOF has been sent by the socket.
    fn eof_received(&mut self) -> PyResult<()> {
        self.connection_lost()
    }
}

impl BufferHandler for AutoProtocol {
    fn read_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        Ok(&mut self.reader_buffer)
    }

    fn read_buffer_filled(&mut self, _amount: usize) -> PyResult<()> {
        match self.selected {
            Protocols::H1 => self.h1.data_received(&mut self.reader_buffer),
        }
    }

    fn write_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        match self.selected {
            Protocols::H1 => {
                self.h1.fill_write_buffer(&mut self.writer_buffer)?;
            }
        };

        Ok(&mut self.writer_buffer)
    }

    fn write_buffer_drained(&mut self, amount: usize) -> PyResult<()> {
        if (amount == 0) | (self.writer_buffer.len() == 0) {
            self.pause_writing()?;
        }

        Ok(())
    }
}
