use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use rustc_hash::{FxHashMap, FxHasher};

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crate::pyre_server::client::Client;
use crate::pyre_server::net::listener::{NoneBlockingListener, Status};
use crate::pyre_server::net::stream::TcpHandle;
use crate::pyre_server::event_loop::{PreSetEventLoop, EventLoop};
use crate::pyre_server::py_callback::CallbackHandler;


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
    clients: FxHashMap<usize, Client>,

    /// The keep alive timeout duration.
    keep_alive: Duration,

    /// The timeout duration for idling clients, if a client has been
    /// inactive above this duration the client is dropped from memory.
    idle_max: Duration,

    /// The python task callback, this creates a callback task to
    /// Python when the server is ready to call it.
    callback: CallbackHandler,
}

impl Server {
    /// Create a new handler with a given backlog limit that wraps a given
    /// tcp listener and event loop helper.
    pub fn new(
        backlog: usize,
        listener: NoneBlockingListener,
        callback: CallbackHandler,

        keep_alive: Duration,
        idle_max: Duration,
    ) -> Self {

        let client_counter: usize = 0;
        let clients = FxHashMap::default();

        Self {
            backlog,

            listener,
            _event_loop: None,

            client_counter,
            clients,

            keep_alive,
            idle_max,

            callback,
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

    /// Finds the first client that is classed as 'idle' which is
    /// then selected to be used as the handler of the new connection.
    fn get_idle_client(&self) -> Option<usize> {
        for (key, client) in self.clients.iter() {
            if client.idle() {
                return Some(*key)
            }
        }

        None
    }

    /// Selects a index using either an existing idle protocol instance
    /// or making a new protocol by returning a index that does not exist in
    /// the clients hashmap.
    fn select_index(&mut self) -> usize {
        match self.get_idle_client() {
            Some(index) => index,
            None => {
                let index = self.client_counter;

                // Better increase it now
                self.client_counter += 1;

                index
            }
        }
    }

    /// An internal function that is invoked when a client has been accepted
    /// and its handle has been wrapped in a `client::Client` struct.
    fn handle_client(&mut self, handle: TcpHandle) -> PyResult<()> {
        let fd = handle.fd();

        let index = self.select_index();

        let loop_ = PreSetEventLoop {
            event_loop: self.event_loop()?.clone(),
            fd,
            index,
            is_reading_: Arc::new(AtomicBool::new(false)),
            is_writing_: Arc::new(AtomicBool::new(false)),
        };

        loop_.resume_reading()?;

        if let Some(client) = self.clients.get_mut(&index) {
            client.bind_handle(handle, loop_)?;
        } else {
            let cli = Client::from_handle(
                handle,
                loop_,
                self.callback.clone(),
            )?;

            self.clients.insert(index, cli);
        }

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

    fn len_clients(&self) -> usize {
        self.clients.len()
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

    /// Polled every x seconds where the time is equivalent to the
    /// `Server.keep_alive` duration.
    fn poll_keep_alive(&mut self)-> PyResult<()> {
        for (_, client) in self.clients.iter_mut() {
            if !client.idle() {
                client.poll_keep_alive(self.keep_alive)?;
            }
        }
        Ok(())
    }

    /// Polled every x seconds where the time is equivalent to the
    /// `Server.idle_max` duration.
    fn poll_idle(&mut self) {
        let keys: Vec<usize> = self.clients.keys().copied().collect();
        for key in keys {
            let maybe_client = self.clients.get(&key);
            if maybe_client.is_none() { continue };

            let client = maybe_client.unwrap();
            if client.idle_duration() > self.idle_max {
                self.clients.remove(&key);
            }
        }
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




