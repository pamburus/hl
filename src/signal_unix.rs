// std imports
use std::thread::{JoinHandle, spawn};
use std::time::{Duration, Instant};

// third-party imports
use signal_hook::{
    consts::signal::{SIGINT, SIGTERM},
    iterator::{Handle, Signals},
};

// local imports
use crate::error::*;

pub use crate::signal_common::SignalExitStatus;
use crate::signal_common::{InterruptCounter, shutdown};

// ---

pub struct SignalHandler {
    signals: Handle,
    thread: Option<JoinHandle<()>>,
}

impl SignalHandler {
    pub fn run<F>(max_count: usize, timeout: Duration, status: SignalExitStatus, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let _guard = Self::new(max_count, timeout, status)?;
        f()
    }

    fn new(max_count: usize, timeout: Duration, status: SignalExitStatus) -> Result<Self> {
        let mut signals = Signals::new([SIGINT, SIGTERM])?;
        let handle = signals.handle();
        log::debug!("signal handler installed");

        let thread = spawn(move || {
            let mut interrupts = InterruptCounter::new(max_count, timeout, Instant::now());
            for signal in &mut signals {
                match signal {
                    SIGINT => {
                        log::info!("received SIGINT");
                        if interrupts.hit(Instant::now()) {
                            shutdown(signal, status);
                        }
                    }
                    // SIGTERM is a request to terminate, so it is never ignored.
                    SIGTERM => {
                        log::info!("received SIGTERM");
                        shutdown(signal, status);
                    }
                    _ => unreachable!(),
                }
            }
        });

        Ok(Self {
            signals: handle,
            thread: Some(thread),
        })
    }
}

impl Drop for SignalHandler {
    fn drop(&mut self) {
        self.signals.close();
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
