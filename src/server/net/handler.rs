use pyo3::prelude::*;

use crate::listener::PyreClientAddrPair;


/// The PyreClientHandler struct is what handles all the actual interactions
/// with the socket, this can be reused several times over and is designed to
/// handle concurrent pipelined requests, hopefully we can support http/2 as
/// well as http/1.1 once h11 works. :-)
#[pyclass]
pub struct PyreClientHandler {
    client: PyreClientAddrPair,
}

#[pymethods]
impl PyreClientHandler {

    /// Used to create a new handler object, generally this should only be
    /// created when absolutely needed.
    #[new]
    fn new(client: PyreClientAddrPair) -> Self {
        PyreClientHandler {
            client,
        }
    }

    /// This is used when recycle the handler objects as the memory allocations
    /// are pretty expensive and we need some way of controlling the ram usage
    /// because theres a weird leak otherwise.
    fn new_client(&mut self, client: PyreClientAddrPair) {
        self.client = client;
    }
}