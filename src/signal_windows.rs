// std imports
use std::time::Duration;

// local imports
use crate::error::*;

// ---

pub struct SignalHandler {}

impl SignalHandler {
    pub fn run<F>(_: usize, _: Duration, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        f()
    }
}
