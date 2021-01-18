use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use hashbrown::HashMap;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::pyre_server::client::Client;
use crate::pyre_server::net::listener::{NoneBlockingListener, Status};
use crate::pyre_server::net::stream::TcpHandle;
use crate::pyre_server::event_loop::{PreSetEventLoop, EventLoop};



/// A handler the managers all clients of a given TcpListener, controlling
/// the callbacks of file descriptor watchers, accepting the clients themselves
/// from the listener and managing the event loop interactions.
#[pyclass]
pub struct Server {
    /// The max amount of time the listener should be accepted from in
    /// a single poll_accept() callback as to improve performance.
    backlog: usize,

    /// The internal listener in which the handler is built around.
    listener: NoneBlockingListener,

    /// The key Python event loop callbacks and interactions helper.
    _event_loop: Option<EventLoop>,

    /// A internal counter for assigning new client indexes
    client_counter: usize,

    /// A pool of clients that are being managed and interacted with.
    clients: HashMap<usize, Client>,
}

impl Server {
    /// Create a new handler with a given backlog limit that wraps a given
    /// tcp listener and event loop helper.
    pub fn new(
        backlog: usize,
        listener: NoneBlockingListener,
    ) -> Self {

        let client_counter: usize = 0;
        let clients = HashMap::new();

        Self {
            backlog,

            listener,
            _event_loop: None,

            client_counter,
            clients,
        }
    }

    /// A inline method for acquiring the event loop helper, this returns an
    /// error if the system has started to handle clients and callbacks without
    /// being init first.
    #[inline]
    fn event_loop(&self) -> PyResult<&EventLoop> {
        if let Some(v) = self._event_loop.as_ref() {
            Ok(v)
        } else {
            Err(PyRuntimeError::new_err(
                "Handler has not been init before handling clients!"
            ))
        }
    }

    /// An internal function that is invoked when a client has been accepted
    /// and its handle has been wrapped in a `client::Client` struct.
    fn handle_client(&mut self, handle: TcpHandle) -> PyResult<()> {
        let index = self.client_counter;

        // Better increase it now
        self.client_counter += 1;

        let fd = handle.fd();

        let loop_ = PreSetEventLoop {
            event_loop: self.event_loop()?.clone(),
            fd,
            index,
            is_reading_: Arc::new(AtomicBool::new(false)),
            is_writing_: Arc::new(AtomicBool::new(false)),
        };

        loop_.resume_reading()?;

        let cli = Client::from_handle(handle,loop_)?;

        self.clients.insert(index, cli);

        Ok(())
    }
}

#[pymethods]
impl Server {
    /// Starts the server by adding a waiter for the listener's file descriptor
    /// for when the listener can accept a client(s).
    fn start(
        &mut self,
        py: Python,
        cb: PyObject,
        poll: PyObject
    ) -> PyResult<()> {

        let fd = self.listener.fd();
        let _ = cb.call1(py, (fd, poll))?;

        Ok(())
    }

    /// Shuts down the server and cancels all clients in the process of
    /// being handled, this also removed the file descriptor waiters.
    fn shutdown(&mut self) -> PyResult<()> {
        for (_, v) in self.clients.iter_mut() {
            v.shutdown()?;
        }

        self.event_loop()?.remove_reader(self.listener.fd())?;

        Ok(())
    }

    /// Called just after the handle has been created, this passes event loop
    /// references with it's callback pre set to itself.
    fn init(
        &mut self,
        add_reader: PyObject,
        remove_reader: PyObject,
        add_writer: PyObject,
        remove_writer: PyObject,
    ) {
        let event_loop = EventLoop::new(
            add_reader,
            remove_reader,
            add_writer,
            remove_writer,
        );

        self._event_loop = Some(event_loop);
    }

    /// Invoked by the python event loop when the file descriptor is ready to be
    /// read from without blocking, in this case the `index` parameter is
    /// what determines which stream is actually ready and which handler
    /// should be called as this is a global handler callback.
    fn poll_read(&mut self, index: usize) -> PyResult<()> {
        let client = self.clients
            .get_mut(&index)
            .expect(&format!("Expected valid client at index: {}", index));

        client.poll_read()?;

        Ok(())
    }

    /// Invoked by the python event loop when the file descriptor is ready to be
    /// written to without blocking, in this case the `index` parameter is
    /// what determines which stream is actually ready and which handler
    /// should be called as this is a global handler callback.
    fn poll_write(&mut self, index: usize) -> PyResult<()> {
        let client = self.clients
            .get_mut(&index)
            .expect(&format!("Expected valid client at index: {}", index));

        client.poll_write()?;

        Ok(())
    }

    /// Invoked by the python event loop when the listener is ready to be
    /// accepted from without blocking, because this can have multiple clients
    /// queued up we have a backlog which determines how many times we call
    /// accept on the listener before returning
    /// (draining clients from the stream).
    fn poll_accept(&mut self) -> PyResult<()> {
        for _ in 0..self.backlog {
            let maybe_handle =  self.listener.accept()?;

            let handle = match maybe_handle {
                Status::Successful(handle) => handle,
                Status::ShouldPause => break
            };

            self.handle_client(handle)?;
        }

        Ok(())
    }
}



