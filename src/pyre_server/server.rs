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


/// The max amount of items the queue can contain at once.
const QUEUE_SIZE: usize = 512;


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

    /// A pool of clients that are being managed and interacted with.
    clients: Slab<Option<Client>>,

    /// A queue of clients which are freely available.
    free_clients: ArrayQueue<usize>,

    /// The keep alive timeout duration.
    keep_alive: Duration,

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
    ) -> Self {
        let clients = Slab::with_capacity(QUEUE_SIZE);
        let free_clients = ArrayQueue::new(QUEUE_SIZE);

        Self {
            backlog,
            listener,
            event_loop_: None,
            clients,
            free_clients,
            keep_alive,
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

        if let Some(id) = self.free_clients.pop() {
            let cli = self.clients.get_mut(id)
                .expect("client did not exist at position given by queue");

            if let Some(cli) = cli {
                cli.bind_handle(
                    handle
                )?;
            } else {
                panic!("unknown memory behaviour occurring")
            };


            return Ok(())
        }

        let id = self.clients.insert(None);

        let loop_ = PreSetEventLoop {
            event_loop: self.event_loop()?.clone(),
            fd,
            index: id,
            is_reading_: Arc::new(AtomicBool::new(false)),
            is_writing_: Arc::new(AtomicBool::new(false)),
        };

        loop_.resume_reading()?;

        let cli = Client::from_handle(
            handle,
            loop_,
            self.callback.clone(),
            self.settings,
        )?;

        self.clients[id] = Some(cli);

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
            if let Some(cli) = v {
                cli.shutdown()?;
            };
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
        if let Some(cli) = self.clients[index].as_mut() {
            cli.poll_read()?;
        }
        Ok(())
    }

    /// Invoked by the python event loop when the file descriptor is ready to be
    /// written to without blocking, in this case the `index` parameter is
    /// what determines which stream is actually ready and which handler
    /// should be called as this is a global handler callback.
    fn poll_write(&mut self, index: usize) -> PyResult<()> {
        if let Some(cli) = self.clients[index].as_mut() {
            cli.poll_write()?;
        }
        Ok(())
    }

    /// Polled every x seconds where the time is equivalent to the
    /// `Server.keep_alive` duration.
    fn poll_keep_alive(&mut self)-> PyResult<()> {
        for (id, client) in self.clients.iter_mut() {
            if let Some(cli) = client {
                if !cli.idle() {
                    cli.poll_keep_alive(self.keep_alive)?;
                } else if !cli.free() {
                    let _ = self.free_clients.push(id);
                }
            }
        }
        Ok(())
    }

    /// Polled every x seconds where the time is equivalent to the
    /// `Server.idle_max` duration.
    fn poll_idle(&mut self) {
        let mut remove = Vec::new();
        for (id, client) in self.clients.iter() {
            if let Some(cli) = client {
                if !cli.idle() | !cli.free() {
                    continue
                }

                // We have enough free clients
                if let Err(_) = self.free_clients.push(id) {
                    remove.push(id);
                };
            }
        }

        for id in remove {
            self.clients.remove(id);
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




