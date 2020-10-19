use pyo3::prelude::*;
use pyo3::{
    exceptions,
    PyAsyncProtocol,
    PyIterProtocol,
    wrap_pyfunction,
};
use pyo3::iter::IterNextOutput;

use std::collections::HashMap;
use std::sync::Arc;
use std::cell::RefCell;
use std::borrow::{Borrow, BorrowMut};

use once_cell::sync::OnceCell;


use crate::asyncio;
use crate::asgi::ASGIRunner;


const MAX_HEADERS: usize = 32;


pub struct FlowControl {
    transport: Arc<PyObject>,
    is_read_paused: bool,
    is_write_paused: bool,
}

impl FlowControl {
    fn new(transport: Arc<PyObject>) -> Self {
        FlowControl {
            transport,
            is_read_paused: false,
            is_write_paused: false,
        }
    }

    pub fn pause_reading(&mut self, py: Python) -> PyResult<()> {
        if !self.is_read_paused {
            self.is_read_paused = true;
            self.transport.call_method0(py,"pause_reading")?;
        }
        Ok(())
    }

    pub fn resume_reading(&mut self, py: Python) -> PyResult<()> {
        if self.is_read_paused {
            self.is_read_paused = false;
            self.transport.call_method0(py,"resume_reading")?;
        }
        Ok(())
    }

    pub fn pause_writing(&mut self) {
        if !self.is_write_paused {
            self.is_write_paused = true;
        }
    }

    pub fn resume_writing(&mut self) {
        if self.is_write_paused {
            self.is_write_paused = false;
        }
    }

    pub fn can_write(&mut self) -> bool {
        !self.is_write_paused
    }
}


#[pyclass]
pub struct RustProtocol {
    transport: Option<Arc<PyObject>>,
    fc:  Arc<FlowControl>,
}

#[pymethods]
impl RustProtocol {
    #[new]
    fn new(py: Python) -> PyResult<Self> {


        // NOTE ON WHY THIS IS CREATED:
        // This exists as a result of Rust's threadsafety checker
        // with PyO3 classes requiring everything in the struct to be
        // thread-safe. Rather than Creating a lock we just use this
        // dud object to fit in it's place instead of dealing with messy
        // locks and Options with mutable borrowing.
        let fc = {
            let dud = asyncio::get_loop(py)?;
            Arc::new(FlowControl::new(Arc::new(dud)))
        };

        Ok(RustProtocol{
            transport: None,
            fc,
        })
    }

    /// Called when some data is received by the asyncio server,
    /// in python this would be a bytes object the equivalent in Rust
    /// being a `&[u8]` type.
    ///
    /// This is what parses any data that it receives, MAX_HEADERS determines what the server
    /// will and will not reject or accept.
    ///
    /// TODO:
    ///     - Handling of ToManyHeaders still needs to be returned as a response
    ///       not a simple raise, otherwise this leaves us vunrable to annoying
    ///       attacks by bots and users.
    ///
    fn data_received(&mut self, py: Python, data: &[u8]) -> PyResult<()> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        let result = req.parse(data);

        let (res, body) = match result {
            Ok(r) => {
                let length_to_split = r.unwrap();
                let body = data.split_at(length_to_split).1;
                (r, body)
            },
            Err(e) => {
                return Err(
                    exceptions::PyRuntimeError::new_err(format!("{}", e.to_string()))
                )
                // todo handle as a response instead
            }
        };

        if res.is_complete() {
            let method = req.method.unwrap().to_string();
            let path = req.path.unwrap().to_string();

            let mut new_headers = HashMap::with_capacity(MAX_HEADERS);
            for header in req.headers {
                new_headers.insert(header.name.to_string(),  header.value.to_vec());
            }

            // This is just a edge case catch, transport should never be None by the time
            // any data is received however we still want to be certain.
            if self.transport.is_some() {
                let task = ASGIRunner::new(
                    method,
                    path,
                    new_headers,
                    self.transport.as_ref().unwrap().clone(),
                    self.fc.clone(),
                );
                let _ = asyncio::create_server_task(py, task)?;
            } else {
                return Err(
                    exceptions::PyRuntimeError::new_err(
                        "Transport was None type upon complete response parsing")
                )
            }


        }

        Ok(())
    }

    /// Called when the other end calls write_eof() or equivalent.
    ///
    /// If this returns a false value (including None), the transport
    /// will close itself.  If it returns a true value, closing the
    /// transport is up to the protocol.
    fn eof_received(&mut self) {  }

    /// Called when a connection is made.
    ///
    /// The argument is the transport representing the pipe connection.
    /// To receive data, wait for data_received() calls.
    /// When the connection is closed, connection_lost() is called.
    fn connection_made(&mut self, py: Python, transport: PyObject) -> PyResult<()>{
        let transport: Arc<PyObject> = Arc::new(transport);

        let flow_control: Arc<FlowControl> = Arc::new(
            FlowControl::new(transport.clone())
        );

        self.fc = flow_control;
        self.transport = Some(transport);

        Ok(())
    }

    /// Called when the connection is lost or closed.
    ///
    /// The argument is an exception object or None (the latter
    /// meaning a regular EOF is received or the connection was lol*
    /// aborted or closed).
    fn connection_lost(&mut self, exc: PyObject) {

    }

    /// Called when the transport's buffer goes over the high-water mark.
    ///
    /// Pause and resume calls are paired -- pause_writing() is called
    /// once when the buffer goes strictly over the high-water mark
    /// (even if subsequent writes increases the buffer size even
    /// more), and eventually resume_writing() is called once when the
    /// buffer size reaches the low-water mark.
    ///
    /// Note that if the buffer size equals the high-water mark,
    /// pause_writing() is not called -- it must go strictly over.
    /// Conversely, resume_writing() is called when the buffer size is
    /// equal or lower than the low-water mark.  These end conditions
    /// are important to ensure that things go as expected when either
    /// mark is zero.
    ///
    /// NOTE:
    ///     - This is the only Protocol callback that is not called
    ///       through EventLoop.call_soon() -- if it were, it would have no
    ///       effect when it's most needed (when the app keeps writing
    ///       without yielding until pause_writing() is called).
    fn pause_writing(&mut self) {
        self.pause_writing();
    }

    /// Called when the transport's buffer drains below the low-water mark.
    ///
    /// See pause_writing() for details.
    fn resume_writing(&mut self) {
        self.resume_writing();
    }

}