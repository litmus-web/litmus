use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

use pyo3::{PyErr, PyResult};

use crate::net::StreamHandle;

/// Represents the state of the socket that is accepting connections.
pub enum Status<T> {
    /// The client was successfully accepted and is ready to be unwrapped
    /// to extract the given Client instance.
    Successful(T),

    /// States that the loop listeners should wait for the fd to become
    /// available again.
    ShouldPause,
}

/// A non-blocking tcp listener, this is just a wrapper over the
/// `std::net::TcpListener` just with non_blocking set to true and
/// a custom `net::NoneBlockingListener.accept()` method implemented for
/// use with Python.
pub struct NoneBlockingListener {
    /// The base TcpListener that is held internally, this should be
    /// set as non-blocking.
    listener: TcpListener,

    pub addr: SocketAddr,
}

impl NoneBlockingListener {
    /// Attempts to bind to a given addresses and returns `Self`
    pub fn bind(addr: &str) -> PyResult<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true).expect("set non-blocking");

        Ok(Self {
            listener,
            addr: addr.parse().expect("invalid addr"),
        })
    }

    /// Accepts a single client from the socket without blocking, returning a
    /// `net::Status` describing if the fd listener should be paused or the
    /// client itself if has been accepted successfully.
    pub fn accept(&self) -> PyResult<Status<StreamHandle>> {
        let (stream, addr) = match self.listener.accept() {
            Ok(pair) => pair,
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                return Ok(Status::ShouldPause)
            },
            Err(e) => return Err(PyErr::from(e)),
        };

        stream.set_nonblocking(true).expect("set non-blocking");

        let handle = StreamHandle::new(stream, addr, self.addr);
        Ok(Status::Successful(handle))
    }

    /// Returns the raw file descriptor of the socket.
    #[cfg(windows)]
    pub fn fd(&self) -> u64 {
        self.listener.as_raw_socket()
    }

    /// Returns the raw file descriptor of the socket.
    #[cfg(unix)]
    pub fn fd(&self) -> i32 {
        self.listener.as_raw_fd()
    }
}
