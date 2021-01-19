use pyo3::PyResult;

use bytes::BytesMut;


/// Defined the necessary polling methods
pub trait SocketCommunicator {
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

    /// Invoked when data is ready to be written to the socket.
    fn writing_paused(&mut self) -> PyResult<()>;
}


/// Defined the necessary methods for a transport handler
pub trait BaseTransport {
    /// Pauses reading of the set connection.
    fn pause_reading(&self) -> PyResult<()>;

    /// Resumes reading of the set connection.
    fn resume_reading(&self) -> PyResult<()>;

    /// Pauses writing of the set connection.
    fn pause_writing(&self) -> PyResult<()>;

    /// Resumes writing of the set connection.
    fn resume_writing(&self) -> PyResult<()>;

}



