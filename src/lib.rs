mod utils;
mod asyncio;
mod http;

use pyo3::prelude::*;
use pyo3::{exceptions, PyAsyncProtocol, PyIterProtocol};
use pyo3::iter::IterNextOutput;

use std::collections::HashMap;

use regex::Regex;
use once_cell::sync::OnceCell;
use std::error::Error;


static CELL: OnceCell<Vec<Regex>> = OnceCell::new();

const MAX_HEADERS: usize = 32;


#[pyclass]
struct RustProtocol {
    transport: Option<PyObject>,
    loop_: PyObject,
}

#[pymethods]
impl RustProtocol {
    #[new]
    fn new(py: Python, regex_patterns: Vec<&str>) -> Self {
        let mut loop_ = asyncio::get_loop(py);

        if loop_.is_err() {
            return panic!("Cannot get event loop.");
        }

        // Set-up regex on the global scale to help efficiency.
        let _: &Vec<Regex> = CELL.get_or_init(|| {
            utils::make_regex_from_vec(regex_patterns)
        });

        RustProtocol{
            transport: None,
            loop_: loop_.unwrap(),
        }
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
                return Err(exceptions::PyRuntimeError::new_err(format!("{:?}", e.backtrace())))
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
        self.transport = Some(transport.clone());
        // let _ = asyncio::pause_reading_transport(py, &transport);



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

impl RustProtocol {
    fn start_response(&mut self, status: u16,) {
        let _ = asyncio::write_transport(
            py, &transport, "\
            HTTP/1.1 200 OK\r\n\
            date: Tue, 13 Oct 2020 20:41:26 GMT\r\n\
            server: uvicorn\r\n\
            content-type: text/plain; charset=utf-8\r\n\
            transfer-encoding: chunked\r\n".as_bytes()
        )?;
        let _ = asyncio::write_transport(
            py, &transport, "content-length: 10\r\n\r\nHelloWorld!".as_bytes()
        )?;
        let _ = asyncio::write_eof_transport(py, &transport)?;
        let _ = asyncio::close_transport(py, &transport)?;
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
