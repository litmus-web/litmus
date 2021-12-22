use bytes::BytesMut;
use pyo3::PyResult;

use crate::event_loop::PreSetEventLoop;
use crate::net::StreamHandle;
use crate::server::CallbackHandler;
use crate::settings::Settings;
use crate::transport::Transport;

pub(crate) trait Reusable: Sized {
    fn new(
        callback: CallbackHandler,
        event_loop: PreSetEventLoop,
        conn: StreamHandle,
        settings: Settings,
    ) -> PyResult<Self>;

    fn set_connection(&mut self, conn: StreamHandle) -> PyResult<()>;
}

pub trait PollHandler {
    fn poll_read(&mut self) -> PyResult<()>;
    fn poll_write(&mut self) -> PyResult<()>;
    fn poll_close(&mut self) -> PyResult<()>;
    fn poll_keep_alive(&mut self) -> PyResult<()>;
    fn shutdown(&mut self) -> PyResult<()>;
    fn is_idle(&self) -> bool;
    fn is_free(&self) -> bool;
    fn set_free(&mut self);
}

pub trait RawPollHandler {
    fn poll_read(&mut self, index: usize) -> PyResult<()>;
    fn poll_write(&mut self, index: usize) -> PyResult<()>;
    fn poll_close(&mut self, index: usize) -> PyResult<()>;
    fn poll_keep_alive(&mut self) -> PyResult<()>;
    fn shutdown(&mut self) -> PyResult<()>;
}

pub trait SocketState {
    /// A new client is being set.
    fn new_connection(&mut self, transport: Transport);

    /// The connection has been lost with the client.
    fn connection_lost(&mut self) -> PyResult<()>;

    /// The EOF has been sent by the socket.
    fn eof_received(&mut self) -> PyResult<()>;
}

/// Defined the necessary buffer handling methods.
pub trait BufferHandler: SocketState {
    /// Called when data is able to be read from the socket,
    /// the returned buffer is filled and then the read_buffer_filled
    /// callback is invoked.
    fn read_buffer_acquire(&mut self) -> PyResult<&mut BytesMut>;

    /// Called once data has been read from the socket after acquiring
    /// the buffer from read_buffer_acquire.
    fn read_buffer_filled(&mut self, amount: usize) -> PyResult<()>;

    /// Called when data is able to be written to the socket,
    /// the returned buffer is drained and written to the socket.
    /// Once all the data has been written to, or all that can be
    /// written has been. The write_buffer_drained is invoked.
    fn write_buffer_acquire(&mut self) -> PyResult<&mut BytesMut>;

    /// Called once data has been written to the socket after acquiring
    /// the buffer from write_buffer_acquire and has been successfully drained.
    fn write_buffer_drained(&mut self, amount: usize) -> PyResult<()>;
}

/// Defines the necessary methods for implementing data handling for the
/// high level protocols.
pub trait ProtocolBuffers {
    /// Invoked when data is read from the socket passing the buffer.
    fn data_received(&mut self, buffer: &mut BytesMut) -> PyResult<()>;

    /// Invoked when data is ready to be written to the socket.
    fn fill_write_buffer(&mut self, buffer: &mut BytesMut) -> PyResult<()>;
}

/// Defined the necessary methods for a transport handler
pub trait BaseTransport {
    /// Closes the connection to the socket.
    fn close(&self) -> PyResult<()>;

    /// Pauses reading of the set connection.
    fn pause_reading(&self) -> PyResult<()>;

    /// Resumes reading of the set connection.
    fn resume_reading(&self) -> PyResult<()>;

    /// Pauses writing of the set connection.
    fn pause_writing(&self) -> PyResult<()>;

    /// Resumes writing of the set connection.
    fn resume_writing(&self) -> PyResult<()>;
}
