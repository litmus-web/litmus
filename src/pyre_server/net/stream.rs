use std::net::{TcpStream, SocketAddr, Shutdown};
use std::io::{Read, Write, ErrorKind};

#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

use bytes::{BytesMut, BufMut};
use pyo3::{PyResult, PyErr};


pub enum SocketStatus {
    Complete(usize),
    WouldBlock,
    Disconnect
}



/// A struct that wraps a given TcpStream and SocketAddr and produces a
/// contain for interactions that are os agnostic.
pub struct TcpHandle {
    /// The internal `std::net::TcpStream` instance that should be set
    /// to be non-blocking.
    stream: TcpStream,

    /// The remote's given socket addr as given by the tcp listener upon
    /// accepting the client / connection.
    pub addr: SocketAddr,
}

impl TcpHandle {
    /// Create a new tcp handle wrapping the given stream and addr.
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        Self { stream, addr }
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
    pub fn read(&mut self, buffer: &mut BytesMut) -> PyResult<SocketStatus> {
        let data = buffer.chunk_mut();
        let mut slice = unsafe {
            std::slice::from_raw_parts_mut(data.as_mut_ptr(),data.len())
        };

        let len = match self.stream.read(&mut slice) {
            Ok(n) => n,
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                return Ok(SocketStatus::WouldBlock)
            },
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                return Ok(SocketStatus::Disconnect)
            },
            Err(ref e) if e.kind() == ErrorKind::ConnectionAborted => {
                return Ok(SocketStatus::Disconnect)
            },
            Err(e) => {
                return Err(PyErr::from(e))
            }
        };

        unsafe { buffer.advance_mut(len); }

        Ok(SocketStatus::Complete(len))
    }

    /// Writes the data from the supplied buffer to the socket returning a
    /// result with the number of bytes written to the socket if the operation
    /// is a success.
    pub fn write(&mut self, buffer: &mut BytesMut) -> PyResult<SocketStatus> {
        println!("writing: {:?}", buffer);  // todo fix infinite polling
        let len = match self.stream.write(buffer) {
            Ok(n) => n,
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                return Ok(SocketStatus::WouldBlock)
            },
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                return Ok(SocketStatus::Disconnect)
            },
            Err(ref e) if e.kind() == ErrorKind::ConnectionAborted => {
                return Ok(SocketStatus::Disconnect)
            },
            Err(e) => {
                return Err(PyErr::from(e))
            }
        };

        let _ = buffer.split_to(len);

        Ok(SocketStatus::Complete(len))
    }

    pub fn close(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}