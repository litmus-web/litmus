use pyo3::PyResult;

use crate::pyre_server::abc::SocketCommunicator;
use crate::pyre_server::net::stream::{TcpHandle, SocketStatus};
use crate::pyre_server::event_loop::PreSetEventLoop;

use crate::pyre_server::protocol_manager::{AutoProtocol, SelectedProtocol};
use crate::pyre_server::transport::Transport;


/// A wrapper around the standard tcp stream and addr to produce a interface
/// able to interact with both a protocol and handler.
pub struct Client {
    /// A cheaply cloneable handle for controlling the event loop callbacks.
    event_loop: PreSetEventLoop,

    /// The internal wrapper that has a high-level interface for interacting
    /// with the low level socket across difference os.
    handle: TcpHandle,

    /// A `ProtoManager` that controls the swapping and interfacing of
    /// multiple protocols.
    protocol: AutoProtocol,
}

impl Client {
    /// Produces a `client::Client` instance from a given TcpHandle.
    pub fn from_handle(
        handle: TcpHandle,
        event_loop: PreSetEventLoop,
    ) -> PyResult<Self> {

        let transport = Transport::new(event_loop.clone());

        // Default is H1 for now, maybe add configurable option later.
        let protocol = AutoProtocol::new(
            SelectedProtocol::H1,
            transport,
        )?;

        Ok(Self {
            event_loop,
            handle,
            protocol,
        })
    }

    /// Invoked when the client is being re-used for another connection after
    /// handling the previous connection to re-cycle memory.
    pub fn _bind_handle(
        &mut self,
        handle: TcpHandle,
        event_loop: PreSetEventLoop,
    ) {
        self.handle = handle;
        self.event_loop = event_loop;
    }

    /// Shuts down the client.
    ///
    /// Invoked when the whole server is
    /// preparing to shutdown and close.
    pub fn shutdown(&mut self) -> PyResult<()> {
        if self.event_loop.is_reading() {
            self.event_loop.pause_reading()?;
        }

        if self.event_loop.is_writing() {
            self.event_loop.pause_writing()?;
        }

        Ok(())
    }

    /// Handles reading from the given socket to a acquired buffer.
    pub fn poll_read(&mut self) -> PyResult<()> {
        let buffer = self.protocol.read_buffer_acquire()?;

        let len = match self.handle.read(buffer)? {
            SocketStatus::WouldBlock => return Ok(()),
            SocketStatus::Complete(len) => len,
            SocketStatus::Disconnect => {
                self.protocol.lost_connection()?;
                return self.shutdown();
            },
        };

        self.protocol.read_buffer_filled(len)?;

        self.protocol.maybe_switch()?;

        Ok(())
    }

    /// Handles writing to the given socket from a acquired buffer.
    pub fn poll_write(&mut self) -> PyResult<()> {
        let buffer = self.protocol.write_buffer_acquire()?;

        let len = match self.handle.write(buffer)? {
            SocketStatus::WouldBlock => return Ok(()),
            SocketStatus::Complete(len) => len,
            SocketStatus::Disconnect => {
                self.protocol.lost_connection()?;
                return self.shutdown();
            },
        };

        self.protocol.write_buffer_drained(len)?;

        Ok(())
    }
}
