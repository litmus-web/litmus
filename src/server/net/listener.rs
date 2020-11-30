// Pyo3 experimental
use pyo3::prelude::*;

// Pyo3 stable stuffs
use pyo3::exceptions::{PyBlockingIOError, PyIOError, PyValueError, PyConnectionError, PyRuntimeError};
use pyo3::{wrap_pyfunction, PyAsyncProtocol, PyIterProtocol, AsPyPointer};
use pyo3::class::iter::{IterNextOutput};

use std::sync::Arc;
use std::sync::mpsc;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io;
use std::io::{Read, Write};
use std::net::Shutdown::Both;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use bytes::{BytesMut, BufMut};

use once_cell::sync::OnceCell;

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawSocket;
#[cfg(target_os = "unix")]
use std::os::unix::io::AsRawFd;


static LOOP_ADD_READER: OnceCell<PyObject> = OnceCell::new();
static LOOP_REMOVE_READER: OnceCell<PyObject> = OnceCell::new();


pub fn setup(loop_add_reader: PyObject, loop_remove_reader: PyObject) {
    LOOP_ADD_READER.get_or_init(|| loop_add_reader);
    LOOP_REMOVE_READER.get_or_init(|| loop_remove_reader);
}


#[pyclass]
pub struct PyreListener {
    listener: TcpListener,
    free_objects: Vec<PyreClientAddrPair>,
    used_objects: Vec<PyreClientAddrPair>,
    backlog: usize,
}

#[pymethods]
impl PyreListener {
    #[new]
    fn new(
        host: &str,
        port: u16,
        backlog: usize,
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
            backlog,

            free_objects: Vec::with_capacity(1024),
            used_objects: Vec::with_capacity(1024),
        })
    }

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
}


#[pyclass]
pub struct PyreClientAddrPair {
    client: TcpStream,
    addr: SocketAddr,
}
