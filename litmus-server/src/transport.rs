use std::net::SocketAddr;

use pyo3::PyResult;

use crate::event_loop::PreSetEventLoop;
use crate::traits::BaseTransport;

#[derive(Clone)]
pub struct Transport {
    pub client: SocketAddr,
    pub server: SocketAddr,
    pub tls: bool,
    event_loop: PreSetEventLoop,
}

impl Transport {
    /// Create a new transport instance bound to the given pre-set event loop
    pub fn new(
        client: SocketAddr,
        server: SocketAddr,
        tls: bool,
        event_loop: PreSetEventLoop,
    ) -> Self {
        Self {
            client,
            server,
            tls,
            event_loop,
        }
    }
}

impl BaseTransport for Transport {
    /// Closes the connection to the socket.
    ///
    /// The closing itself is invoked using loop.call_soon, this is not
    /// guaranteed to be instant.
    fn close(&self) -> PyResult<()> {
        self.event_loop.close_socket()
    }

    /// Removes the file descriptor listener from the event loop
    /// therefore pausing reading callbacks.
    fn pause_reading(&self) -> PyResult<()> {
        self.event_loop.remove_reader()
    }

    /// Adds the file descriptor listener to the event loop ready to start
    /// polling the reading callback when data can be read from the socket.
    fn resume_reading(&self) -> PyResult<()> {
        self.event_loop.add_reader()
    }

    /// Removes the file descriptor listener from the event loop
    /// therefore pausing writing callbacks.
    fn pause_writing(&self) -> PyResult<()> {
        self.event_loop.remove_writer()
    }

    /// Adds the file descriptor listener to the event loop ready to start
    /// polling the writing callback when data can be written to the socket.
    fn resume_writing(&self) -> PyResult<()> {
        self.event_loop.add_writer()
    }
}
