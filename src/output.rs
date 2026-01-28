use std::collections::HashMap;
use std::io::{self, Write};
use std::process::{Child, Command, ExitStatus, Stdio};

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::process::ExitStatusExt,
};

use crate::pager::SelectedPager;

pub type OutputStream = Box<dyn Write + Send + Sync>;

pub struct Pager {
    process: Child,
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
