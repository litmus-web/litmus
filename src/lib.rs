extern crate pretty_env_logger;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use std::env;
use std::time::Duration;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use litmus_server::responders::{DataReceiver, DataSender};
use litmus_server::server::Server;
use litmus_server::settings::ServerSettings;


#[pyfunction]
pub fn set_log_level(log_level: &str) {
    let _ = env::set_var("RUST_LOG", log_level);
}

#[pyfunction]
pub fn init_logger() {
    pretty_env_logger::init();
}


#[pyfunction]
pub fn create_server(
    callback: PyObject,
    binders: Vec<&str>,
    backlog: usize,
    keep_alive: u64,
) -> PyResult<Server> {
    let settings = ServerSettings {
        backlog,
        keep_alive: Duration::from_secs(keep_alive),
    };

    let server = Server::connect(settings, callback, binders)?;

    Ok(server)
}

#[pymodule]
fn litmus(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_server, m)?)?;
    m.add_function(wrap_pyfunction!(set_log_level, m)?)?;
    m.add_function(wrap_pyfunction!(init_logger, m)?)?;
    m.add_class::<Server>()?;
    m.add_class::<DataSender>()?;
    m.add_class::<DataReceiver>()?;
    Ok(())
}
