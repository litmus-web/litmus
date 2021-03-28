use std::net::SocketAddr;
use std::time::Duration;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use pyre_server::net::listener::NoneBlockingListener;
use pyre_server::py_callback::CallbackHandler;
use pyre_server::responders::receiver::DataReceiver;
use pyre_server::responders::sender::DataSender;
use pyre_server::server::Server;
use pyre_server::settings::Settings;

use pyre_framework::RouterMatcher;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;


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
    callback: PyObject,
    backlog: usize,
    keep_alive: u64,
) -> PyResult<Server> {
    let binder = format!("{}:{}", host, port);

    let socket_addr: SocketAddr = binder.clone().parse().unwrap();
    let settings = Settings::new(false, socket_addr);
    let listener = NoneBlockingListener::bind(&binder)?;
    let callback = CallbackHandler::new(callback);

    let keep_alive = Duration::from_secs(keep_alive);

    let new_handler = Server::new(
        settings,
        backlog,
        listener,
        callback,
        keep_alive,
    );

    Ok(new_handler)
}


///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn pyre(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_server, m)?)?;
    m.add_class::<Server>()?;
    m.add_class::<DataSender>()?;
    m.add_class::<DataReceiver>()?;
    m.add_class::<RouterMatcher>()?;
    Ok(())
}
