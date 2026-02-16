use std::collections::HashMap;
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
/// Supports two origins:
/// - **Default**: Resolves the pager command from environment variables (`PAGER`, or an
///   application-specific variable), falling back to `less`.
/// - **Custom**: Uses a pre-resolved command and environment variables, skipping all
///   resolution logic. Useful when the caller has already determined the pager command
///   (e.g., from a configuration file).
pub struct Pager {
    origin: CommandOrigin,
    env: HashMap<String, String>,
}

impl Pager {
    /// Creates a new pager configuration with default settings.
    ///
    /// Resolves the pager command from environment variables and falls back to `less`.
    pub fn new() -> Self {
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
    /// Only used with environment origin (created with [`Pager::new`]).
    pub fn env_var(mut self, name: impl Into<String>) -> Self {
        if let CommandOrigin::FromEnv { ref mut app_env_var } = self.origin {
            *app_env_var = Some(name.into());
        }
        self
    }

    /// Sets an environment variable to pass to the pager process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Sets multiple environment variables to pass to the pager process.
    pub fn envs(mut self, vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        self.env.extend(vars.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Starts the pager process.
    pub fn start(mut self) -> std::io::Result<StartedPager> {
        let (pager_path, args, apply_less_defaults) = match self.origin {
            CommandOrigin::FromEnv { app_env_var } => {
                let (path, args) = resolve(app_env_var);
                (path, args, true)
            }
            CommandOrigin::Custom { command } => {
                let (pager, args) = match command.split_first() {
                    Some((pager, args)) => (PathBuf::from(pager), args.to_vec()),
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "empty pager command",
                        ));
                    }
                };
                (pager, args, false)
            }
        };

        // Apply `less` defaults when resolved from env vars.
        if apply_less_defaults && pager_path.file_stem() == Some(&OsString::from("less")) {
            self.env.entry("LESSCHARSET".into()).or_insert("UTF-8".into());
        }

        let mut command = Command::new(&pager_path);
        command.args(&args);
        if apply_less_defaults && pager_path.file_stem() == Some(&OsString::from("less")) {
            command.arg("-R");
        }
        for (key, value) in &self.env {
            command.env(key, value);
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

/// The origin of the pager command, determining how it should be resolved.
enum CommandOrigin {
    /// Resolve pager from environment variables.
    FromEnv { app_env_var: Option<String> },
    /// Use a custom command.
    Custom { command: Vec<String> },
}

/// Resolves the pager command from environment variables, falling back to `less`.
fn resolve(app_env_var: Option<String>) -> (PathBuf, Vec<String>) {
    let mut pager = "less".to_owned();

    let app_pager = app_env_var.and_then(|v| env::var(v).ok()).filter(|v| !v.is_empty());
    if let Some(p) = app_pager {
        pager = p;
    } else if let Ok(p) = env::var("PAGER")
        && !p.is_empty()
    {
        pager = p;
    };

    let parts = shellwords::split(&pager).unwrap_or(vec![pager]);
    let (exe, args) = match parts.split_first() {
        Some((exe, args)) => (exe.clone(), args.to_vec()),
        None => (parts[0].clone(), vec![]),
    };
    let pager_path = PathBuf::from(&exe);

    (pager_path, args)
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

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pager_new_creates_default_instance() {
        let pager = Pager::new();
        assert!(matches!(pager.origin, CommandOrigin::FromEnv { app_env_var: None }));
    }

    #[test]
    fn pager_default_creates_default_instance() {
        let pager = Pager::default();
        assert!(matches!(pager.origin, CommandOrigin::FromEnv { app_env_var: None }));
    }

    #[test]
    fn pager_env_var_sets_app_env_var() {
        let pager = Pager::new().env_var("HL_PAGER");
        assert!(matches!(pager.origin, CommandOrigin::FromEnv { app_env_var: Some(ref v) } if v == "HL_PAGER"));
    }

    #[test]
    fn pager_custom_stores_command() {
        let pager = Pager::custom(["less", "-R"]);
        assert!(matches!(pager.origin, CommandOrigin::Custom { ref command } if command == &["less", "-R"]));
    }

    #[test]
    fn pager_env_sets_env_var() {
        let pager = Pager::custom(["less"]).env("LESSCHARSET", "UTF-8");
        assert_eq!(pager.env.get("LESSCHARSET"), Some(&"UTF-8".to_string()));
    }

    #[test]
    fn pager_envs_sets_multiple() {
        let pager = Pager::custom(["less"]).envs([("A", "1"), ("B", "2")]);
        assert_eq!(pager.env.get("A"), Some(&"1".to_string()));
        assert_eq!(pager.env.get("B"), Some(&"2".to_string()));
    }
}
