use pyo3::prelude::*;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type CheapPyObject = Arc<PyObject>;

#[cfg(windows)]
pub type SocketFd = u64;

#[cfg(unix)]
pub type SocketFd = i32;

#[derive(Clone)]
pub struct EventLoop {
    add_reader: CheapPyObject,
    remove_reader: CheapPyObject,
    add_writer: CheapPyObject,
    remove_writer: CheapPyObject,
    close_socket: CheapPyObject,
}

impl EventLoop {
    pub fn new(
        add_reader: PyObject,
        remove_reader: PyObject,
        add_writer: PyObject,
        remove_writer: PyObject,
        close_socket: PyObject,
    ) -> Self {
        Self {
            add_reader: Arc::from(add_reader),
            remove_reader: Arc::from(remove_reader),
            add_writer: Arc::from(add_writer),
            remove_writer: Arc::from(remove_writer),
            close_socket: Arc::from(close_socket),
        }
    }

    pub fn close_socket(&self, index: usize) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = self.close_socket.call1(py, (index,))?;
            Ok(())
        })
    }

    /// Start monitoring the file descriptor for read availability
    /// and invokes a callback once the fd is available for reading.
    pub fn add_reader(&self, fd: SocketFd, index: usize) -> PyResult<()> {
        self.invoke_add(&self.add_reader, fd, index)
    }

    /// Stop monitoring the file descriptor for read availability.
    pub fn remove_reader(&self, fd: SocketFd) -> PyResult<()> {
        self.invoke_remove(&self.remove_reader, fd)
    }

    /// Start monitoring the file descriptor for write availability
    /// and invokes a callback once the fd is available for writing.
    pub fn add_writer(&self, fd: SocketFd, index: usize) -> PyResult<()> {
        self.invoke_add(&self.add_writer, fd, index)
    }

    /// Stop monitoring the file descriptor for write availability.
    pub fn remove_writer(&self, fd: SocketFd) -> PyResult<()> {
        self.invoke_remove(&self.remove_writer, fd)
    }

    fn invoke_remove(&self, cb: &PyObject, fd: SocketFd) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = cb.call1(py, (fd,))?;
            Ok(())
        })
    }

    fn invoke_add(&self, cb: &PyObject, fd: SocketFd, index: usize) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = cb.call1(py, (fd, index))?;
            Ok(())
        })
    }
}

/// A wrapper around an EventLoop with a pre-set file descriptor and index.
///
/// This helps abstract the set socket away from handlers that are designed
/// to handle multiple different sockets.
/// In theory in order to change a handler's socket it should only need
/// to change the pre-set event loop.
#[derive(Clone)]
pub struct PreSetEventLoop {
    /// The event loop to invoke.
    event_loop: EventLoop,

    /// The socket file descriptor to invoke the event loop with.
    fd: SocketFd,

    /// The index location of the handler for the file descriptor.
    index: usize,

    is_reading: Arc<AtomicBool>,

    is_writing: Arc<AtomicBool>,
}

impl PreSetEventLoop {
    pub fn new(event_loop: EventLoop, fd: SocketFd, index: usize) -> Self {
        Self {
            event_loop,
            fd,
            index,
            is_reading: Arc::new(AtomicBool::new(false)),
            is_writing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_fd(&mut self, fd: SocketFd) {
        self.is_reading.store(false, Ordering::Relaxed);
        self.is_writing.store(false, Ordering::Relaxed);
        self.fd = fd;
    }

    #[inline]
    fn is_reading(&self) -> bool {
        self.is_reading.load(Ordering::Relaxed)
    }

    #[inline]
    fn is_writing(&self) -> bool {
        self.is_writing.load(Ordering::Relaxed)
    }

    pub fn close_socket(&self) -> PyResult<()> {
        self.event_loop.close_socket(self.index)
    }

    /// Start monitoring the socket for read readiness.
    pub fn add_reader(&self) -> PyResult<()> {
        if !self.is_reading() {
            self.event_loop.add_reader(self.fd, self.index)?;
            self.is_reading.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Stop monitoring the socket for read readiness.
    pub fn remove_reader(&self) -> PyResult<()> {
        if self.is_reading() {
            self.event_loop.remove_reader(self.fd)?;
            self.is_reading.store(false, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Start monitoring the socket for write readiness.
    pub fn add_writer(&self) -> PyResult<()> {
        if !self.is_writing() {
            self.event_loop.add_writer(self.fd, self.index)?;
            self.is_writing.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Stops monitoring the socket for write readiness.
    pub fn remove_writer(&self) -> PyResult<()> {
        if self.is_writing() {
            self.event_loop.remove_writer(self.fd)?;
            self.is_writing.store(false, Ordering::Relaxed);
        }

        Ok(())
    }
}
