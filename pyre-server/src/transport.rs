use crate::abc::BaseTransport;
use crate::event_loop::PreSetEventLoop;

use pyo3::PyResult;
use std::net::SocketAddr;


/// Handles the higher level controls for a protocol to use e.g.
/// pausing and resuming reading from the event loop.
#[derive(Clone)]
pub struct Transport {
    pub client: SocketAddr,
    event_loop: PreSetEventLoop
}

impl Transport {
    /// Create a new transport instance bound to the given pre-set event loop
    pub fn new(client: SocketAddr, event_loop: PreSetEventLoop) -> Self {
        Self {
            client,
            event_loop
        }
    }
}

impl BaseTransport for Transport {
    /// Closes the connection to the socket.
    ///
    /// The closing itself is invoked using loop.call_soon, this is not
    /// guaranteed to be instant.
    fn close(&self) -> PyResult<()> {
        self.event_loop.close()
    }

    /// Removes the file descriptor listener from the event loop
    /// therefore pausing reading callbacks.
    fn pause_reading(&self) -> PyResult<()> {
        if self.event_loop.is_reading() {
            self.event_loop.pause_reading()?;
        }
        Ok(())
    }

    /// Adds the file descriptor listener to the event loop ready to start
    /// polling the reading callback when data can be read from the socket.
    fn resume_reading(&self) -> PyResult<()> {
        if !self.event_loop.is_reading() {
            self.event_loop.resume_reading()?;
        }
        Ok(())
    }

    /// Removes the file descriptor listener from the event loop
    /// therefore pausing writing callbacks.
    fn pause_writing(&self) -> PyResult<()> {
        if self.event_loop.is_writing() {
            self.event_loop.pause_writing()?;
        }
        Ok(())
    }

    /// Adds the file descriptor listener to the event loop ready to start
    /// polling the writing callback when data can be written to the socket.
    fn resume_writing(&self) -> PyResult<()> {
        if !self.event_loop.is_writing() {
            self.event_loop.resume_writing()?;
        }
        Ok(())
    }
}


