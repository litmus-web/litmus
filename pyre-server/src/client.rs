use pyo3::PyResult;
use std::time::Instant;

use crate::event_loop::PreSetEventLoop;
use crate::net::{SocketStatus, StreamHandle};
use crate::protocols::{AutoProtocol, Protocols};
use crate::server::CallbackHandler;
use crate::settings::Settings;
use crate::traits::{BufferHandler, PollHandler, Reusable, SocketState};
use crate::transport::Transport;

pub struct ClientHandler {
    event_loop: PreSetEventLoop,
    connection: StreamHandle,
    settings: Settings,

    protocol: AutoProtocol,

    is_free: bool,
    is_idle: bool,
    last_time: Instant,
    idle_for: Instant,
}

impl Reusable for ClientHandler {
    fn new(
        callback: CallbackHandler,
        event_loop: PreSetEventLoop,
        connection: StreamHandle,
        settings: Settings,
    ) -> PyResult<Self> {
        event_loop.add_reader()?;

        let transport = Transport::new(
            connection.addr,
            connection.server,
            connection.tls,
            event_loop.clone(),
        );

        let protocol = AutoProtocol::new(settings.clone(), Protocols::H1, transport, callback);

        Ok(Self {
            event_loop,
            connection,
            settings,
            protocol,

            is_free: false,
            is_idle: false,
            last_time: Instant::now(),
            idle_for: Instant::now(),
        })
    }

    fn set_connection(&mut self, connection: StreamHandle) -> PyResult<()> {
        self.event_loop.set_fd(connection.fd());
        self.connection = connection;

        let transport = Transport::new(
            self.connection.addr,
            self.connection.server,
            self.connection.tls,
            self.event_loop.clone(),
        );
        self.protocol.new_connection(transport);
        self.event_loop.add_reader()?;

        Ok(())
    }
}

impl PollHandler for ClientHandler {
    fn poll_read(&mut self) -> PyResult<()> {
        let buffer = self.protocol.read_buffer_acquire()?;

        let len = match self.connection.read(buffer)? {
            SocketStatus::WouldBlock => return Ok(()),
            SocketStatus::Complete(len) => len,
            SocketStatus::Disconnect => {
                self.protocol.connection_lost()?;
                self.is_idle = true;
                self.idle_for = Instant::now();
                return self.shutdown();
            }
        };

        // EOF
        if len == 0 {
            self.protocol.eof_received()?;
            return Ok(());
        }

        self.protocol.read_buffer_filled(len)?;

        self.last_time = Instant::now();

        self.protocol.maybe_switch()?;

        Ok(())
    }

    fn poll_write(&mut self) -> PyResult<()> {
        let buffer = self.protocol.write_buffer_acquire()?;

        let len = match self.connection.write(buffer)? {
            SocketStatus::WouldBlock => return Ok(()),
            SocketStatus::Complete(len) => len,
            SocketStatus::Disconnect => {
                self.protocol.connection_lost()?;
                self.is_idle = true;
                self.idle_for = Instant::now();
                return self.shutdown();
            }
        };

        self.protocol.write_buffer_drained(len)?;

        Ok(())
    }

    fn poll_close(&mut self) -> PyResult<()> {
        self.connection.close();
        self.protocol.connection_lost()?;
        Ok(())
    }

    fn poll_keep_alive(&mut self) -> PyResult<()> {
        if self.last_time.elapsed() >= self.settings.keep_alive {
            self.connection.close();
            self.is_idle = true;
            self.idle_for = Instant::now();
            return self.shutdown();
        }
        Ok(())
    }

    fn shutdown(&mut self) -> PyResult<()> {
        self.connection.close();
        self.protocol.connection_lost()?;
        Ok(())
    }

    fn is_idle(&self) -> bool {
        self.is_idle
    }

    fn is_free(&self) -> bool {
        self.is_free
    }

    fn set_free(&mut self) {
        self.is_free = true;
    }
}
