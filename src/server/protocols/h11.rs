/// Pyre HTTP/1.1
///
/// This contains both the protocol class itself and the flow control as well.
/// this system only handles http/1.1 protocols, other protocols will have their
/// own respective files should they be added; all protocols should interact
/// with the ASGI interface directly.
///
/// CHANGE_LOG:
///     10/10/2020 - Removed Callback from protocol creation to a static item
///                  using OnceCell, improves performance and lowers memory usage.
///
///     20/10/2020 - Moved FlowControl to use atomic booleans, how this didnt
///                  cause a compile error before the system moved to internal
///                  mutability I dont know.
///
///     22/10/2020 - Completely stripped everything out other than flow control
///
///     23/10/2020 - Re-constructed the Rust Protocol for a cleaner design.
///                  Removed SeqLock design for buffer due to Vectors not having
///                  the `Copy` trait which screws up the system.
///
///     24/10/2020 - Re-Organise and refactor names.
///
///     25/20/2020 - Separate certain tasks like error checking and extending
///                  buffer data to separate functions make the data_received()
///                  function not as long in a effort to make it more readable.
///
///     29/20/2020 - Remove all of the attempts at pointer based or reference
///                  counted sharing of the request buffer, without using
///                  locks this is pretty much impossible, the solution is to
///                  use sync_channels allowing us to also make our wait_for
///                  system.
///
///     29/20/2020 - Add Sender half error catch, this should be moved to its
///                  own function though.
///
///     30/20/2020 - Massive cleanup of parser stuff,
///
/// TO_ADD:
///     - Add a way for MAX_HEADERS to be set when the server application
///       is first made otherwise this could cause issues later on for users
///       who rely on more than 32 Headers.
///
///     - Add a response error for hitting the ToManyHeaders Error and
///       remove / only log the internal handling as this is to protect the
///       server rather than a error.
///
/// TODO:
///     - Debug why the server significantly slows down upon the send
///       transport write.
///

use pyo3::prelude::*;
use pyo3::exceptions;
use pyo3::exceptions::PyRuntimeError;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::collections::HashMap;

use bytes::{BytesMut, Bytes};

use crate::asyncio;
use crate::server::asgi;
use crate::utils;
use crate::server::flow_control::FlowControl;
use std::sync::atomic::Ordering::Relaxed;


const MAX_HEADERS: usize = 32;

/// 64KiB Chunk
const HIGH_WATER_LIMIT: usize = 64 * 1024;

/// Max amount of messages to buffer onto the channel
const CHANNEL_BUFFER_SIZE: usize = 10;

/// Standard Keep-Alive timeout
const KEEP_ALIVE_TIMEOUT: usize = 5;

type FlowControlType = Arc<FlowControl>;
type TransportType = Arc<PyObject>;


#[pyclass]
pub struct RustProtocol {
    callback: PyObject,
    flow_control: FlowControlType,
    transport: Option<TransportType>,

    parse_complete: bool,
    parser_body: BytesMut,
    parser_sender: Option<mpsc::SyncSender<BytesMut>>,

    expected_content_length: usize,
    body_length_parsed: usize,
    more_body: Arc<AtomicBool>,
}

#[pymethods]
impl RustProtocol {
    #[new]
    pub fn new(
        py: Python,
        callback: PyObject,
    ) -> PyResult<Self> {
        Ok(RustProtocol {
            callback,

            flow_control: Arc::new(FlowControl::default(py)?),
            transport: None,

            parse_complete: false,
            parser_body: BytesMut::new(),
            parser_sender: None,

            expected_content_length: 0,
            body_length_parsed: 0,
            more_body: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Called when some data is received by the asyncio server,
    /// in python this would be a bytes object the equivalent in Rust
    /// being a `&[u8]` type.
    ///
    /// This is what parses any data that it receives, MAX_HEADERS determines
    /// what the server will and will not reject or accept.
    ///
    /// TODO:
    ///     - Handling of ToManyHeaders still needs to be returned as a response
    ///       not a simple raise, otherwise this leaves us vulnerable to annoying
    ///       attacks by bots and users.
    ///
    pub fn data_received(
        &mut self,
        py: Python,
        data: &[u8],
    ) -> PyResult<()> {
        self.add_data(py, data)?;

        // Send to receive and clear body if we have finished parsing.
        // we only want to do this if we're not still parsing.
        if self.parse_complete {
            self.more_body.store(
                !self.is_message_complete(),
                Ordering::Relaxed,
            );
            self.send_to_receive(self.parser_body.clone())?;
            self.parser_body.clear();
            return Ok(())
        }

        // Handles atomically setting more body or not for the ASGI callback.
        {
            let more_body = !self.is_message_complete();
            self.more_body.store(more_body, Ordering::Relaxed);
        }

        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        let result = req.parse(self.parser_body.as_ref());

        let res = match result {
            Ok(res) => res,
            Err(_) => {
                return Ok(self.on_bad_request(py)?);
            }
        };

        if res.is_partial() {
            return Ok(self.raise_if_required()?)
        }

        let parts = utils::request_to_parts(req);
        let (
            method,
            path,
            headers,
            content_length,
        ) = match parts {
            Err(_) => return Ok(self.on_bad_request(py)?),
            Ok(p) => p,
        };

        self.expected_content_length = content_length;
        self.separate_body(res.unwrap());
        self.parse_complete = true;

        if self.transport.is_none() {
            return Err(exceptions::PyRuntimeError::new_err(
                "Transport was None at task creation."
            ))
        }

        self.create_handler_task(
            py,
            method,
            path,
            headers,
        )?;

        self.send_to_receive(self.parser_body.clone())?;
        self.parser_body.clear();

        Ok(())
    }

    fn on_bad_request(&mut self, py: Python) -> PyResult<()> {
        // todo handle properly
        return Err(exceptions::PyRuntimeError::new_err(
            "Bad Request"
        ))
    }

    /// Called when the other end calls write_eof() or equivalent.
    ///
    /// If this returns a false value (including None), the transport
    /// will close itself.  If it returns a true value, closing the
    /// transport is up to the protocol.
    pub fn eof_received(&mut self) {}

    /// Called when a connection is made.
    ///
    /// The argument is the transport representing the pipe connection.
    /// To receive data, wait for data_received() calls.
    /// When the connection is closed, connection_lost() is called.
    pub fn connection_made(&mut self, transport: PyObject) -> PyResult<()>{
        if self.transport.is_none() {
            self.transport = Some(Arc::new(transport));
        }

        Ok(())
    }

    /// Called when the connection is lost or closed.
    ///
    /// The argument is an exception object or None (the latter
    /// meaning a regular EOF is received or the connection was
    /// aborted or closed).
    pub fn connection_lost(
        &mut self,
        py: Python,
        _exc: PyObject
    ) -> PyResult<()>{
        let transport_ref = match self.transport.as_ref() {
            Some(t) => t,
            _ => return Ok(())
        };
        self.flow_control.disconnected.store(true, Relaxed);


        Ok(())
    }

    /// Called when the transport's buffer goes over the high-water mark.
    ///
    /// Pause and resume calls are paired -- `pause_writing()` is called
    /// once when the buffer goes strictly over the high-water mark
    /// (even if subsequent writes increases the buffer size even
    /// more), and eventually `resume_writing()` is called once when the
    /// buffer size reaches the low-water mark.
    ///
    /// Note that if the buffer size equals the high-water mark,
    /// `pause_writing()` is not called -- it must go strictly over.
    /// Conversely, `resume_writing()` is called when the buffer size is
    /// equal or lower than the low-water mark.  These end conditions
    /// are important to ensure that things go as expected when either
    /// mark is zero.
    ///
    /// NOTE:
    ///     - This is the only Protocol callback that is not called
    ///       through `EventLoop.call_soon()` -- if it were, it would have no
    ///       effect when it's most needed (when the app keeps writing
    ///       without yielding until `pause_writing()` is called).
    pub fn pause_writing(&self) {
        self.flow_control.pause_writing()
    }

    /// Called when the transport's buffer drains below the low-water mark.
    ///
    /// See pause_writing() for details.
    pub fn resume_writing(&self) {
        self.flow_control.resume_writing()
    }

    /// The callback given to `EventLoop.call_later()` which closes
    /// the connection when the keep alive timeout has elapsed.
    pub fn keep_alive_callback(&mut self, py: Python) -> PyResult<()> {
        let transport_ref = match self.transport.as_ref() {
            Some(t) => t,
            _ => return Ok(())
        };

        if !self.flow_control.is_closing(py)? {
            let _ = transport_ref.call_method0(py, "close")?;
        }
        Ok(())
    }
}

impl RustProtocol {
    /// Takes the parsed request details method, path and headers
    /// and produces the ASGI handler task and callback channel.
    fn create_handler_task(
        &mut self,
        py: Python,
        method: String,
        path: Bytes,
        headers: HashMap<Bytes, Bytes>,
    ) -> PyResult<()> {
        let (tx, rx) = mpsc::sync_channel(CHANNEL_BUFFER_SIZE);
        self.parser_sender = Some(tx);

        if let Some(transport) = self.transport.as_ref() {
            let fut = asgi::ASGIRunner::new(
                py,
                self.callback.clone(),
                transport.clone(),
                method,
                path,
                headers,
                self.flow_control.clone(),
                self.more_body.clone(),
                rx,
            )?;
            asyncio::create_callback_task(py, fut)?;
        }


        Ok(())
    }

    /// Appends the data to the parsing buffer and also checks if the
    /// socket needs to pause reading to avoid a overflow.
    ///
    /// If the data is being added and the sender half of the receive
    /// channel is not None this will also send that extended buffer
    /// to the receiver.
    ///
    fn add_data(&mut self, py: Python, data: &[u8]) -> PyResult<()> {
        // As well as checking the general limit, It also limits how much
        // the system will take before rejecting a request.
        self.parser_body.extend(data);
        if self.parser_body.len() >= HIGH_WATER_LIMIT {
            println!("Pausing!");
            self.flow_control.pause_reading(py)?;
        }

        // Counts the amount of body parsed overall,
        // the headers etc... are removed once the original
        // parsing is complete.
        self.body_length_parsed += self.parser_body.len();

        Ok(())
    }

    /// This what actually sends the data to the receive channel,
    /// it does not check if the channel is full or not, the default
    /// channel size is 10 and should never get close to that.
    fn send_to_receive(&self, body: BytesMut) -> PyResult<()> {
        if self.parser_sender.is_none() {
            return Ok(())
        }

        let tx = self.parser_sender
            .as_ref()
            .unwrap();

        if let Err(e) = tx.send(body) {

            // todo make this a multi line string -> ?
            return Err(exceptions::PyRuntimeError::new_err(format!(
                "Rust channel sender could not send data to receiving buffer.\n Original Error: {:?}\n",
                e.to_string()
            )))
        }
        Ok(())
    }

    /// Checks if the system should raise an error that the request
    /// is too big hitting the high water limit before finishing
    /// initial parsing.
    fn raise_if_required(&self) -> PyResult<()> {
        if self.flow_control.is_write_paused.load(Ordering::Relaxed) {
            return Err(
                exceptions::PyRuntimeError::new_err(
                    "Request size too big."
                )
            )
            // todo make this a response not a error
        }

        Ok(())
    }

    /// Simple helper function to stop the parsing section
    /// becoming too long.
    fn is_message_complete(&self) -> bool {
        self.body_length_parsed == self.expected_content_length
    }

    /// Simple helper function to stop the parsing section
    /// becoming too long.
    fn separate_body(&mut self, cut_at: usize) {

        // Remove it from the total amount parsed as this
        // determines if the body is complete excluding headers.
        self.body_length_parsed -= cut_at;

        self.parser_body = self.parser_body.split_off(cut_at);
    }
}





