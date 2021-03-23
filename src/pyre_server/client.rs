use pyo3::PyResult;
use std::time::{Instant, Duration};

use crate::pyre_server::abc::SocketCommunicator;
use crate::pyre_server::net::stream::{TcpHandle, SocketStatus};
use crate::pyre_server::event_loop::PreSetEventLoop;
use crate::pyre_server::protocol_manager::{AutoProtocol, SelectedProtocol};
use crate::pyre_server::transport::Transport;
use crate::pyre_server::py_callback::CallbackHandler;
use crate::pyre_server::settings::Settings;


/// A wrapper around the standard tcp stream and addr to produce a interface
/// able to interact with both a protocol and handler.
pub struct Client {
    /// A cheaply cloneable handle for controlling the event loop callbacks.
    event_loop: PreSetEventLoop,

    /// The internal wrapper that has a high-level interface for interacting
    /// with the low level socket across difference os.
    handle: TcpHandle,

    /// A manager that controls the swapping and interfacing of
    /// multiple protocols.
    protocol: AutoProtocol,

    /// Represents if the client is idle because the client has closed
    /// the connection or the protocol has closed the connection.
    is_idle: bool,

    /// When data was last received.
    last_time: Instant,

    /// The instant since the client was set to idle.
    idle_for: Instant,
}

impl Client {
    /// Produces a `client::Client` instance from a given TcpHandle.
    pub fn from_handle(
        handle: TcpHandle,
        event_loop: PreSetEventLoop,
        callback: CallbackHandler,
        settings: Settings,
    ) -> PyResult<Self> {

        let transport = Transport::new(
            handle.addr,
            event_loop.clone(),
            );

        // Default is H1 for now, maybe add configurable option later.
        let protocol = AutoProtocol::new(
            settings,
            SelectedProtocol::H1,
            transport,
            callback,
        )?;

        Ok(Self {
            event_loop,
            handle,
            protocol,

            is_idle: false,
            last_time: Instant::now(),
            idle_for: Instant::now(),
        })
    }

    /// Invoked when the client is being re-used for another connection after
    /// handling the previous connection to re-cycle memory.
    pub fn bind_handle(
        &mut self,
        handle: TcpHandle,
    ) -> PyResult<()> {
        self.handle = handle;

        self.is_idle = false;

        let transport = Transport::new(
            self.handle.addr,
            self.event_loop.clone()
        );
        self.protocol.new_connection(transport)?;

        Ok(())
    }

    /// Represents if the client is idle because the client has closed
    /// the connection or the protocol has closed the connection.
    #[inline]
    pub fn idle(&self) -> bool {
        self.is_idle
    }

    /// Measures how long the client has been inactive for.
    #[inline]
    pub fn idle_duration(&self) -> Duration {
        self.idle_for.elapsed()
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
                self.is_idle = true;
                self.idle_for = Instant::now();
                return self.shutdown();
            },
        };

        // EOF
        if len == 0 {
            self.protocol.eof_received()?;
            return Ok(())
        }

        self.protocol.read_buffer_filled(len)?;

        self.last_time = Instant::now();

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
                self.is_idle = true;
                self.idle_for = Instant::now();
                return self.shutdown();
            },
        };

        self.protocol.write_buffer_drained(len)?;

        Ok(())
    }

    /// Called every X seconds equal to the passed duration.
    /// This is what the protocols should use to check if the timeout
    /// period has elapsed.
    pub fn poll_keep_alive(&mut self, limit: Duration) -> PyResult<()> {
        if self.last_time.elapsed() >= limit {
            self.handle.close();
            self.is_idle = true;
            self.idle_for = Instant::now();
            return self.shutdown()
        }
        Ok(())
    }
}
