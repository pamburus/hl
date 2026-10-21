// std imports
use std::io;
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

pub use crate::signal_common::SignalExitStatus;
use crate::signal_common::{InterruptCounter, shutdown};

// ---

/// Signal number reported for Ctrl-C, matching `SIGINT` on Unix.
const SIGINT: i32 = 2;

static STATE: Mutex<Option<State>> = Mutex::new(None);

pub struct SignalHandler {}

impl SignalHandler {
    pub fn run<F>(max_count: usize, timeout: Duration, status: SignalExitStatus, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let _guard = Self::new(max_count, timeout, status)?;
        f()
    }

    fn new(max_count: usize, timeout: Duration, status: SignalExitStatus) -> Result<Self> {
        *lock() = Some(State {
            interrupts: InterruptCounter::new(max_count, timeout, Instant::now()),
            status,
        });

        if unsafe { SetConsoleCtrlHandler(Some(handler), TRUE) } == FALSE {
            *lock() = None;
            return Err(io::Error::last_os_error().into());
        }

        log::debug!("signal handler installed");

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
    interrupts: InterruptCounter,
    status: SignalExitStatus,
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

    log::info!("received Ctrl-C");

    let status = match lock().as_mut() {
        Some(state) => state.interrupts.hit(Instant::now()).then_some(state.status),
        None => return FALSE,
    };

    match status {
        Some(status) => shutdown(SIGINT, status),
        None => TRUE,
    }
}
