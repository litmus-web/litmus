// Pyo3 experimental
use pyo3::prelude::*;

// Pyo3 stable stuffs
use pyo3::exceptions::{PyBlockingIOError, PyRuntimeError, PyConnectionError, PyNotImplementedError};

use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io;

use once_cell::sync::OnceCell;

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawSocket;

#[cfg(target_os = "unix")]
use std::os::unix::io::AsRawFd;
use std::sync::Arc;


/// This is the main listener type, built off of a Rust based TcpListener
#[pyclass]
pub struct PyreListener {
    listener: TcpListener,
    callback: Arc<PyObject>
}

#[pymethods]
impl PyreListener {
    #[new]
    fn new(
        callback: PyObject,
        host: &str,
        port: u16,
    ) -> PyResult<Self> {

        let addr = format!("{}:{}", host, port);
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                let msg = format!("{:?}", e);
                return Err(PyConnectionError::new_err(msg))
            }
        };

        listener.set_nonblocking(true)
            .expect("Failed to set non-blocking");

        println!("Listening for connections on {}", addr);

        Ok(PyreListener {
            listener,
            callback: Arc::new(callback)
        })
    }

    /// Accepts a single client returning a internal pair for handling,
    /// ideally would be nice to have all of this be internal however the
    /// limitation of lifetimes and threadsafety make this rather hard.
    fn accept(&mut self) -> PyResult<PyreClientAddrPair> {
        let (client, addr) = match self.listener.accept() {
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                return Err(PyBlockingIOError::new_err(()))
            },
            Err(e) => {
                return Err(PyRuntimeError::new_err(format!("{:?}", e)))
            },
            Ok(pair) => pair,
        };
        client.set_nonblocking(true).expect("Cant set non-blocking");

        Ok(PyreClientAddrPair{
            client,
            addr,
            callback: self.callback.clone(),
        })
    }

    /// This is equivalent to python's socket.fileno()
    /// depending on what platform you are on will affect what is
    /// returned hence the configs.
    #[cfg(target_os = "windows")]
    fn fd(&self) -> u64 {
        self.listener.as_raw_socket()
    }

    #[cfg(target_os = "unix")]
    fn fd(&self) -> i32 {
        self.listener.as_raw_fd()
    }
}


/// This is purely a internal type that handles the client and addr pair
/// that the handlers can then use to interact with the socket, this should
/// never be made from python as this is purely a internal type.
#[pyclass]
pub struct PyreClientAddrPair {
    pub client: TcpStream,
    pub addr: SocketAddr,
    pub callback: Arc<PyObject>,
}

#[pymethods]
impl PyreClientAddrPair {
    /// This is equivalent to python's socket.fileno()
    /// depending on what platform you are on will affect what is
    /// returned hence the configs.
    #[cfg(target_os = "windows")]
    fn fd(&self) -> u64 {
        self.client.as_raw_socket()
    }

    #[cfg(target_os = "unix")]
    fn fd(&self) -> i32 {
        self.client.as_raw_fd()
    }
}

impl FromPyObject<'_> for PyreClientAddrPair {
    fn extract(ob: &PyAny) -> PyResult<Self> {
        Err(PyNotImplementedError::new_err(
            "Client pairs are a internal type and should not be made."
        ))
    }
}
