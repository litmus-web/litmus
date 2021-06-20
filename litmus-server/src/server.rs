use pyo3::prelude::*;
use pyo3::types::PyTuple;

use std::sync::Arc;

use crate::client::ClientHandler;
use crate::event_loop::EventLoop;
use crate::manager::ClientManager;
use crate::net::{NoneBlockingListener, Status};
use crate::settings::{ServerSettings, Settings};
use crate::traits::RawPollHandler;

/// A cheaply cloneable helper function that wraps a python callback.
#[derive(Clone)]
pub(crate) struct CallbackHandler {
    /// The python callback itself.
    cb: Arc<PyObject>,
}

impl CallbackHandler {
    /// Creates a new instance of this struct wrapping the PyObject in a
    /// arc to make for cheap clones.
    pub(crate) fn new(cb: PyObject) -> Self {
        Self { cb: Arc::new(cb) }
    }

    /// Invokes the callback by acquiring the gil internally.
    pub(crate) fn invoke(&self, args: impl IntoPy<Py<PyTuple>>) -> PyResult<()> {
        Python::with_gil(|py| -> PyResult<()> {
            let _ = self.cb.call1(py, args)?;
            Ok(())
        })
    }
}

#[pyclass(name = "_Server")]
pub struct Server {
    settings: Settings,

    event_loop: Option<EventLoop>,

    callback: CallbackHandler,

    listeners: Vec<NoneBlockingListener>,

    manager: Option<ClientManager<ClientHandler>>,
}

impl Server {
    #[timed::timed(duration(printer = "trace!"))]
    pub fn connect(
        settings: ServerSettings,
        callback: PyObject,
        binders: Vec<&str>,
    ) -> PyResult<Self> {
        let mut listeners = Vec::new();
        for bind in binders {
            info!("binding to {}", bind);
            let listener = NoneBlockingListener::bind(bind)?;
            listeners.push(listener);
        }

        Ok(Self {
            settings: Arc::from(settings),
            callback: CallbackHandler::new(callback),
            listeners,
            event_loop: None,
            manager: None,
        })
    }

    #[inline]
    fn event_loop(&self) -> &EventLoop {
        if let Some(el) = self.event_loop.as_ref() {
            el
        } else {
            error!("event loop not initialised, aborting execution");
            panic!("event loop uninitialised at time of calling")
        }
    }

    #[inline]
    fn manager(&mut self) -> &mut ClientManager<ClientHandler> {
        self.manager.as_mut().expect("initialised")
    }
}

#[pymethods]
impl Server {
    /// Run litmus
    fn ignite(&mut self, py: Python, accept_callback: PyObject) -> PyResult<()> {
        let start = std::time::Instant::now();
        for (index, listener) in self.listeners.iter().enumerate() {
            let fd = listener.fd();
            let _ = accept_callback.call1(py, (fd, index))?;
            info!(
                "listener on http://{} ready to accept connection",
                &listener.addr
            );
        }

        info!(
            "litmus ignited! All listeners registered in {:?}",
            start.elapsed()
        );

        Ok(())
    }

    fn init(
        &mut self,
        add_reader: PyObject,
        remove_reader: PyObject,
        add_writer: PyObject,
        remove_writer: PyObject,
        close_socket: PyObject,
    ) {
        let event_loop = EventLoop::new(
            add_reader,
            remove_reader,
            add_writer,
            remove_writer,
            close_socket,
        );

        self.event_loop.replace(event_loop);

        self.manager.replace(ClientManager::new(
            self.callback.clone(),
            self.event_loop().clone(),
            self.settings.clone(),
        ));
    }

    fn len_clients(&mut self) -> usize {
        self.manager().len_clients()
    }

    fn purge_clients(&mut self) {
        self.manager().purge_clients()
    }

    #[timed::timed(duration(printer = "trace!"))]
    fn poll_accept(&mut self, index: usize) -> PyResult<()> {
        let listener = &self.listeners[index];

        let mut accepted = Vec::new();
        let backlog = self.settings.backlog;
        for _ in 0..backlog {
            let maybe_handle = listener.accept()?;

            match maybe_handle {
                Status::Successful(conn) => accepted.push(conn),
                Status::ShouldPause => break,
            }
        }

        let manager = self.manager();
        for conn in accepted {
            manager.handle_connection(conn)?;
        }

        Ok(())
    }

    #[timed::timed(duration(printer = "trace!"))]
    fn poll_read(&mut self, index: usize) -> PyResult<()> {
        self.manager().poll_read(index)
    }

    #[timed::timed(duration(printer = "trace!"))]
    fn poll_write(&mut self, index: usize) -> PyResult<()> {
        self.manager().poll_write(index)
    }

    #[timed::timed(duration(printer = "trace!"))]
    fn poll_close(&mut self, index: usize) -> PyResult<()> {
        self.manager().poll_close(index)
    }

    fn poll_keep_alive(&mut self) -> PyResult<()> {
        self.manager().poll_keep_alive()
    }

    fn shutdown(&mut self) -> PyResult<()> {
        self.manager().shutdown()
    }
}
