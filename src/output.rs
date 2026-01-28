use std::collections::HashMap;
use std::io::{self, Write};
use std::process::{Child, Command, ExitStatus, Stdio};

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::process::ExitStatusExt,
};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};

use crate::pager::SelectedPager;

pub type OutputStream = Box<dyn Write + Send + Sync>;

pub struct Pager {
    process: Child,
}

/// A handle that can be used to detect when the pager's stdin pipe is closed.
/// This is useful for event-based detection of pager exit.
#[cfg(unix)]
pub struct PagerPipeHandle {
    fd: RawFd,
}

#[cfg(unix)]
impl PagerPipeHandle {
    /// Returns the raw file descriptor of the pager's stdin pipe.
    /// This fd can be monitored with kqueue/poll for POLLHUP to detect pipe closure.
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

#[cfg(unix)]
impl AsRawFd for PagerPipeHandle {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Pager {
    /// Creates a new pager from a selection result.
    ///
    /// Returns `Ok(Some(Pager))` if a pager was selected and spawned successfully.
    /// Returns `Ok(None)` if no pager was selected (`SelectedPager::None`).
    /// Returns `Err` if spawning the pager process failed.
    pub fn from_selection(selection: SelectedPager) -> io::Result<Option<Self>> {
        match selection {
            SelectedPager::None => Ok(None),
            SelectedPager::Pager { command, env } => {
                let pager = Self::spawn(command, env)?;
                Ok(Some(pager))
            }
        }
    }

    /// Spawns a pager process with the given command and environment variables.
    fn spawn(command: Vec<String>, env: HashMap<String, String>) -> io::Result<Self> {
        let (executable, args) = match command.split_first() {
            Some((exe, args)) => (exe, args),
            None => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty pager command"));
            }
        };

        let mut cmd = Command::new(executable);
        cmd.args(args);

        for (key, value) in &env {
            cmd.env(key, value);
        }

        let process = cmd.stdin(Stdio::piped()).spawn()?;

        Ok(Self { process })
    }

    /// Returns a handle to the pager's stdin pipe that can be used to detect when it closes.
    #[cfg(unix)]
    pub fn pipe_handle(&self) -> Option<PagerPipeHandle> {
        self.process.stdin.as_ref().map(|stdin| PagerPipeHandle {
            fd: stdin.as_raw_fd(),
        })
    }

    #[cfg(unix)]
    fn recover(status: ExitStatus) {
        if let Some(signal) = status.signal() {
            if signal == 9 {
                eprintln!("\x1bm\nhl: pager killed");
                if stdin().is_terminal() {
                    Command::new("stty").arg("echo").status().ok();
                }
            }
        }
    }

    #[cfg(not(unix))]
    #[allow(unused_variables)]
    fn recover(status: ExitStatus) {}
}

impl Drop for Pager {
    fn drop(&mut self) {
        log::debug!("pager: drop called, waiting for process to exit");
        if let Ok(status) = self.process.wait() {
            log::debug!("pager: process exited with status: {:?}", status);
            Self::recover(status);
        } else {
            log::debug!("pager: wait() failed");
        }
        log::debug!("pager: drop finished");
    }
}

impl Write for Pager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = self.process.stdin.as_mut().unwrap().write(buf);
        if let Err(ref e) = result {
            log::debug!("pager write error: {} (kind: {:?})", e, e.kind());
        }
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.process.stdin.as_mut().unwrap().flush();
        if let Err(ref e) = result {
            log::debug!("pager flush error: {} (kind: {:?})", e, e.kind());
        }
        result
    }
}
