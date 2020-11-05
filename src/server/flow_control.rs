use pyo3::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::asyncio;

pub struct FlowControl {
    pub transport: Arc<PyObject>,
    pub is_read_paused: AtomicBool,
    pub is_write_paused: AtomicBool,
    pub waiting_for_continue: AtomicBool,
    pub disconnected: AtomicBool,
}

impl FlowControl {
    pub fn new(transport: Arc<PyObject>) -> Self {
        FlowControl {
            transport,
            is_read_paused: AtomicBool::from(false),
            is_write_paused: AtomicBool::from(false),
            waiting_for_continue: AtomicBool::from(false),
            disconnected: AtomicBool::from(false)
        }
    }

    pub fn default(py: Python) -> PyResult<Self> {
        let loop_ = asyncio::get_loop(py)?;

        Ok(FlowControl {
            transport: Arc::new(loop_),
            is_read_paused: Default::default(),
            is_write_paused: Default::default(),
            waiting_for_continue: Default::default(),
            disconnected: Default::default(),
        })
    }

    pub fn pause_writing(&self) {
        if !self.is_write_paused.load(Ordering::Relaxed) {
            self.is_write_paused.store(true, Ordering::Relaxed);
        }
    }

    pub fn resume_writing(&self) {
        if self.is_write_paused.load(Ordering::Relaxed) {
            self.is_write_paused.store(false, Ordering::Relaxed);
        }
    }

    pub fn pause_reading(&self, py: Python) -> PyResult<()> {
        if !self.is_read_paused.load(Ordering::Relaxed) {
            self.is_read_paused.store(true, Ordering::Relaxed);
            self.transport.call_method0(py, "pause_reading")?;
        }
        Ok(())
    }

    pub fn resume_reading(&self, py: Python) -> PyResult<()> {
        if self.is_read_paused.load(Ordering::Relaxed) {
            self.is_read_paused.store(false, Ordering::Relaxed);
            self.transport.call_method0(py, "resume_reading")?;
        }
        Ok(())
    }

    pub fn is_closing(&self, py: Python) -> PyResult<bool> {
        Ok(self.transport.call_method0(py, "is_closing")?.is_true(py)?)
    }
}
