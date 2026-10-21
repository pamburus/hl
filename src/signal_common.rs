// std imports
use std::process::exit;
use std::time::{Duration, Instant};

// ---

/// How the process reports its exit after a handled termination signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalExitStatus {
    /// Exit with `128 + signal`, the code a shell reports for a process killed by that signal.
    Conventional,
    /// Exit with 0, for modes where a signal is the normal way to stop, such as `--follow`.
    Success,
}

// ---

/// Terminates the process in response to `signal`.
///
/// This is the single exit point for all handled signals,
/// so any work that must be done at exit belongs here.
pub(crate) fn shutdown(signal: i32, status: SignalExitStatus) -> ! {
    let code = exit_code(signal, status);
    log::info!("shutdown complete on signal {signal}, exiting with code {code}");
    exit(code)
}

fn exit_code(signal: i32, status: SignalExitStatus) -> i32 {
    match status {
        SignalExitStatus::Conventional => 0x80 + signal,
        SignalExitStatus::Success => 0,
    }
}

// ---

/// Counts interrupts and decides when they should no longer be ignored.
pub(crate) struct InterruptCounter {
    max_count: usize,
    timeout: Duration,
    count: usize,
    ts: Instant,
}

impl InterruptCounter {
    pub(crate) fn new(max_count: usize, timeout: Duration, now: Instant) -> Self {
        Self {
            max_count,
            timeout,
            count: 0,
            ts: now,
        }
    }

    /// Registers an interrupt received at `now`.
    ///
    /// Returns `true` if the process should exit.
    pub(crate) fn hit(&mut self, now: Instant) -> bool {
        if self.count < self.max_count {
            self.count += 1;
        }
        if now.duration_since(self.ts) > self.timeout && self.count != 0 {
            log::info!("interrupt counter reset after {:?} timeout", self.timeout);
            self.count = 0;
        }
        self.ts = now;
        if self.count == self.max_count {
            return true;
        }
        log::info!("interrupt ignored ({}/{})", self.count, self.max_count);
        false
    }
}

#[cfg(test)]
mod tests;
