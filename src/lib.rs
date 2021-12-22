use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::exceptions::PyValueError;

use std::str::FromStr;
use std::time::Duration;

use log::LevelFilter;
use fern::colors::{Color, ColoredLevelConfig};

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use litmus_server::responders::{DataReceiver, DataSender};
use litmus_server::server::Server;
use litmus_server::settings::ServerSettings;


#[pyfunction]
pub fn init_logger(
    log_level: &str,
    log_file: Option<String>,
    pretty: bool,
) -> PyResult<()> {
    let level = match LevelFilter::from_str(log_level) {
        Ok(l) => l,
        Err(e) => return Err(PyValueError::new_err(e.to_string()))
    };

    let mut colours = ColoredLevelConfig::new();

    if pretty {
        colours = colours
            .info(Color::Green)
            .warn(Color::Yellow)
            .error(Color::BrightRed)
            .debug(Color::Magenta)
            .trace(Color::Cyan);
    }

    let mut builder = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} | {} | {:<5} - {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colours.color(record.level()),
                message,
            ))
        })
        .level(level)
        .level_for("compress", LevelFilter::Off)
        .chain(std::io::stdout());

    if let Some(file) = log_file {
        builder = builder.chain(fern::log_file(file)?);
    }

    let _ = builder.apply();

    Ok(())
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
    m.add_function(wrap_pyfunction!(init_logger, m)?)?;
    m.add_class::<Server>()?;
    m.add_class::<DataSender>()?;
    m.add_class::<DataReceiver>()?;
    Ok(())
}
