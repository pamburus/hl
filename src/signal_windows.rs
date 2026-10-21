// std imports
use std::io;
use std::process::exit;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// third-party imports
use windows_sys::{
    Win32::{
        Foundation::{FALSE, TRUE},
        System::Console::{CTRL_C_EVENT, SetConsoleCtrlHandler},
    },
    core::BOOL,
};

// local imports
use crate::error::*;

// ---

/// Exit code used when the process is terminated by Ctrl-C, matching `0x80 + SIGINT` on Unix.
const EXIT_CODE: i32 = 0x80 + 2;

static STATE: Mutex<Option<State>> = Mutex::new(None);

pub struct SignalHandler {}

impl SignalHandler {
    pub fn run<F>(max_count: usize, timeout: Duration, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let _guard = Self::new(max_count, timeout)?;
        f()
    }

    fn new(max_count: usize, timeout: Duration) -> Result<Self> {
        *lock() = Some(State::new(max_count, timeout));

        if unsafe { SetConsoleCtrlHandler(Some(handler), TRUE) } == FALSE {
            *lock() = None;
            return Err(io::Error::last_os_error().into());
        }

        Ok(Self {})
    }
}

impl Drop for SignalHandler {
    fn drop(&mut self) {
        unsafe { SetConsoleCtrlHandler(Some(handler), FALSE) };
        *lock() = None;
    }
}

// ---

struct State {
    max_count: usize,
    timeout: Duration,
    count: usize,
    ts: Instant,
}

impl State {
    fn new(max_count: usize, timeout: Duration) -> Self {
        Self {
            max_count,
            timeout,
            count: 0,
            ts: Instant::now(),
        }
    }

    /// Registers an interrupt and returns `true` if the process should exit.
    fn interrupt(&mut self, now: Instant) -> bool {
        if self.count < self.max_count {
            self.count += 1;
        }
        if now.duration_since(self.ts) > self.timeout {
            self.count = 0;
        }
        self.ts = now;
        self.count == self.max_count
    }
}

fn lock() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

unsafe extern "system" fn handler(ctrl_type: u32) -> BOOL {
    log::debug!("console control handler received event {ctrl_type}");

    if ctrl_type != CTRL_C_EVENT {
        // Let the next handler (ultimately the default one) process other events.
        return FALSE;
    }

    let should_exit = match lock().as_mut() {
        Some(state) => state.interrupt(Instant::now()),
        None => return FALSE,
    };

    if should_exit {
        log::debug!("console control handler calling exit({EXIT_CODE})");
        exit(EXIT_CODE);
    }

    TRUE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_zero_count_exits_immediately() {
        let start = Instant::now();
        let mut state = State::new(0, Duration::from_secs(1));
        assert!(state.interrupt(start));
    }

    #[test]
    fn test_interrupt_exits_after_max_count() {
        let start = Instant::now();
        let mut state = State::new(3, Duration::from_secs(1));
        assert!(!state.interrupt(start));
        assert!(!state.interrupt(start + Duration::from_millis(100)));
        assert!(state.interrupt(start + Duration::from_millis(200)));
    }

    #[test]
    fn test_interrupt_resets_after_timeout() {
        let start = Instant::now();
        let mut state = State::new(2, Duration::from_secs(1));
        assert!(!state.interrupt(start));
        assert!(!state.interrupt(start + Duration::from_secs(2)));
        assert!(!state.interrupt(start + Duration::from_millis(2500)));
        assert!(state.interrupt(start + Duration::from_millis(3000)));
    }
}
