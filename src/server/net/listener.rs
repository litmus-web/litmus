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


static LOOP_ADD_READER: OnceCell<PyObject> = OnceCell::new();
static LOOP_REMOVE_READER: OnceCell<PyObject> = OnceCell::new();


/// This sets up the net package's global state, this is absolutely required
/// to stop large amounts of python calls and clones, this MUST be setup before
/// any listeners can be created otherwise you risk UB.
pub fn setup(loop_add_reader: PyObject, loop_remove_reader: PyObject) {
    LOOP_ADD_READER.get_or_init(|| loop_add_reader);
    LOOP_REMOVE_READER.get_or_init(|| loop_remove_reader);
}

/// This is the main listener type, built off of a Rust based TcpListener
#[pyclass]
pub struct PyreListener {
    listener: TcpListener,
}

#[pymethods]
impl PyreListener {
    #[new]
    fn new(
        host: &str,
        port: u16,
    ) -> PyResult<Self> {
        if let _ = LOOP_ADD_READER.get().is_none() {
            return Err(PyRuntimeError::new_err(
                "Global state has not been setup, \
                did you forget to call pyre.setup()?"
            ))
        }


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
    client: TcpStream,
    addr: SocketAddr,
}

impl FromPyObject<'_> for PyreClientAddrPair {
    fn extract(ob: &PyAny) -> PyResult<Self> {
        Err(PyNotImplementedError::new_err(
            "Client pairs are a internal type and should not be made."
        ))
    }
}
