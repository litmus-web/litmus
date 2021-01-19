use pyo3::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;


type CheapPyObject = Arc<PyObject>;


/// A helper struct that interacts with the python event loop for adding and
/// removing file descriptor listeners.
pub struct EventLoop {
    add_reader_: CheapPyObject,
    remove_reader_: CheapPyObject,

    add_writer_: CheapPyObject,
    remove_writer_: CheapPyObject,
}

impl EventLoop {
    /// Build a new event loop helper built off the given callbacks.
    pub fn new(
        _add_reader: PyObject,
        _remove_reader: PyObject,
        _add_writer: PyObject,
        _remove_writer: PyObject,
    ) -> Self {
        Self {
            add_reader_: CheapPyObject::new(_add_reader),
            remove_reader_: CheapPyObject::new(_remove_reader),
            add_writer_: CheapPyObject::new(_add_writer),
            remove_writer_: CheapPyObject::new(_remove_writer),
        }
    }
}

impl Clone for EventLoop {
    fn clone(&self) -> Self {
        Self {
            add_reader_: self.add_reader_.clone(),
            remove_reader_: self.remove_reader_.clone(),
            add_writer_: self.add_writer_.clone(),
            remove_writer_: self.remove_writer_.clone(),
        }
    }
}

impl EventLoop {
    #[cfg(target_os = "windows")]
    /// Start monitoring the fd file descriptor for read availability
    /// and invokes a callback once the fd is available for reading.
    pub fn add_reader(&self, fd: u64, index: usize) -> PyResult<()> {
        self.invoke_add(&self.add_reader_, fd, index)
    }

    #[cfg(target_os = "windows")]
    /// Stop monitoring the fd file descriptor for read availability.
    pub fn remove_reader(&self, fd: u64) -> PyResult<()> {
        self.invoke_remove(&self.remove_reader_, fd)
    }

    #[cfg(target_os = "windows")]
    /// Start monitoring the fd file descriptor for write availability
    /// and invokes a callback once the fd is available for writing.
    pub fn add_writer(&self, fd: u64, index: usize) -> PyResult<()> {
        self.invoke_add(&self.add_writer_, fd, index)
    }

    #[cfg(target_os = "windows")]
    /// Stop monitoring the fd file descriptor for write availability.
    pub fn remove_writer(&self, fd: u64) -> PyResult<()> {
        self.invoke_remove(&self.remove_writer_, fd)
    }

    #[cfg(target_os = "windows")]
    fn invoke_remove(&self, cb: &PyObject, fd: u64) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = cb.call1(py, (fd,))?;
            Ok(())
        })
    }

    #[cfg(target_os = "windows")]
    fn invoke_add(&self, cb: &PyObject, fd: u64, index: usize) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = cb.call1(py, (fd, index))?;
            Ok(())
        })
    }

    #[cfg(target_os = "unix")]
    /// Start monitoring the fd file descriptor for read availability
    /// and invokes a callback once the fd is available for reading.
    pub fn add_reader(&self, fd: i32, index: usize) -> PyResult<()> {
        self.invoke_add(&self.add_reader_, fd, index)
    }

    #[cfg(target_os = "unix")]
    /// Stop monitoring the fd file descriptor for read availability.
    pub fn remove_reader(&self, fd: i32) -> PyResult<()> {
        self.invoke_remove(&self.remove_reader_, fd)
    }

    #[cfg(target_os = "unix")]
    /// Start monitoring the fd file descriptor for write availability
    /// and invokes a callback once the fd is available for writing.
    pub fn add_writer(&self, fd: i32, index: usize) -> PyResult<()> {
        self.invoke_add(&self.add_writer_, fd, index)
    }

    #[cfg(target_os = "unix")]
    /// Stop monitoring the fd file descriptor for write availability.
    pub fn remove_writer(&self, fd: i32) -> PyResult<()> {
        self.invoke_remove(&self.remove_writer_, fd)
    }

    #[cfg(target_os = "unix")]
    fn invoke_remove(&self, cb: &PyObject, fd: i32) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = cb.call1(py, (fd,))?;
            Ok(())
        })
    }

    #[cfg(target_os = "unix")]
    fn invoke_add(&self, cb: &PyObject, fd: i32, index: usize) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = cb.call1(py, (fd, index))?;
            Ok(())
        })
    }
}


/// A pre configured event loop wrapper that has a given file descriptor and
/// index, this means it can be called without having to specify the given
/// file descriptor and index.
#[derive(Clone)]
pub struct PreSetEventLoop {
    pub event_loop: EventLoop,
    #[cfg(target_os = "unix")]
    pub fd: i32,

    #[cfg(target_os = "windows")]
    pub fd: u64,

    pub index: usize,

    pub is_reading_: Arc<AtomicBool>,
    pub is_writing_: Arc<AtomicBool>,
}


impl PreSetEventLoop {
    /// Resumes the file descriptor listener waiting for when the fd can be
    /// read from.
    pub fn resume_reading(&self) -> PyResult<()> {
        self.event_loop.add_reader(self.fd, self.index)?;
        self.is_reading_.store(true, Relaxed);

        Ok(())
    }

    /// Pauses / removes the file descriptor listener waiting for when the fd
    /// can be read from.
    pub fn pause_reading(&self) -> PyResult<()> {
        self.event_loop.remove_reader(self.fd)?;
        self.is_reading_.store(false, Relaxed);

        Ok(())
    }

    /// Resumes the file descriptor listener waiting for when the fd can be
    /// written to.
    pub fn resume_writing(&self) -> PyResult<()> {
        self.event_loop.add_writer(self.fd, self.index)?;
        self.is_writing_.store(true, Relaxed);

        Ok(())
    }

    /// Pauses / removes the file descriptor listener waiting for when the fd
    /// can be written to.
    pub fn pause_writing(&self) -> PyResult<()> {
        self.event_loop.remove_writer(self.fd)?;
        self.is_writing_.store(false, Relaxed);

        Ok(())
    }

    /// Indicates if the file descriptor is being watched for being readable.
    #[inline]
    pub fn is_reading(&self) -> bool {
        self.is_reading_.load(Relaxed)
    }

    /// Indicates if the file descriptor is being watched for being writeable.
    #[inline]
    pub fn is_writing(&self) -> bool {
        self.is_writing_.load(Relaxed)
    }
}