use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::process::ExitStatusExt,
};

use crate::error::*;

pub type OutputStream = Box<dyn Write + Send + Sync>;

pub struct Pager {
    stdin: Option<std::process::ChildStdin>,
    child: Option<Child>,
    monitor: Option<std::thread::JoinHandle<()>>,
}

impl Pager {
    pub fn new() -> Result<Self> {
        let mut pager = "less".to_owned();

        if let Ok(p) = env::var("HL_PAGER") {
            if !p.is_empty() {
                pager = p;
            }
        } else if let Ok(p) = env::var("PAGER") {
            if !p.is_empty() {
                pager = p;
            }
        };

        let pager = shellwords::split(&pager).unwrap_or(vec![pager]);
        let (pager, args) = match pager.split_first() {
            Some((pager, args)) => (pager, args),
            None => (&pager[0], &pager[0..0]),
        };
        let pager = PathBuf::from(pager);
        let mut command = Command::new(&pager);
        for arg in args {
            command.arg(arg);
        }
        if pager.file_stem() == Some(&OsString::from("less")) {
            command.arg("-R");
            command.env("LESSCHARSET", "UTF-8");
        }

        let mut process = command.stdin(Stdio::piped()).spawn()?;
        let stdin = process.stdin.take();

        Ok(Self {
            stdin,
            child: Some(process),
            monitor: None,
        })
    }

    /// Registers a callback to be invoked when the pager process exits.
    ///
    /// The callback runs in a background thread that waits for the child process.
    pub fn on_close<F: FnOnce() + Send + 'static>(&mut self, callback: F) {
        if let Some(mut child) = self.child.take() {
            self.monitor = Some(std::thread::spawn(move || {
                if let Ok(status) = child.wait() {
                    Self::recover(status);
                }
                callback();
            }));
        }
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
        // Close stdin first so the pager receives EOF.
        self.stdin.take();
        // Wait for child to exit if on_close was not called.
        if let Some(mut child) = self.child.take() {
            if let Ok(status) = child.wait() {
                Self::recover(status);
            }
        }
        // Wait for the monitor thread to finish if on_close was called.
        if let Some(handle) = self.monitor.take() {
            handle.join().ok();
        }
    }
}

impl Write for Pager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdin.as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdin.as_mut().unwrap().flush()
    }
}
