use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use slab::Slab;

use crate::event_loop::{EventLoop, PreSetEventLoop};
use crate::net::StreamHandle;
use crate::server::CallbackHandler;
use crate::settings::Settings;
use crate::traits::{PollHandler, RawPollHandler, Reusable};

const MAX_QUEUE_SIZE: usize = 512;

macro_rules! get_or_reject {
    ($clients:expr, $index:expr) => {{
        if let Some(handle) = $clients[$index].as_mut() {
            Ok(handle)
        } else {
            Err(PyRuntimeError::new_err(format!(
                "client does not exist with index {}",
                $index
            )))
        }
    }};
}

pub(crate) struct ClientManager<C: Reusable + PollHandler> {
    /// The Python callback
    callback: CallbackHandler,

    /// A slab of clients.
    clients: Slab<Option<C>>,

    /// The set event loop which mirrors handles in Python.
    event_loop: EventLoop,

    /// The server configuration settings.
    settings: Settings,
}

impl<C: Reusable + PollHandler> ClientManager<C> {
    pub(crate) fn new(
        callback: CallbackHandler,
        event_loop: EventLoop,
        settings: Settings,
    ) -> Self {
        Self {
            clients: Slab::with_capacity(MAX_QUEUE_SIZE),
            callback,
            event_loop,
            settings,
        }
    }

    pub(crate) fn handle_connection(&mut self, conn: StreamHandle) -> PyResult<()> {
        let index = self.clients.insert(None);
        debug!(
            "creating new index {} for new connection: {:?}",
            index, conn.addr
        );
        let el = PreSetEventLoop::new(self.event_loop.clone(), conn.fd(), index);
        let handle = C::new(self.callback.clone(), el, conn, self.settings.clone())?;
        self.clients[index].replace(handle);

        Ok(())
    }

    pub(crate) fn len_clients(&self) -> usize {
        self.clients.len()
    }
}

impl<C: Reusable + PollHandler> RawPollHandler for ClientManager<C> {
    /// Invokes a read event on a given handler.
    ///
    /// Invoked by the python event loop when the file descriptor is ready to be
    /// read from without blocking, in this case the `index` parameter is
    /// what determines which stream is actually ready and which handler
    /// should be called as this is a global handler callback.
    fn poll_read(&mut self, index: usize) -> PyResult<()> {
        let handle = get_or_reject!(&mut self.clients, index)?;
        handle.poll_read()
    }

    /// Invokes a write event on a given handler.
    ///
    /// Invoked by the python event loop when the file descriptor is ready to be
    /// written to without blocking, in this case the `index` parameter is
    /// what determines which stream is actually ready and which handler
    /// should be called as this is a global handler callback.
    fn poll_write(&mut self, index: usize) -> PyResult<()> {
        let handle = get_or_reject!(&mut self.clients, index)?;
        handle.poll_write()
    }

    /// Invokes a close event on a given handler.
    fn poll_close(&mut self, index: usize) -> PyResult<()> {
        let handle = get_or_reject!(&mut self.clients, index)?;
        handle.poll_close()
    }

    /// Checks if any clients need to close sockets from a keep alive
    /// timeout.
    fn poll_keep_alive(&mut self) -> PyResult<()> {
        let mut remove = Vec::new();
        for (id, client) in self.clients.iter_mut() {
            if client.is_none() {
                remove.push(id);
            }

            let client = client.as_mut().unwrap();
            if !client.is_idle() {
                client.poll_keep_alive()?;
            } else if !client.is_free() {
                client.set_free();
                remove.push(id);
            }
        }

        for id in remove {
            self.clients.remove(id);
        }

        Ok(())
    }

    /// Shuts down any clients in the system and flushes the free clients.
    fn shutdown(&mut self) -> PyResult<()> {
        for (_, v) in self.clients.iter_mut() {
            if let Some(cli) = v {
                cli.shutdown()?;
            };
        }

        Ok(())
    }
}
