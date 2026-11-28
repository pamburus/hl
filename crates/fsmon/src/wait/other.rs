use std::{convert::Infallible, fs::File, io, sync::Arc, time::Duration};

use super::{Entry, Wake};
use crate::stop::Stop;

// ---

/// Native notifications, which are not supported here, so no watch is ever made.
pub struct Watch(Infallible);

impl Watch {
    pub fn new(_entries: &[Entry], _file: Option<&File>, _stop: Arc<Stop>) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native notifications are not supported",
        ))
    }

    pub fn wait(&mut self, _timeout: Option<Duration>) -> io::Result<Wake> {
        match self.0 {}
    }
}
