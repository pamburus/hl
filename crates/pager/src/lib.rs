use std::collections::HashMap;
use std::env;
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
    pub fn start(self) -> Option<std::io::Result<StartedPager>> {
        let (pager_path, args) = match self.origin {
            CommandOrigin::FromEnv { app_env_var } => resolve(app_env_var)?,
            CommandOrigin::Custom { command } => {
                let (pager, args) = match command.split_first() {
                    Some((pager, args)) => (PathBuf::from(pager), args.to_vec()),
                    None => {
                        return Some(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "empty pager command",
                        )));
                    }
                };
                (pager, args)
            }
        };

        let mut command = Command::new(&pager_path);
        command.args(&args);
        for (key, value) in &self.env {
            command.env(key, value);
        }

        let mut process = match command.stdin(Stdio::piped()).spawn() {
            Ok(p) => p,
            Err(e) => return Some(Err(e)),
        };
        let stdin = process.stdin.take();

        Some(Ok(StartedPager {
            stdin,
            process: Some(process),
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
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdin().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdin().flush()
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
            if let Some(code) = recover(status) {
                std::process::exit(code);
            }
        }
    }
}

// ---

#[cfg(unix)]
fn recover(status: ExitStatus) -> Option<i32> {
    if let Some(signal) = status.signal() {
        if signal == 9 {
            eprintln!("\x1bm\nhl: pager killed");
            if stdin().is_terminal() {
                Command::new("stty").arg("echo").status().ok();
            }
        }
        // Exit with 128 + signal number (standard Unix convention)
        // SIGPIPE is 13, so 128 + 13 = 141 (matches git behavior)
        return Some(128 + signal);
    } else if let Some(code) = status.code() {
        if code != 0 {
            // When pager exits with non-zero, exit with 141 (SIGPIPE convention)
            // This matches git's behavior when the pager fails
            return Some(141);
        }
    }
    None
}

#[cfg(not(unix))]
fn recover(status: ExitStatus) -> Option<i32> {
    if let Some(code) = status.code() {
        if code != 0 {
            // When pager exits with non-zero, exit with 141 (SIGPIPE convention)
            // This matches git's behavior when the pager fails
            return Some(141);
        }
    }
    None
}

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
