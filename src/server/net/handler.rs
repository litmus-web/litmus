// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyIOError};

use std::{io, mem};
use std::sync::Arc;
use std::mem::MaybeUninit;
use std::sync::atomic::Ordering::Relaxed;
use std::net::Shutdown::Both;
use std::sync::atomic::AtomicBool;
use std::io::{Write, Read};
use std::num::ParseIntError;

use bytes::{BytesMut, BufMut};
use once_cell::sync::OnceCell;

use crate::listener::PyreClientAddrPair;


const MAX_BUFFER_SIZE: usize = 512 * 1024;

static LOOP_CREATE_TASK: OnceCell<PyObject> = OnceCell::new();
static LOOP_REMOVE_READER: OnceCell<PyObject> = OnceCell::new();
static LOOP_REMOVE_WRITER: OnceCell<PyObject> = OnceCell::new();


/// This sets up the net package's global state, this is absolutely required
/// to stop large amounts of python calls and clones, this MUST be setup before
/// any listeners can be created otherwise you risk UB.
pub fn setup(
    loop_create_task: PyObject,
    loop_remove_reader: PyObject,
    loop_remove_writer: PyObject,
) {
    LOOP_CREATE_TASK.get_or_init(|| loop_create_task);
    LOOP_REMOVE_READER.get_or_init(|| loop_remove_reader);
    LOOP_REMOVE_WRITER.get_or_init(|| loop_remove_writer);
}


/// The PyreClientHandler struct is what handles all the actual interactions
/// with the socket, this can be reused several times over and is designed to
/// handle concurrent pipelined requests, hopefully we can support http/2 as
/// well as http/1.1 once h11 works. :-)
#[pyclass]
pub struct PyreClientHandler {
    client_handle: PyreClientAddrPair,

    // buffers
    reading_buffer: BytesMut,
    writing_buffer: BytesMut,

    // Pre-Built callbacks
    resume_reading: MaybeUninit<Arc<PyObject>>,
    resume_writing: MaybeUninit<Arc<PyObject>>,

    // state
    reading: Arc<AtomicBool>,
    writing: Arc<AtomicBool>,

    should_parse: bool,
    chunked_encoding: bool,
    expected_content_size: usize,
}

/// The implementations for all initialisation of objects and existing object
#[pymethods]
impl PyreClientHandler {
    /// Used to create a new handler object, generally this should only be
    /// created when absolutely needed.
    #[new]
    fn new(client: PyreClientAddrPair) -> PyResult<Self> {
        if let _ = LOOP_REMOVE_READER.get().is_none() {
            return Err(PyRuntimeError::new_err(
                "Global state has not been setup, \
                did you forget to call pyre.setup()?"
            ))
        }

        Ok(PyreClientHandler {
            client_handle: client,

            reading_buffer: BytesMut::with_capacity(MAX_BUFFER_SIZE),
            writing_buffer: BytesMut::with_capacity(MAX_BUFFER_SIZE),

            resume_reading: MaybeUninit::<Arc<PyObject>>::uninit(),
            resume_writing: MaybeUninit::<Arc<PyObject>>::uninit(),

            reading: Arc::new(AtomicBool::new(true)),
            writing: Arc::new(AtomicBool::new(false)),

            should_parse: true,
            chunked_encoding: false,
            expected_content_size: 0,
        })
    }

    /// This should only be called when the object is first made, if this
    /// is called after being called once you will run into ub because it
    /// will not drop the value.
    fn init(&mut self, add_reader: PyObject, add_writer: PyObject) {
        let mut resume_ptr = self.resume_reading.as_mut_ptr();
        unsafe { resume_ptr.write(Arc::new(add_reader)) };

        let mut resume_ptr = self.resume_writing.as_mut_ptr();
        unsafe { resume_ptr.write(Arc::new(add_writer)) };
    }

    /// This is used when recycle the handler objects as the memory allocations
    /// are pretty expensive and we need some way of controlling the ram usage
    /// because theres a weird leak otherwise.
    fn new_client(&mut self, client: PyreClientAddrPair) {
        self.reset_state();
        self.client_handle = client;
    }

    /// Resets all state the handler might have as to not interfere
    /// with new client handles.
    fn reset_state(&mut self) {
        self.reading_buffer.clear();
        self.writing_buffer.clear();

        self.reading.store(true, Relaxed);
        self.writing.store(false, Relaxed);
    }
}

/// All callback events e.g. when `EventLoop.add_reader` calls the callback.
#[pymethods]
impl PyreClientHandler {
    /// Called when the event loop detects that the socket is able
    /// to be read from without blocking.
    fn poll_read(&mut self) -> PyResult<()> {
        self.read_socket();

        if self.should_parse {
            self.parse()?;
        } else {
            self.feed_date()?;
        }

        Ok(())
    }

    /// Called when the event loop detects that the socket is able
    /// to be written to without blocking.
    fn poll_write(&mut self) -> PyResult<()> {
        Ok(())
    }
}

/// General utils for handling the sockets
impl PyreClientHandler {
    fn close_and_cleanup(&mut self) -> PyResult<()> {
        if self.reading.load(Relaxed) {
            //self.pause_reading()?;
        }

        if self.writing.load(Relaxed) {
            //self.remove_writer()?;
        }
        let _ = self.client_handle.client.shutdown(Both);
        Ok(())
    }

    fn respond_with_error(&mut self, msg: &'static str) {
        let _ = self.client_handle.client.write(format!(
            "HTTP/1.1 400 Bad Request\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/plain; charset=UTF-8\r\n\r\n\
            {}", &msg.len(), msg
        ).as_bytes());
    }

}

impl PyreClientHandler {
    fn parse(&mut self) -> PyResult<()> {

        Ok(())
    }

    fn feed_date(&mut self) -> PyResult<()> {
        Ok(())
    }

    /// Reads data from the socket to the internal buffer.
    fn read_socket(&mut self) -> PyResult<()> {
        let data = self.reading_buffer.bytes_mut();
        let slice = unsafe {
            std::slice::from_raw_parts_mut(data.as_mut_ptr(),data.len())
        };

        return match self.client_handle.client.read(slice) {
            Ok(len) => {
                unsafe { self.reading_buffer.advance_mut(len); }
                //self.on_read_complete()?;
                Ok(())
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                Ok(())
            },
            Err(ref e) if (
                    (e.kind() == io::ErrorKind::ConnectionReset) |
                    (e.kind() == io::ErrorKind::ConnectionAborted) |
                    (e.kind() == io::ErrorKind::BrokenPipe)
            ) => {
                self.close_and_cleanup()?;
                Ok(())
            },
            Err(e) => Err(PyIOError::new_err(format!(
                "{:?}", e
            ))),
        };
    }
}