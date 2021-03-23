use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use crossbeam::queue::ArrayQueue;
use slab::Slab;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crate::pyre_server::client::Client;
use crate::pyre_server::net::listener::{NoneBlockingListener, Status};
use crate::pyre_server::net::stream::TcpHandle;
use crate::pyre_server::event_loop::{PreSetEventLoop, EventLoop};
use crate::pyre_server::py_callback::CallbackHandler;
use crate::pyre_server::settings::Settings;


const QUEUE_SIZE: usize = 512;


struct ClientManager {
    clients: Slab<Client>,
    free_clients: ArrayQueue<usize>,
}

impl ClientManager {
    fn new() -> Self {
        let clients = Slab::with_capacity(QUEUE_SIZE);
        let free_clients = ArrayQueue::new(QUEUE_SIZE);
        Self {
            clients,
            free_clients,
        }
    }

    fn get_free_client(&mut self) -> Option<&mut Client> {
        let id = self.free_clients.pop()?;
        self.clients.get_mut(id)
    }

    fn submit_client(&mut self, client: Client) {
        self.clients.insert(client);
    }

    fn check_clients(&mut self) {
        for (id, client) in self.clients.iter() {
            if client.idle() {
                self.free_clients.push(id);
            }
        }
    }

    fn clean_clients(&mut self, max_time: Duration) {
        for (id, client) in self.clients.iter() {
            if client.idle_duration() > max_time {
                self.clients.remove(id);
            }
        }
    }
}


/// A handler the managers all clients of a given TcpListener, controlling
/// the callbacks of file descriptor watchers, accepting the clients themselves
/// from the listener and managing the event loop interactions.
#[pyclass(name="_Server")]
pub struct Server {
    /// The max amount of time the listener should be accepted from in
    /// a single poll_accept() callback as to improve performance.
    backlog: usize,

    /// The internal listener in which the handler is built around.
    listener: NoneBlockingListener,

    /// The key Python event loop callbacks and interactions helper.
    event_loop_: Option<EventLoop>,

    /// A internal counter for assigning new client indexes
    client_counter: usize,

    /// A pool of clients that are being managed and interacted with.
    clients: ClientManager,

    /// The keep alive timeout duration.
    keep_alive: Duration,

    /// The timeout duration for idling clients, if a client has been
    /// inactive above this duration the client is dropped from memory.
    idle_max: Duration,

    /// The python task callback, this creates a callback task to
    /// Python when the server is ready to call it.
    callback: CallbackHandler,

    /// The server configuration used to construct a ASGI scope.
    settings: Settings,
}

impl Server {
    /// Create a new handler with a given backlog limit that wraps a given
    /// tcp listener and event loop helper.
    pub fn new(
        settings: Settings,
        backlog: usize,
        listener: NoneBlockingListener,
        callback: CallbackHandler,
        keep_alive: Duration,
        idle_max: Duration,
    ) -> Self {
        let client_counter: usize = 0;
        let clients = ClientManager::new();

        Self {
            backlog,
            listener,
            event_loop_: None,
            client_counter,
            clients,
            keep_alive,
            idle_max,
            callback,
            settings,
        }
    }

    /// A inline method for acquiring the event loop helper, this returns an
    /// error if the system has started to handle clients and callbacks without
    /// being init first.
    #[inline]
    fn event_loop(&self) -> PyResult<&EventLoop> {
        if let Some(v) = self.event_loop_.as_ref() {
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
        let fd = handle.fd();

        let loop_ = PreSetEventLoop {
            event_loop: self.event_loop()?.clone(),
            fd,
            index,
            is_reading_: Arc::new(AtomicBool::new(false)),
            is_writing_: Arc::new(AtomicBool::new(false)),
        };

        loop_.resume_reading()?;

        if let Some(client) = self.clients.get_free_client() {
            client.bind_handle(handle)?;
        } else {
            let cli = Client::from_handle(
                handle,
                loop_,
                self.callback.clone(),
                self.settings,
            )?;

            self.clients.submit_client(cli);
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

        self.event_loop_ = Some(event_loop);
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




