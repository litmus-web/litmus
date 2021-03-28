use pyo3::PyResult;
use bytes::BytesMut;

use crate::abc::{SocketCommunicator, ProtocolBuffers, BaseTransport};
use crate::switch::{Switchable, SwitchStatus};
use crate::protocols::h1;
use crate::transport::Transport;
use crate::py_callback::CallbackHandler;
use crate::settings::Settings;


const MAX_BUFFER_LIMIT: usize = 256 * 1024;


/// The allowed protocols to set the auto protocol to.
pub enum SelectedProtocol {
    /// Selects the H1 protocol.
    H1,
}


/// A changeable protocol which does not modify the external API.
pub struct AutoProtocol {
    /// The selector that determines which protocol is called and when.
    selected: SelectedProtocol,

    /// The selected transport, this handles all event loop interactions.
    transport: Transport,

    /// The http/1 protocol handler.
    h1: h1::H1Protocol,

    /// The writer buffer that covers all protocols, this saves memory as
    /// we have to create each protocol instance per client so we dont want
    /// to be creating 3 * 256KB every time.
    writer_buffer: BytesMut,

    /// The reader buffer that covers all protocols, this saves memory as
    /// we have to create each protocol instance per client so we dont want
    /// to be creating 3 * 256KB every time.
    reader_buffer: BytesMut,
}

impl AutoProtocol {
    /// Creates a new auto protocol with the protocol set the specified
    /// `SelectedProtocol` enum.
    pub fn new(
        settings: Settings,
        selected: SelectedProtocol,
        transport: Transport,
        callback: CallbackHandler,
    ) -> PyResult<Self> {

        let mut h1 = h1::H1Protocol::new(settings, callback);

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
    /// Called when the protocol is in charge of a new socket / handle,
    /// the `Transport` can be used to pause and resume reading from this
    /// socket.
    pub fn new_connection(&mut self, transport: Transport) -> PyResult<()> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.new_connection(transport)
            },
        }
    }

    /// Called when the connection is lost from the protocol in order to
    /// properly reset state.
    pub fn lost_connection(&mut self) -> PyResult<()> {
        self.reader_buffer.clear();
        self.writer_buffer.clear();

        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.lost_connection()
            },
        }
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

    /// Pauses reading from the event loop and notifies the protocol of
    /// the pause to allow the protocol to re-wake the state later on.
    fn pause_writing(&mut self) -> PyResult<()> {
        self.transport.pause_writing()?;
        Ok(())
    }

    /// The EOF has been sent by the socket.
    pub fn eof_received(&mut self) -> PyResult<()> {
        match self.selected {
            SelectedProtocol::H1 => {
                self.h1.eof_received()
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
        if (amount == 0) | (self.writer_buffer.len() == 0) {
            self.pause_writing()?;
        }

        Ok(())
    }
}
