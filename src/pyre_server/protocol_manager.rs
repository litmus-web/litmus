use pyo3::PyResult;
use bytes::BytesMut;

use crate::pyre_server::abc::{SocketCommunicator, ProtocolBuffers, BaseTransport};
use crate::pyre_server::switch::{Switchable, SwitchStatus};

use crate::pyre_server::protocols::h1;
use crate::pyre_server::transport::Transport;


const MAX_BUFFER_LIMIT: usize = 256 * 1024;


/// The allowed protocols to set the auto protocol to.
pub enum SelectedProtocol {
    /// Selects the H1 protocol.
    H1,
}


/// A changeable protocol which does not modify the external API.
pub struct AutoProtocol {
    selected: SelectedProtocol,
    transport: Transport,

    h1: h1::H1Protocol,

    writer_buffer: BytesMut,
    reader_buffer: BytesMut,
}

impl AutoProtocol {
    /// Creates a new auto protocol with the protocol set the specified
    /// `SelectedProtocol` enum.
    pub fn new(
        selected: SelectedProtocol,
        transport: Transport,
    ) -> PyResult<Self> {

        let mut h1 = h1::H1Protocol::new();
        h1.new_connection(transport.clone())?;

        let buff1 = BytesMut::with_capacity(MAX_BUFFER_LIMIT);
        let buff2 = BytesMut::with_capacity(MAX_BUFFER_LIMIT);

        Ok(Self {
            selected,
            transport,
            h1,
            writer_buffer: buff1,
            reader_buffer: buff2,
        })
    }
}

impl AutoProtocol {
    /// Allows the chance to switch protocol just after reading has
    /// finished.
    pub fn maybe_switch(&mut self) -> PyResult<SwitchStatus> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.switch_protocol()
            },
        }
    }
}

impl SocketCommunicator for AutoProtocol {
    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn read_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        Ok(&mut self.reader_buffer)
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn read_buffer_filled(&mut self, _amount: usize) -> PyResult<()> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.data_received(&mut self.reader_buffer)
            },
        }
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn write_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        match self.selected {
            SelectedProtocol::H1 => {
                self.h1.fill_write_buffer(&mut self.writer_buffer)?;
            },
        };

        Ok(&mut self.writer_buffer)
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn write_buffer_drained(&mut self, amount: usize) -> PyResult<()> {
        if amount == 0 {
            self.transport.pause_writing()?;
        }

        Ok(())
    }
}
