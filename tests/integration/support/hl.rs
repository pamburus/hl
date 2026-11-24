use std::{
    ffi::OsStr,
    io::{BufRead, BufReader},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// A running `hl` process, which is killed when dropped, so a failed test does not leave it running.
pub struct Hl {
    child: Child,
    /// The lines it prints, until its output is closed.
    output: Option<mpsc::Receiver<String>>,
}

impl Hl {
    /// Starts `hl` with `args`, using the default configuration, with no pager and no colors.
    ///
    /// The `HL_*` environment variables are removed, so options set in the environment of the tests do not apply.
    pub fn spawn<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hl"));
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("HL_") {
                command.env_remove(name);
            }
        }
        let mut child = command
            .args(["--config", "-", "--paging", "never", "--color", "never"])
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
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
        }
    }

    /// Returns whether a line containing `text` is printed before `deadline`.
    ///
    /// Panics with the exit status if `hl` closes its output before.
    pub fn wait_line(&mut self, text: &str, deadline: Instant) -> bool {
        let output = self.output.as_ref().expect("the output is closed");
        loop {
            match output.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(line) if line.contains(text) => return true,
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => return false,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("hl closed its output and exited with {}", self.child.wait().unwrap())
                }
            }
        }
    }

    /// Closes the output and returns the exit status of `hl`, unless it does not exit within 5 seconds.
    ///
    /// A process that waits for input, such as one following a file, finds the output closed only when it prints
    /// again, so `poke` is called until it exits to make it print.
    pub fn stop(&mut self, mut poke: impl FnMut()) -> Option<ExitStatus> {
        self.output = None;
        (0..50).find_map(|_| {
            poke();
            thread::sleep(Duration::from_millis(100));
            self.child.try_wait().unwrap()
        })
    }
}

impl Drop for Hl {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.child.kill().ok();
            self.child.wait().ok();
        }
    }
}
