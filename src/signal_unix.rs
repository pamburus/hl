// std imports
use std::process::exit;
use std::thread::{spawn, JoinHandle};
use std::time::{Duration, Instant};

// third-party imports
use signal_hook::{iterator::Signals, SIGINT};

// local imports
use crate::error::*;

// ---

pub struct SignalHandler {
    signals: Signals,
    thread: Option<JoinHandle<()>>,
}

impl SignalHandler {
    pub fn run<F>(max_count: usize, timeout: Duration, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let _guard = Self::new(max_count, timeout)?;
        f()
    }

    fn new(max_count: usize, timeout: Duration) -> Result<Self> {
        let signals = Signals::new(&[SIGINT])?;
        let s = signals.clone();

        let thread = spawn(move || {
            let mut count = 0;
            let mut ts = Instant::now();
            for signal in &s {
                match signal {
                    SIGINT => {
                        if count < max_count {
                            count += 1;
                        }
                        let now = Instant::now();
                        if now.duration_since(ts) > timeout {
                            count = 0;
                        }
                        if count == max_count {
                            exit(0x80 + signal);
                        }
                        ts = now;
                    }
                    _ => unreachable!(),
                }
            }
        });

        Ok(Self {
            signals,
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
