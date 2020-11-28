
// Pyo3 experimental
use pyo3::experimental::prelude::*;
use pyo3::experimental::types::experimental::Bytes as PyBytes;  // Not dealing with moving names

// Pyo3 stable stuffs
use pyo3::exceptions::{PyBlockingIOError, PyIOError, PyValueError};
use pyo3::{wrap_pyfunction, PyAsyncProtocol, PyIterProtocol};
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

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "macos")]
use std::os::unix::io::AsRawFd;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static CALLBACK: OnceCell<PyObject> = OnceCell::new();
static LOOP_CREATE_TASK: OnceCell<PyObject> = OnceCell::new();
static LOOP_REMOVE_READER: OnceCell<PyObject> = OnceCell::new();
static LOOP_REMOVE_WRITER: OnceCell<PyObject> = OnceCell::new();

const MAX_BUFFER_SIZE: usize = 512 * 1024;
const MAX_HEADERS: usize = 48;

const READER_AVAILABLE: &'static str = "POLL_READ";
const WRITER_AVAILABLE: &'static str = "POLL_WRITE";


#[pyfunction]
fn setup(
    callback: PyObject,
    create_task: PyObject,
    remove_reader: PyObject,
    remove_writer: PyObject,
) {
    CALLBACK.get_or_init(|| callback);
    LOOP_CREATE_TASK.get_or_init(|| create_task);

    LOOP_REMOVE_READER.get_or_init(|| remove_reader);
    LOOP_REMOVE_WRITER.get_or_init(|| remove_writer);
}


#[pyclass]
struct PyreListener {
    listener: TcpListener,
}

#[pymethods]
impl PyreListener {
    #[new]
    fn new(addr: &str) -> Self {
        let listener = TcpListener::bind(&addr).unwrap();
        listener.set_nonblocking(true).expect("Couldn't set non-blocking");
        println!("Serving Connections @ http://{}", addr);

        PyreListener {
            listener,
        }
    }

    fn accept(&self) -> PyResult<PyreH11Client> {
        return match self.listener.accept() {
            Ok((client, addr)) => Ok({
                let _ = client
                    .set_nonblocking(true)
                    .expect("failed to set non-blocking");

                let _ = client
                    .set_nodelay(true)
                    .expect("failed to set no-delay");

                PyreH11Client::create_client(
                    client,
                    addr,
                )
            }),
            Err(_) =>  Err(PyBlockingIOError::new_err(""))
        };
    }

    #[cfg(target_os = "windows")]
    fn fileno(&self) -> u64 {
        self.listener.as_raw_socket()
    }

    #[cfg(target_os = "linux")]
    fn fileno(&self) -> i32 {
        self.listener.as_raw_fd()
    }

    #[cfg(target_os = "macos")]
    fn fileno(&self) -> i32 {
        self.listener.as_raw_fd()
    }
}


#[pyclass]
struct PyreH11Client {
    client: TcpStream,
    addr: SocketAddr,

    // state internals
    reading: Arc<AtomicBool>,
    writing: Arc<AtomicBool>,
    parsing_complete: bool,
    response_complete: Arc<AtomicBool>,

    // callbacks
    py_resume_reading: Option<Arc<PyObject>>,
    py_resume_writing: Option<Arc<PyObject>>,

    // storage
    body: BytesMut,
    sending_body: BytesMut,

    // channels
    sending_receiver: mpsc::Receiver<BytesMut>,
    sending_sender: mpsc::SyncSender<BytesMut>,

    reading_sender: Option<mpsc::SyncSender<BytesMut>>,
}

impl PyreH11Client {
    fn create_client(
        client: TcpStream,
        addr: SocketAddr,
    ) -> Self {

        let (s_tx, s_rx) = {
            mpsc::sync_channel(1)
        };

        PyreH11Client {
            client,
            addr,

            // state internals
            reading: Arc::from(AtomicBool::new(true)),
            writing: Arc::from(AtomicBool::new(false)),
            parsing_complete: false,
            response_complete: Arc::from(AtomicBool::new(false)),

            // callbacks
            py_resume_reading: None,
            py_resume_writing: None,

            // storage
            body: BytesMut::with_capacity(MAX_BUFFER_SIZE),
            sending_body: BytesMut::with_capacity(MAX_BUFFER_SIZE),

            // channels
            sending_receiver: s_rx,
            sending_sender: s_tx,

            reading_sender: None,
        }
    }
}

#[pymethods]
impl PyreH11Client {
    /// This is the main handler of incoming loop callback events,
    /// e.g. the reader and writer callbacks. This should never be called
    /// by any external object other than the event loop.
    #[call]
    fn __call__(&mut self, event: &str) -> PyResult<()> {
        return match event {
            READER_AVAILABLE => {
                Ok(self.poll_read()?)
            },
            WRITER_AVAILABLE => {
                Ok(self.poll_write()?)
            },
            _ => Err(PyValueError::new_err("Invalid event received"))
        }
    }

    fn init(
        &mut self,
        resume_reading_cb: PyObject,
        resume_writing_cb: PyObject
    ) {
        self.py_resume_writing = Some(Arc::new(resume_writing_cb));
        self.py_resume_reading = Some(Arc::new(resume_reading_cb));
    }

    #[cfg(target_os = "windows")]
    fn fd(&self) -> u64 { self.client.as_raw_socket() }

    #[cfg(target_os = "linux")]
    fn fd(&self) -> i32 { self.client.as_raw_fd() }

    #[cfg(target_os = "macos")]
    fn fd(&self) -> i32 { self.client.as_raw_fd() }

    /// Removes the file descriptor reader that calls the `__call__` method
    /// with the `READER_AVAILABLE` event when data can be read from the socket.
    ///
    /// It acquires the GIL by itself because it doesnt require creating a
    /// reference to itself when instantiating a callback.
    fn pause_reading(&self) -> PyResult<()> {
        let remove_reader = unsafe { LOOP_REMOVE_READER.get_unchecked() };

        Python::with_gil(|py: Python| -> PyResult<()> {
            let _ = remove_reader.call1(py,(self.fd(),))?;
            Ok(())
        })?;
        self.reading.store(false, Relaxed);
        Ok(())
    }

    /// Removes the file descriptor writer that calls the `__call__` method
    /// with the `WRITER_AVAILABLE` event when data can write to the socket.
    ///
    /// It acquires the GIL by itself because it doesnt require creating a
    /// reference to itself when instantiating a callback.
    fn remove_writer(&self) -> PyResult<()> {
        let remove_writer = unsafe { LOOP_REMOVE_WRITER.get_unchecked() };

        Python::with_gil(|py: Python| -> PyResult<()> {
            let _: PyObject = remove_writer.call1(py,(self.fd(),))?;
            Ok(())
        })?;
        self.writing.store(false, Relaxed);
        Ok(())
    }
}

/// Reading events
impl PyreH11Client {
    fn close_and_cleanup(&mut self) -> PyResult<()> {
        if self.reading.load(Relaxed) {
            self.pause_reading()?;
        }

        if self.writing.load(Relaxed) {
            self.remove_writer()?;
        }
        let _ = self.client.shutdown(Both);
        Ok(())
    }

    fn reset_state(&mut self) {
        self.body.clear();
        self.sending_body.clear();
        self.parsing_complete = false;
    }

    fn poll_read(&mut self) -> PyResult<()> {
        if self.response_complete.load(Relaxed) {
            self.reset_state();
            self.response_complete.store(false, Relaxed);
        }

        let data = self.body.bytes_mut();
        let slice = unsafe {
            std::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len())
        };

        let res = match self.client.read(slice) {
            Ok(len) => {
                unsafe { self.body.advance_mut(len); }
                self.on_read_complete()?;
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

        return res
    }

    fn on_read_complete(&mut self) -> PyResult<()> {
        if !self.parsing_complete {
            self.parse()?;
        }

        if !self.body.is_empty() {
            self.on_body()?;
        }
        Ok(())
    }
}

/// Writing events
impl PyreH11Client {
    fn poll_write(&mut self) -> PyResult<()> {
        match self.sending_receiver.try_recv() {
            Ok(p) => self.sending_body.extend_from_slice(p.as_ref()),
            Err(_) => {},
        };

        let n = match self.client.write(self.sending_body.as_ref()) {
            Ok(n) => n,
            Err(ref e) if (
                (e.kind() == io::ErrorKind::ConnectionReset) |
                    (e.kind() == io::ErrorKind::ConnectionAborted) |
                    (e.kind() == io::ErrorKind::BrokenPipe)
            ) => {
                self.close_and_cleanup()?;
                return Ok(())
            },
            Err(e) => {
                self.remove_writer()?;
                return Err(PyIOError::new_err(
                    format!("{:?}", e)
                ));
            }
        };

        if n < self.sending_body.len() {
            self.sending_body = self.sending_body.split_off(n);
        } else {
            self.remove_writer()?;
            self.sending_body.clear();
        }
        Ok(())
    }
}

/// Actual Client handling
impl PyreH11Client {
    fn respond_with_error(&mut self, msg: &'static str) {
        let _ = self.client.write(format!(
            "HTTP/1.1 400 Bad Request\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/plain; charset=UTF-8\r\n\r\n\
            {}", &msg.len(), msg
        ).as_bytes());
        self.response_complete.store(true, Relaxed);
    }

    fn parse(&mut self) -> PyResult<()> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut request = httparse::Request::new(&mut headers);

        let status = match request.parse(self.body.as_ref()) {
            Ok(s) => s,
            Err(ref e) if httparse::Error::TooManyHeaders.eq(e) => {
                self.respond_with_error("To many headers");
                return Ok(())
            },
            Err(_) => {
                self.respond_with_error("Invalid request");
                return Ok(())
            }
        };

        if status.is_partial() {
            self.respond_with_error("Header block to big");
            return Ok(())
        }

        self.parsing_complete = true;

        // got to make the channels now otherwise it'll be invalid
        let (tx, rx) = {
            mpsc::sync_channel(2)
        };
        self.reading_sender = Some(tx);

        self.body = self.body.split_off(status.unwrap());

        // Prime the channel
        self.on_body()?;

        self.on_parse_complete(rx)?;
        Ok(())
    }

    fn on_parse_complete(&mut self, receiver: mpsc::Receiver<BytesMut>) -> PyResult<()> {
        let send = PySender{
            writing: self.writing.clone(),
            resume: self.py_resume_writing.as_ref().unwrap().clone(),
            sending_channel: self.sending_sender.clone(),
            body: BytesMut::new(),
            response_complete: self.response_complete.clone(),
        };

        let receiver = PyReceiver {
            reading: self.reading.clone(),
            resume: self.py_resume_reading.as_ref().unwrap().clone(),
            reading_channel: receiver,
        };

        let create_task = unsafe { LOOP_CREATE_TASK.get_unchecked() };
        Python::with_gil(|py: Python| -> PyResult<()> {
            let _ = create_task.call1(py,(send, receiver))?;
            Ok(())
        })?;
        Ok(())
    }

    fn on_body(&mut self) -> PyResult<()> {
        let send = match self.reading_sender.as_mut() {
            Some(s) => s,
            _ => return Ok(())  // this should never be none, i hope.
        };

        let sub = match send.try_send(self.body.clone()) {
            Ok(_) => {
                self.body.clear();
                return Ok(())
            },
            Err(e) => Err(PyIOError::new_err(
                format!("{:?}", e)
            )),
        };

        sub
    }
}


/// The sender callback for python this is implemented as a awaitable and
/// to get to this stage I want to die
#[pyclass]
struct PySender {
    writing: Arc<AtomicBool>,
    resume: Arc<PyObject>,
    sending_channel: mpsc::SyncSender<BytesMut>,
    body: BytesMut,
    response_complete: Arc<AtomicBool>,
}

#[pymethods]
impl PySender {
    #[call]
    fn __call__(mut slf: PyRefMut<Self>, data: Vec<u8>) -> PyResult<PyRefMut<Self>> {
        slf.body.extend(data);
        Ok(slf)
    }
}

#[pyproto]
impl PyAsyncProtocol for PySender {
    fn __await__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
}

#[pyproto]
impl PyIterProtocol for PySender {
    fn __next__(mut slf: PyRefMut<Self>) -> PyResult<IterNextOutput<(), ()>> {
        // This basically invokes the reading callback
        if !slf.writing.load(Relaxed) {
            let _ = slf.resume.call0(slf.py())?;
            slf.writing.store(true, Relaxed);
        }

        match slf.sending_channel.try_send(slf.body.clone()) {
            Ok(_) => slf.body.clear(),
            Err(_) => {
                return Ok(IterNextOutput::Yield(()));
            }
        };

        slf.response_complete.store(true, Relaxed);
        Ok(IterNextOutput::Return(()))
    }
}


/// The receiver callback for python this is implemented as a awaitable and
/// to get to this stage I want to die
#[pyclass]
struct PyReceiver {
    reading: Arc<AtomicBool>,
    resume: Arc<PyObject>,
    reading_channel: mpsc::Receiver<BytesMut>,
}

#[pymethods]
impl PyReceiver {
    #[call]
    fn __call__(slf: PyRef<Self>) -> PyResult<PyRef<Self>> {
        Ok(slf)
    }
}

#[pyproto]
impl PyAsyncProtocol for PyReceiver {
    fn __await__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
}

#[pyproto]
impl PyIterProtocol for PyReceiver {
    fn __next__(slf: PyRefMut<Self>) -> PyResult<IterNextOutput<(), Py<PyBytes>>> {
        // This basically invokes the reading callback
        if !slf.reading.load(Relaxed) {
            let _ = slf.resume.call0(slf.py())?;
            slf.reading.store(true, Relaxed);
        }

        let data = match slf.reading_channel.try_recv() {
            Ok(d) => d,
            Err(_) => {
                return Ok(IterNextOutput::Yield(()));
            }
        };

        let byte_block = Py::from(PyBytes::new(
            slf.py(),
            data.as_ref()
        ));
        Ok(IterNextOutput::Return(byte_block))
    }
}


///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn pyre(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyreListener>()?;
    m.add_class::<PyreH11Client>()?;
    m.add_function(wrap_pyfunction!(setup, m)?).unwrap();
    Ok(())
}
