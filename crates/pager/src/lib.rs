use std::collections::HashMap;
use std::env;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::process::ExitStatusExt,
};

// ---

/// Pager configuration and builder.
///
/// Supports two origins:
/// - **FromEnv**: Resolves the pager command from environment variables (`PAGER`, or an
///   application-specific variable). Returns `None` from `start()` if no pager is configured.
/// - **Custom**: Uses a pre-resolved command and environment variables, skipping all
///   resolution logic. Useful when the caller has already determined the pager command
///   (e.g., from a configuration file).
pub struct Pager {
    origin: CommandOrigin,
    env: HashMap<String, String>,
}

impl Pager {
    /// Creates a new pager configuration that resolves from environment variables.
    ///
    /// Checks `PAGER` environment variable. Returns `None` from `start()` if not set.
    pub fn from_env() -> Self {
        Self {
            origin: CommandOrigin::FromEnv { app_env_var: None },
            env: HashMap::new(),
        }
    }

    /// Creates a pager with a pre-resolved command, skipping environment variable resolution.
    pub fn custom(command: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            origin: CommandOrigin::Custom {
                command: command.into_iter().map(Into::into).collect(),
            },
            env: HashMap::new(),
        }
    }

    /// Sets an application-specific environment variable to check
    /// for the pager command (e.g. `"HL_PAGER"`).
    /// Takes priority over `PAGER`.
    ///
    /// Only used with environment origin (created with [`Pager::from_env`]).
    pub fn lookup_var(mut self, name: impl Into<String>) -> Self {
        if let CommandOrigin::FromEnv { ref mut app_env_var } = self.origin {
            *app_env_var = Some(name.into());
        }
        self
    }

    /// Sets an environment variable to pass to the pager process.
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Sets multiple environment variables to pass to the pager process.
    pub fn with_env(mut self, vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        self.env.extend(vars.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Starts the pager process.
    ///
    /// Returns `None` if the origin is `FromEnv` and no pager is configured in environment variables.
    pub fn start(self) -> Option<io::Result<StartedPager>> {
        let (pager_path, args) = match self.origin {
            CommandOrigin::FromEnv { app_env_var } => resolve(app_env_var)?,
            CommandOrigin::Custom { command } => {
                let (pager, args) = match command.split_first() {
                    Some((pager, args)) => (PathBuf::from(pager), args.to_vec()),
                    None => {
                        return Some(Err(io::Error::new(io::ErrorKind::InvalidInput, "empty pager command")));
                    }
                };
                (pager, args)
            }
        };

        let pager_command = {
            let mut parts = vec![pager_path.to_string_lossy().into_owned()];
            parts.extend(args.iter().cloned());
            parts
        };

        let mut cmd = Command::new(&pager_path);
        cmd.args(&args);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        let mut process = match cmd.stdin(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(p) => p,
            Err(e) => return Some(Err(e)),
        };

        let stdin = process.stdin.take();
        let stderr_reader = process.stderr.take().map(|stderr| {
            thread::spawn(move || {
                let mut buf = String::new();
                let mut reader = io::BufReader::new(stderr);
                reader.read_to_string(&mut buf)?;
                Ok(buf)
            })
        });

        Some(Ok(StartedPager {
            stdin,
            process: Some(process),
            stderr_reader,
            command: pager_command,
        }))
    }
}

// ---

/// The origin of the pager command, determining how it should be resolved.
enum CommandOrigin {
    /// Resolve pager from environment variables.
    FromEnv { app_env_var: Option<String> },
    /// Use a custom command.
    Custom { command: Vec<String> },
}

/// Resolves the pager command from environment variables.
///
/// Returns `None` if no pager is configured.
fn resolve(app_env_var: Option<String>) -> Option<(PathBuf, Vec<String>)> {
    let pager = app_env_var
        .and_then(|v| env::var(v).ok())
        .filter(|v| !v.is_empty())
        .or_else(|| env::var("PAGER").ok().filter(|v| !v.is_empty()))?;

    let parts = shellwords::split(&pager).unwrap_or(vec![pager]);
    let (exe, args) = match parts.split_first() {
        Some((exe, args)) => (exe.clone(), args.to_vec()),
        None => (parts[0].clone(), vec![]),
    };
    let pager_path = PathBuf::from(&exe);

    Some((pager_path, args))
}

// ---

/// A running pager process.
///
/// Implements `Write` to pipe data into the pager's stdin.
/// On drop, closes stdin and waits for the process to exit.
pub struct StartedPager {
    stdin: Option<ChildStdin>,
    process: Option<Child>,
    stderr_reader: Option<JoinHandle<io::Result<String>>>,
    command: Vec<String>,
}

impl StartedPager {
    /// Detaches the child process, returning it as a `PagerProcess`.
    ///
    /// After calling this, this pager's `Drop` will no longer wait for the process.
    /// The caller is responsible for ensuring the returned `PagerProcess` is
    /// waited on or dropped to recover terminal state.
    pub fn detach_process(&mut self) -> Option<PagerProcess> {
        let command = self.command.clone();
        self.process.take().map(|child| PagerProcess {
            child,
            stderr_reader: self.stderr_reader.take(),
            command,
        })
    }

    fn stdin(&mut self) -> &mut ChildStdin {
        self.stdin.as_mut().expect("pager stdin is not available")
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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdin().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdin().flush()
    }
}

// ---

/// A detached pager child process.
///
/// Provides [`wait`](PagerProcess::wait) to explicitly wait for the process and obtain
/// its exit status along with any captured stderr output.
///
/// If dropped without calling `wait`, the process is still waited on and terminal
/// state is recovered, but exit status and stderr are discarded.
pub struct PagerProcess {
    child: Child,
    stderr_reader: Option<JoinHandle<io::Result<String>>>,
    command: Vec<String>,
}

impl PagerProcess {
    /// Waits for the pager process to exit and recovers terminal state.
    ///
    /// Returns the exit result containing the process exit status and any
    /// captured stderr output.
    pub fn wait(&mut self) -> io::Result<PagerExitResult> {
        let status = self.child.wait()?;
        recover(status);
        let stderr = self.collect_stderr();
        Ok(PagerExitResult {
            command: self.command.clone(),
            status,
            stderr,
        })
    }

    fn collect_stderr(&mut self) -> String {
        self.stderr_reader
            .take()
            .and_then(|h| h.join().ok())
            .and_then(|r| r.ok())
            .unwrap_or_default()
    }
}

impl Drop for PagerProcess {
    fn drop(&mut self) {
        if let Ok(status) = self.child.wait() {
            recover(status);
        }
    }
}

// ---

/// Result of waiting for a pager process to exit.
pub struct PagerExitResult {
    /// The command that was used to start the pager.
    pub command: Vec<String>,
    /// The exit status of the pager process.
    pub status: ExitStatus,
    /// Captured stderr output from the pager process.
    pub stderr: String,
}

impl PagerExitResult {
    /// Returns `true` if the pager exited successfully.
    ///
    /// Both exit code 0 (normal success) and 130 (user interrupted with Ctrl+C)
    /// are considered successful.
    pub fn is_success(&self) -> bool {
        self.status.success() || self.status.code() == Some(130)
    }

    /// Returns the exit code of the pager process, if available.
    pub fn exit_code(&self) -> Option<i32> {
        self.status.code()
    }

    /// Returns the signal that terminated the pager process, if any.
    #[cfg(unix)]
    pub fn signal(&self) -> Option<i32> {
        self.status.signal()
    }
}

// ---

/// Recovers terminal state after a pager process exits.
///
/// This only resets terminal state when needed (e.g., re-enabling echo after
/// the pager was killed by SIGKILL). It does not interpret exit codes or
/// make any decisions about how the application should exit.
#[cfg(unix)]
fn recover(status: ExitStatus) {
    if let Some(signal) = status.signal()
        && signal == 9
        && stdin().is_terminal()
    {
        Command::new("stty").arg("echo").status().ok();
    }
}

/// Recovers terminal state after a pager process exits (non-Unix stub).
#[cfg(not(unix))]
fn recover(_status: ExitStatus) {}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pager_from_env_creates_instance() {
        let pager = Pager::from_env();
        assert!(matches!(pager.origin, CommandOrigin::FromEnv { app_env_var: None }));
    }

    #[test]
    fn pager_env_var_sets_app_env_var() {
        let pager = Pager::from_env().lookup_var("HL_PAGER");
        assert!(matches!(pager.origin, CommandOrigin::FromEnv { app_env_var: Some(ref v) } if v == "HL_PAGER"));
    }

    #[test]
    fn pager_custom_stores_command() {
        let pager = Pager::custom(["less", "-R"]);
        assert!(matches!(pager.origin, CommandOrigin::Custom { ref command } if command == &["less", "-R"]));
    }

    #[test]
    fn pager_env_sets_env_var() {
        let pager = Pager::custom(["less"]).with_env_var("LESSCHARSET", "UTF-8");
        assert_eq!(pager.env.get("LESSCHARSET"), Some(&"UTF-8".to_string()));
    }

    #[test]
    fn pager_envs_sets_multiple() {
        let pager = Pager::custom(["less"]).with_env([("A", "1"), ("B", "2")]);
        assert_eq!(pager.env.get("A"), Some(&"1".to_string()));
        assert_eq!(pager.env.get("B"), Some(&"2".to_string()));
    }
}
