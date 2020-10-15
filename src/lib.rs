mod utils;
mod asyncio;
mod http;

use pyo3::prelude::*;
use pyo3::{exceptions, PyAsyncProtocol, PyIterProtocol};
use pyo3::iter::IterNextOutput;

use std::collections::HashMap;

use regex::Regex;
use once_cell::sync::OnceCell;


// static URL_REGEX: OnceCell<Vec<Regex>> = OnceCell::new();

const MAX_HEADERS: usize = 32;


struct FlowControl {
    transport: PyObject,
    is_read_paused: bool,
    is_write_paused: bool,
    wait_for_write: bool,
}

impl FlowControl {
    fn pause_reading(&mut self, py: Python) {
        if !self.is_read_paused {
            self.is_read_paused = true;
            self.transport.call_method0(py, "pause_reading");
        }
    }

    fn resume_reading(&mut self, py: Python) {
        if self.is_read_paused {
            self.is_read_paused = false;
            let _ = self.transport.call_method0(py, "resume_reading");
        }
    }

    fn pause_writing(&mut self) {
        if !self.is_write_paused {
            self.is_write_paused = true;
            self.wait_for_write = true;
        }
    }

    fn resume_writing(&mut self) {
        if self.is_write_paused {
            self.is_write_paused = false;
            self.wait_for_write = false;
        }
    }

    fn can_write(&mut self) -> bool {
        !self.wait_for_write
    }
}


#[pyclass]
struct RustProtocol {
    transport: PyObject,
    loop_: PyObject,
}

#[pymethods]
impl RustProtocol {
    #[new]
    fn new(py: Python, _regex_patterns: Vec<&str>) -> PyResult<Self> {

        // get the running event loop from python
        let mut loop_ = asyncio::get_loop(py);
        if loop_.is_err() {
            return Err(exceptions::PyRuntimeError::new_err("Cannot get event loop."))
        }

        // uses the event loop just as a dud object, to get
        // overridden later.
        let dud = asyncio::get_loop(py);
        if dud.is_err() {
            return Err(exceptions::PyRuntimeError::new_err("Cannot get event loop."))
        }

        // Set-up regex on the global scale to help efficiency.
        //let _: &Vec<Regex> = URL_REGEX.get_or_init(|| {
        //    utils::make_regex_from_vec(regex_patterns)
        //});

        Ok(RustProtocol{
            transport: dud.unwrap(),
            loop_: loop_.unwrap(),
        })
    }

    /// Called when some data is received.
    ///
    /// The argument is a bytes object.
    fn data_received(&mut self, py: Python, data: &[u8]) -> PyResult<()> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        let result = req.parse(data);
        let res = match result {
            Ok(r) => r,
            Err(e) => {
                return Err(exceptions::PyRuntimeError::new_err(format!("{}", e.to_string())))
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

            let task = RequestResponseCycle::new(
                method,
                path,
                new_headers
            );
            let _ = asyncio::create_server_task(py, task);

            self.start_response(
                py,
                200,
                vec![(b"content-type", b"text/plain"), (b"content-length", b"0")]
            )?;
        }

        Ok(())
    }

    /// Called when the other end calls write_eof() or equivalent.
    ///
    /// If this returns a false value (including None), the transport
    /// will close itself.  If it returns a true value, closing the
    /// transport is up to the protocol.
    fn eof_received(&mut self) {

    }

    /// Called when a connection is made.
    ///
    /// The argument is the transport representing the pipe connection.
    /// To receive data, wait for data_received() calls.
    /// When the connection is closed, connection_lost() is called.
    fn connection_made(&mut self, py: Python, transport: PyObject) -> PyResult<()>{
        self.transport = transport;
        Ok(())
    }

    /// Called when the connection is lost or closed.
    ///
    /// The argument is an exception object or None (the latter
    /// meaning a regular EOF is received or the connection was
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
    /// NOTE: This is the only Protocol callback that is not called
    /// through EventLoop.call_soon() -- if it were, it would have no
    /// effect when it's most needed (when the app keeps writing
    /// without yielding until pause_writing() is called).
    fn pause_writing(&mut self) {

    }

    /// Called when the transport's buffer drains below the low-water mark.
    ///
    ///  See pause_writing() for details.
    fn resume_writing(&mut self) {

    }


}

/// Area used for handling the writing to the socket.
/// contains:
///     - start_response()
///     - send_body()
///     - send_end()
///
impl RustProtocol {

    /// start response is used for handling writing the status code and
    /// related headers, similar to the ASGI response.start system.
    ///
    /// If the status is not a valid code as described in http::get_status...
    /// the system will return '' as a &str, otherwise it will return the HTTP
    /// status line e.g 200 -> 200 OK
    ///
    /// Headers should follow the lines of the ASGI system taking a vector (python list)
    /// containing a tuple of byte strings.
    ///
    fn start_response(
        &mut self,
        py: Python,
        status: u16,
        headers: Vec<(&[u8], &[u8])>,
    ) -> PyResult<()> {
        let status_line = http::get_status_from_u16(status);

        // Check if its not the default
        if status_line == "" {
            return Err(
                exceptions::PyRuntimeError::new_err(
                    format!("Status code {:?} is not a recognised code.", status)))
        }

        // Main block to be sent
        let mut parts: Vec<Vec<u8>> = Vec::default();

        // First line containing protocol and Status
        let first_line = Vec::from(format!("HTTP/1.1 {}", status_line));
        parts.push(first_line);

        let mut part: Vec<u8>;
        for (name, value) in headers {
            part = [name, value].join(": ".as_bytes());
            parts.push(part);
        }
        parts.push(Vec::from("\r\n".as_bytes()));

        let header_block: Vec<u8> = parts.join("\r\n".as_bytes());

        let _ = asyncio::write_transport(py, &self.transport, header_block.as_ref())?;

        Ok(())
    }

    fn send_body(&mut self, py: Python, body: &[u8]) -> PyResult<()> {
        Ok(asyncio::write_transport(py, &self.transport, body)?)
    }


    fn send_end(&mut self, py: Python) -> PyResult<()> {
        Ok(asyncio::write_eof_transport(py, &self.transport)?)
    }
}



#[pyclass]
pub struct RequestResponseCycle {
    method: String,
    path: String,
    headers: HashMap<String, Vec<u8>>
}

#[pymethods]
impl RequestResponseCycle {
    #[new]
    fn new(
        method: String,
        path: String,
        headers: HashMap<String, Vec<u8>>,
    ) -> Self {
        RequestResponseCycle {
            method,
            path,
            headers
        }
    }
}

#[pyproto]
impl PyAsyncProtocol for RequestResponseCycle {
    fn __await__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
}

#[pyproto]
impl PyIterProtocol for RequestResponseCycle {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> PyResult<IterNextOutput<Option<PyObject>, Option<PyObject>>> {
        Ok(IterNextOutput::Return(None))
    }
}



///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn _pyre(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<RustProtocol>()?;
    m.add_class::<RequestResponseCycle>()?;
    Ok(())
}
