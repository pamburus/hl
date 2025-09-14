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
    process: Child,
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

        let process = command.stdin(Stdio::piped()).spawn()?;

        Ok(Self { process })
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
        if let Ok(status) = self.process.wait() {
            Self::recover(status);
        }
    }
}

impl Write for Pager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.process.stdin.as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.process.stdin.as_mut().unwrap().flush()
    }
}
