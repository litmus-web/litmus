/// Pyre is a HTTP written in Rust for Python, taking inspiration from the
/// ASGI interface while also building on past servers mistakes and issues.
///
/// Aims:
///     - Support HTTP/1 Protocol
///     - Support HTTP/2 Protocol
///     - Support WebSocket Protocol
///


mod pyre_server;

use pyre_server::server::Server;
use crate::pyre_server::net::listener::NoneBlockingListener;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;


/// Creates a client handler instance linked to a TcpListener and event loop.
///
/// Args:
///     host:
///         The given host string to bind to e.g. '127.0.0.1'.
///     port:
///         The given port to bind to e.g. 6060.
///     backlog:
///         The max amount of iterations to do when accepting clients
///         when the socket is ready and has been invoked.
///
/// Returns:
///     A un-initialised HandleClients instance linked to the main listener.
#[pyfunction]
fn create_server(
    host: &str,
    port: u16,
    backlog: usize,
) -> PyResult<Server> {
    let binder = format!("{}:{}", host, port);

    let listener = NoneBlockingListener::bind(&binder)?;
    let new_handler = Server::new(backlog, listener);

    Ok(new_handler)
}


///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn pyre(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_server, m)?)?;
    m.add_class::<Server>()?;
    Ok(())
}
