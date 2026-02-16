use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};

#[cfg(unix)]
use std::{
    io::{IsTerminal, stdin},
    os::unix::process::ExitStatusExt,
};

type EnvProvider = Box<dyn Fn(&str) -> Option<String>>;

// ---

/// Pager configuration and builder.
///
/// Supports two origins:
/// - **FromEnv**: Resolves the pager command from environment variables. Checks an
///   application-specific variable first (if set), then falls back to `PAGER`.
///   Returns `None` from `start()` if no pager is configured.
/// - **Custom**: Uses a pre-resolved command and environment variables, skipping all
///   resolution logic. Useful when the caller has already determined the pager command
///   (e.g., from a configuration file).
pub struct Pager {
    origin: CommandOrigin,
    env: HashMap<String, String>,
    env_provider: EnvProvider,
}

impl Pager {
    /// Creates a new pager configuration that resolves from environment variables.
    ///
    /// Checks `PAGER` by default. Use [`lookup_var`](Pager::lookup_var) to add an
    /// application-specific variable that takes priority. Returns `None` from `start()`
    /// if no pager is configured.
    pub fn from_env() -> Self {
        Self {
            origin: CommandOrigin::FromEnv { app_env_var: None },
            env: HashMap::new(),
            env_provider: Box::new(|v| env::var(v).ok()),
        }
    }

    /// Creates a pager with a pre-resolved command, skipping environment variable resolution.
    pub fn custom(command: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            origin: CommandOrigin::Custom {
                command: command.into_iter().map(Into::into).collect(),
            },
            env: HashMap::new(),
            env_provider: Box::new(|v| env::var(v).ok()),
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

    /// Overrides the environment variable provider used to resolve pager commands.
    ///
    /// By default, [`start`](Pager::start) reads from the actual process environment
    /// using [`std::env::var`]. Use this method to inject a custom provider, for example
    /// in tests.
    ///
    /// Only used with environment origin (created with [`Pager::from_env`]).
    pub fn with_env_provider(mut self, f: impl Fn(&str) -> Option<String> + 'static) -> Self {
        self.env_provider = Box::new(f);
        self
    }

    /// Starts the pager process.
    ///
    /// Returns `None` if the origin is `FromEnv` and no pager is configured in environment variables.
    pub fn start(self) -> Option<io::Result<StartedPager>> {
        let (pager_path, args) = match &self.origin {
            CommandOrigin::FromEnv { app_env_var } => self.resolve(app_env_var.as_deref())?,
            CommandOrigin::Custom { command } => match command.split_first() {
                Some((pager, args)) => (PathBuf::from(pager), args.to_vec()),
                None => {
                    return Some(Err(io::Error::new(io::ErrorKind::InvalidInput, "empty pager command")));
                }
            },
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

        let mut process = match cmd.stdin(Stdio::piped()).spawn() {
            Ok(p) => p,
            Err(e) => return Some(Err(e)),
        };

        let stdin = process.stdin.take();

        Some(Ok(StartedPager {
            stdin,
            process: Some(process),
            command: pager_command,
        }))
    }

    /// Resolves the pager command from environment variables using this pager's env provider.
    ///
    /// Returns `None` if no pager is configured.
    fn resolve(&self, app_env_var: Option<&str>) -> Option<(PathBuf, Vec<String>)> {
        let pager = app_env_var
            .and_then(|v| (self.env_provider)(v))
            .filter(|v| !v.is_empty())
            .or_else(|| (self.env_provider)("PAGER").filter(|v| !v.is_empty()))?;

        let parts = shellwords::split(&pager).unwrap_or_else(|_| vec![pager]);
        parts
            .split_first()
            .map(|(exe, args)| (PathBuf::from(exe), args.to_vec()))
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

// ---

/// A running pager process.
///
/// Implements `Write` to pipe data into the pager's stdin.
/// On drop, closes stdin and waits for the process to exit.
pub struct StartedPager {
    stdin: Option<ChildStdin>,
    process: Option<Child>,
    command: Vec<String>,
}

impl StartedPager {
    /// Detaches the child process, returning it as a `PagerProcess`.
    ///
    /// After calling this, this pager's `Drop` will no longer wait for the process.
    /// The caller is responsible for ensuring the returned `PagerProcess` is
    /// either waited on or dropped, so that the child process and terminal state are recovered.
    pub fn detach_process(&mut self) -> Option<PagerProcess> {
        let command = self.command.clone();
        self.process.take().map(|child| PagerProcess { child, command })
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
/// its exit status.
///
/// If dropped without calling `wait`, the process is still waited on and terminal
/// state is recovered, but exit status is discarded.
pub struct PagerProcess {
    child: Child,
    command: Vec<String>,
}

impl PagerProcess {
    /// Waits for the pager process to exit and recovers terminal state.
    ///
    /// Returns the exit result containing the process exit status.
    pub fn wait(&mut self) -> io::Result<PagerExitResult> {
        let status = self.child.wait()?;
        recover(status);
        Ok(PagerExitResult {
            command: self.command.clone(),
            status,
        })
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
mod tests;
