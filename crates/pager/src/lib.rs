use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::process::ExitStatusExt,
};

// ---

/// Pager configuration and builder.
///
/// Resolves the pager command from environment variables and starts the process.
/// Checks an optional application-specific env var first, then `PAGER`,
/// falling back to `less`.
pub struct Pager {
    app_env_var: Option<String>,
}

impl Pager {
    /// Creates a new pager configuration with default settings.
    pub fn new() -> Self {
        Self { app_env_var: None }
    }

    /// Sets an application-specific environment variable to check
    /// for the pager command (e.g. `"HL_PAGER"`).
    /// Takes priority over `PAGER`.
    pub fn env_var(mut self, name: impl Into<String>) -> Self {
        self.app_env_var = Some(name.into());
        self
    }

    /// Starts the pager process.
    pub fn start(self) -> std::io::Result<StartedPager> {
        let mut pager = "less".to_owned();

        let app_pager = self
            .app_env_var
            .and_then(|v| env::var(v).ok())
            .filter(|v| !v.is_empty());
        if let Some(p) = app_pager {
            pager = p;
        } else if let Ok(p) = env::var("PAGER")
            && !p.is_empty()
        {
            pager = p;
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

        Ok(StartedPager {
            stdin,
            process: Some(process),
        })
    }
}

impl Default for Pager {
    fn default() -> Self {
        Self::new()
    }
}

// ---

/// A running pager process.
///
/// Implements `Write` to pipe data into the pager's stdin.
/// On drop, closes stdin and waits for the process to exit.
pub struct StartedPager {
    stdin: Option<ChildStdin>,
    process: Option<Child>,
}

impl StartedPager {
    /// Detaches the child process, returning it as a `PagerProcess`.
    ///
    /// After calling this, this pager's `Drop` will no longer wait for the process.
    /// The caller is responsible for ensuring the returned `PagerProcess` is
    /// dropped to wait for the child and recover terminal state.
    pub fn detach_process(&mut self) -> Option<PagerProcess> {
        self.process.take().map(PagerProcess)
    }
}

impl Drop for StartedPager {
    fn drop(&mut self) {
        // Close stdin first to signal EOF to the pager.
        self.stdin.take();

        // Wait for the process if it hasn't been detached.
        if let Some(mut process) = self.process.take()
            && let Ok(status) = process.wait()
        {
            recover(status);
        }
    }
}

impl Write for StartedPager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdin
            .as_mut()
            .expect("pager stdin is not available")
            .write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdin
            .as_mut()
            .expect("pager stdin is not available")
            .flush()
    }
}

// ---

/// A detached pager child process.
///
/// When dropped, waits for the child process to exit and recovers terminal state.
pub struct PagerProcess(Child);

impl Drop for PagerProcess {
    fn drop(&mut self) {
        if let Ok(status) = self.0.wait() {
            recover(status);
        }
    }
}

// ---

#[cfg(unix)]
fn recover(status: ExitStatus) {
    if let Some(signal) = status.signal()
        && signal == 9
    {
        eprintln!("\x1bm\nhl: pager killed");
        if stdin().is_terminal() {
            Command::new("stty").arg("echo").status().ok();
        }
    }
}

#[cfg(not(unix))]
#[allow(unused_variables)]
fn recover(status: ExitStatus) {}
