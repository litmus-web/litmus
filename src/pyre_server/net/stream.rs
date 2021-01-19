use std::net::{TcpStream, SocketAddr};
use std::io::{Read, Write};

#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

use bytes::{BytesMut, BufMut};
use pyo3::PyResult;



/// A struct that wraps a given TcpStream and SocketAddr and produces a
/// contain for interactions that are os agnostic.
pub struct TcpHandle {
    /// The internal `std::net::TcpStream` instance that should be set
    /// to be non-blocking.
    stream: TcpStream,

    /// The remote's given socket addr as given by the tcp listener upon
    /// accepting the client / connection.
    _addr: SocketAddr,
}

impl TcpHandle {
    /// Create a new tcp handle wrapping the given stream and addr.
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        Self { stream, _addr: addr }
    }

    /// Returns the raw file descriptor of the socket.
    #[cfg(windows)]
    pub fn fd(&self) -> u64 {
        self.stream.as_raw_socket()
    }

    /// Returns the raw file descriptor of the socket.
    #[cfg(unix)]
    pub fn fd(&self) -> i32 {
        self.stream.as_raw_fd()
    }

    /// Reads the data from the socket to the supplied buffer returning
    /// a result with the number of bytes read if the operation is a success.
    pub fn read(&mut self, buffer: &mut BytesMut) -> PyResult<usize> {
        let data = buffer.chunk_mut();
        let mut slice = unsafe {
            std::slice::from_raw_parts_mut(data.as_mut_ptr(),data.len())
        };

        let len = self.stream.read(&mut slice)?;

        unsafe { buffer.advance_mut(len); }

        Ok(len)
    }

    /// Writes the data from the supplied buffer to the socket returning a
    /// result with the number of bytes written to the socket if the operation
    /// is a success.
    pub fn write(&mut self, buffer: &mut BytesMut) -> PyResult<usize> {
        let len = self.stream.write(buffer)?;

        let _ = buffer.split_to(len);

        Ok(len)
    }
}