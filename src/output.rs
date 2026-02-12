use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::thread::JoinHandle;

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::process::ExitStatusExt,
};

use cancel::CancellationToken;

use crate::error::*;

pub type OutputStream = Box<dyn Write + Send + Sync>;

pub struct Pager {
    stdin: Option<std::process::ChildStdin>,
    cancellation: Arc<CancellationToken>,
    monitor: Option<JoinHandle<()>>,
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
        let cancellation = Arc::new(CancellationToken::new()?);

        let monitor = {
            let ct = cancellation.clone();
            Some(std::thread::spawn(move || {
                if let Ok(status) = process.wait() {
                    Self::recover(status);
                }
                ct.cancel();
            }))
        };

        Ok(Self {
            stdin,
            cancellation,
            monitor,
        })
    }

    pub fn cancellation_token(&self) -> Arc<CancellationToken> {
        self.cancellation.clone()
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
        // Wait for the monitor thread (which waits for the child process).
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
