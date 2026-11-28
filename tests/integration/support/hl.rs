use std::{
    ffi::OsStr,
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

// ---

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

// ---

/// A running `hl` process, which is stopped when dropped, so a failed test does not leave it running.
///
/// What it writes to its standard error is printed when it is dropped, as output of the test that ran it,
/// which the test harness shows only if the test fails.
pub struct Hl {
    child: Child,
    /// The entries it prints, until its output is closed.
    output: Option<mpsc::Receiver<String>>,
    /// What it writes to its standard error, once it closes it.
    errors: Option<thread::JoinHandle<Vec<u8>>>,
}

impl Hl {
    /// Starts `hl` with `args`, using the default configuration, with no pager and no colors.
    ///
    /// The `HL_*` environment variables are removed, so options set in the environment of the tests do not apply,
    /// except `HL_DEBUG_LOG*`, so its debug log can be turned on to see why a test fails.
    pub fn spawn<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self::spawn_with_delimiter(args, b'\n')
    }

    /// Starts `hl` with `args`, as [`Hl::spawn`] does, with its debug log turned on for the messages that `filter`
    /// selects, written without colors, for [`Hl::errors`] to return.
    pub fn spawn_with_debug_log<I, S>(args: I, filter: &str) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self::spawn_in_with_debug_log(&std::env::current_dir().unwrap(), args, filter)
    }

    /// Starts `hl` with `args` in the directory `dir`, as [`Hl::spawn_with_debug_log`] does.
    pub fn spawn_in_with_debug_log<I, S>(dir: &Path, args: I, filter: &str) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let (hl, stdout) = Self::spawn_unread_with(args, |command| {
            command
                .current_dir(dir)
                .env("HL_DEBUG_LOG", filter)
                .env("HL_DEBUG_LOG_STYLE", "never");
        });
        hl.read(stdout, b'\n')
    }

    /// Starts `hl` with `args`, as [`Hl::spawn`] does, splitting its output into entries at `delimiter`.
    pub fn spawn_with_delimiter<I, S>(args: I, delimiter: u8) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let (hl, stdout) = Self::spawn_unread(args);
        hl.read(stdout, delimiter)
    }

    /// Reads the entries of `stdout`, split at `delimiter`, from a thread of its own.
    fn read(mut self, stdout: ChildStdout, delimiter: u8) -> Self {
        let mut stdout = BufReader::new(stdout);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = Vec::new();
            // Once the receiver is dropped, the output is closed by returning.
            while stdout.read_until(delimiter, &mut buf).is_ok_and(|n| n != 0) {
                let entry = buf.strip_suffix(&[delimiter]).unwrap_or(&buf);
                if tx.send(String::from_utf8_lossy(entry).trim_end().to_owned()).is_err() {
                    return;
                }
                buf.clear();
            }
        });
        self.output = Some(rx);
        self
    }

    /// Starts `hl` with `args`, as [`Hl::spawn`] does, returning its output to be read by the caller.
    ///
    /// The output read by [`Hl::spawn`] is closed only once `hl` prints again, which this one does not wait for.
    pub fn spawn_unread<I, S>(args: I) -> (Self, ChildStdout)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self::spawn_unread_with(args, |_| {})
    }

    /// Starts `hl` with `args`, as [`Hl::spawn`] does, with a pipe as its standard input, whose writing end is returned.
    pub fn spawn_with_stdin<I, S>(args: I) -> (Self, ChildStdin)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let (mut hl, stdout) = Self::spawn_unread_with(args, |command| {
            command.stdin(Stdio::piped());
        });
        let stdin = hl.child.stdin.take().unwrap();
        (hl.read(stdout, b'\n'), stdin)
    }

    /// Starts `hl` with `args`, as [`Hl::spawn_unread`] does, with `command` changed by `configure` before it starts.
    fn spawn_unread_with<I, S>(args: I, configure: impl FnOnce(&mut Command)) -> (Self, ChildStdout)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hl"));
        for (name, _) in std::env::vars_os() {
            // Names of environment variables are not case-sensitive on Windows.
            let upper = name.to_string_lossy().to_ascii_uppercase();
            if upper.starts_with("HL_") && !upper.starts_with("HL_DEBUG_LOG") {
                command.env_remove(name);
            }
        }
        command.stdin(Stdio::null());
        configure(&mut command);
        let mut child = command
            .args(["--config", "-", "--paging", "never", "--color", "never"])
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let errors = thread::spawn(move || {
            let mut errors = Vec::new();
            stderr.read_to_end(&mut errors).ok();
            errors
        });
        let stdout = child.stdout.take().unwrap();
        let hl = Self {
            child,
            output: None,
            errors: Some(errors),
        };
        (hl, stdout)
    }

    /// Returns the first entry containing `text`, unless no such entry is printed within `timeout`.
    ///
    /// The entries printed before it are skipped, so waits for several entries must follow the order they are printed in.
    ///
    /// Panics if `hl` closes its output before, with its exit status if it exits within 5 seconds.
    pub fn wait_entry(&mut self, text: &str, timeout: Duration) -> Option<String> {
        let deadline = Instant::now() + timeout;
        let output = self.output.as_ref().expect("the output is closed");
        loop {
            // Checked on each entry, so entries printed without a pause do not keep it waiting past the deadline.
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match output.recv_timeout(remaining) {
                Ok(entry) if entry.contains(text) => return Some(entry),
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => return None,
                Err(mpsc::RecvTimeoutError::Disconnected) => self.closed(),
            }
        }
    }

    /// Returns the next `count` entries it prints, or fewer if the rest are not printed within `timeout`.
    ///
    /// Panics if `hl` closes its output before, with its exit status if it exits within 5 seconds.
    pub fn next_entries(&mut self, count: usize, timeout: Duration) -> Vec<String> {
        let deadline = Instant::now() + timeout;
        let output = self.output.as_ref().expect("the output is closed");
        let mut entries = Vec::with_capacity(count);
        while entries.len() < count {
            match output.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(entry) => entries.push(entry),
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => self.closed(),
            }
        }
        entries
    }

    /// Returns all the entries it prints, once it closes its output, and its exit status.
    ///
    /// Panics unless it closes its output within `timeout` and exits within 5 seconds after.
    pub fn wait_output(&mut self, timeout: Duration) -> (Vec<String>, ExitStatus) {
        let deadline = Instant::now() + timeout;
        let output = self.output.take().expect("the output is closed");
        let mut entries = Vec::new();
        loop {
            // Checked on each entry, so entries printed without a pause do not keep it waiting past the deadline.
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                panic!("hl did not close its output within {timeout:?}");
            }
            match output.recv_timeout(remaining) {
                Ok(entry) => entries.push(entry),
                Err(mpsc::RecvTimeoutError::Timeout) => panic!("hl did not close its output within {timeout:?}"),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let status = self
            .wait_exit(|| {})
            .expect("hl closed its output but is still running");
        (entries, status)
    }

    /// Panics because `hl` closed its output, with its exit status if it exits within 5 seconds.
    fn closed(&mut self) -> ! {
        match self.wait_exit(|| {}) {
            Some(status) => panic!("hl closed its output and exited with {status}"),
            None => panic!("hl closed its output but is still running"),
        }
    }

    /// Closes the output and returns the exit status of `hl`, unless it does not exit within 5 seconds.
    ///
    /// A process that waits for input, such as one following a file, finds the output closed only when it prints
    /// again, so `poke` is called until it exits to make it print.
    pub fn stop(&mut self, poke: impl FnMut()) -> Option<ExitStatus> {
        self.output = None;
        self.wait_exit(poke)
    }

    /// Returns the exit status of `hl`, unless it does not exit within 5 seconds, calling `poke` while it waits.
    pub fn wait_exit(&mut self, mut poke: impl FnMut()) -> Option<ExitStatus> {
        for _ in 0..50 {
            if let Some(status) = self.child.try_wait().unwrap() {
                return Some(status);
            }
            poke();
            thread::sleep(Duration::from_millis(100));
        }
        self.child.try_wait().unwrap()
    }

    /// Returns the CPU time `hl` has used so far, in user and system mode together.
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    pub fn cpu_time(&self) -> Duration {
        imp::cpu_time(&self.child)
    }

    /// Stops `hl` and returns what it wrote to its standard error.
    pub fn errors(mut self) -> String {
        self.terminate();
        let errors = self
            .errors
            .take()
            .and_then(|errors| errors.join().ok())
            .unwrap_or_default();
        String::from_utf8_lossy(&errors).into_owned()
    }

    /// Stops `hl` unless it has exited.
    fn terminate(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            // Asked to exit first, so that it writes its coverage data, which it does not when killed.
            #[cfg(unix)]
            {
                // SAFETY: the process is a child that has not been waited for, so its id is not reused.
                unsafe { libc::kill(self.child.id() as libc::pid_t, libc::SIGTERM) };
                let deadline = Instant::now() + Duration::from_secs(2);
                while self.child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(10));
                }
            }
            self.child.kill().ok();
            self.child.wait().ok();
        }
    }
}

impl Drop for Hl {
    fn drop(&mut self) {
        self.terminate();
        if let Some(errors) = self.errors.take().and_then(|errors| errors.join().ok())
            && !errors.is_empty()
        {
            eprint!("hl wrote to its standard error:\n{}", String::from_utf8_lossy(&errors));
        }
    }
}
