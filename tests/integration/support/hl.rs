use std::{
    ffi::OsStr,
    io::{BufRead, BufReader, Read},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// A running `hl` process, which is killed when dropped, so a failed test does not leave it running.
///
/// What it writes to its standard error is printed when it is dropped, as output of the test that ran it,
/// which the test harness shows only if the test fails.
pub struct Hl {
    child: Child,
    /// The lines it prints, until its output is closed.
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
        let mut command = Command::new(env!("CARGO_BIN_EXE_hl"));
        for (name, _) in std::env::vars_os() {
            // Names of environment variables are not case-sensitive on Windows.
            let upper = name.to_string_lossy().to_ascii_uppercase();
            if upper.starts_with("HL_") && !upper.starts_with("HL_DEBUG_LOG") {
                command.env_remove(name);
            }
        }
        let mut child = command
            .args(["--config", "-", "--paging", "never", "--color", "never"])
            .args(args)
            .stdin(Stdio::null())
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
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut line = Vec::new();
            // Once the receiver is dropped, the output is closed by returning.
            while stdout.read_until(b'\n', &mut line).is_ok_and(|n| n != 0) {
                if tx.send(String::from_utf8_lossy(&line).trim_end().to_owned()).is_err() {
                    return;
                }
                line.clear();
            }
        });
        Self {
            child,
            output: Some(rx),
            errors: Some(errors),
        }
    }

    /// Returns whether a line containing `text` is printed within `timeout`.
    ///
    /// The lines printed before it are skipped, so waits for several lines must follow the order they are printed in.
    ///
    /// Panics if `hl` closes its output before, with its exit status if it exits within 5 seconds.
    pub fn wait_line(&mut self, text: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let output = self.output.as_ref().expect("the output is closed");
        loop {
            // Checked on each line, so lines printed without a pause do not keep it waiting past the deadline.
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            match output.recv_timeout(remaining) {
                Ok(line) if line.contains(text) => return true,
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => return false,
                Err(mpsc::RecvTimeoutError::Disconnected) => match self.wait_exit(|| {}) {
                    Some(status) => panic!("hl closed its output and exited with {status}"),
                    None => panic!("hl closed its output but is still running"),
                },
            }
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
    fn wait_exit(&mut self, mut poke: impl FnMut()) -> Option<ExitStatus> {
        for _ in 0..50 {
            if let Some(status) = self.child.try_wait().unwrap() {
                return Some(status);
            }
            poke();
            thread::sleep(Duration::from_millis(100));
        }
        self.child.try_wait().unwrap()
    }
}

impl Drop for Hl {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.child.kill().ok();
            self.child.wait().ok();
        }
        if let Some(errors) = self.errors.take().and_then(|errors| errors.join().ok())
            && !errors.is_empty()
        {
            eprint!("hl wrote to its standard error:\n{}", String::from_utf8_lossy(&errors));
        }
    }
}
