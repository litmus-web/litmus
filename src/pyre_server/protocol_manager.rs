use pyo3::PyResult;
use bytes::BytesMut;

use crate::pyre_server::abc::SocketCommunicator;
use crate::pyre_server::switch::{Switchable, SwitchStatus};

use crate::pyre_server::protocols::h1;
use crate::pyre_server::transport::Transport;


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

        Ok(Self {
            selected,
            transport,
            h1,
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
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.read_buffer_acquire()
            },
        }
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn read_buffer_filled(&mut self, amount: usize) -> PyResult<()> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.read_buffer_filled(amount)
            },
        }
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn write_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.write_buffer_acquire()
            },
        }
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn write_buffer_drained(&mut self, amount: usize) -> PyResult<()> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.write_buffer_drained(amount)
            },
        }
    }
}
