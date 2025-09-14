// std imports
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

// third-party imports
use once_cell::sync::Lazy;

// local imports
use crate::{
    appdirs::AppDirs,
    error::Result,
    settings::{Settings, Source, SourceFile},
};

// ---

pub const APP_NAME: &str = "hl";

/// Get the default settings.
pub fn default() -> &'static Settings {
    Default::default()
}

/// Load settings from the given file.
pub fn at<I, P>(paths: I) -> Loader
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    Loader::new(paths.into_iter().map(|path| path.as_ref().into()).collect())
}

/// Load settings from the default configuration file per platform.
pub fn load() -> Result<Settings> {
    Loader::new(Vec::new()).load()
}

// ---

pub struct Loader {
    paths: Vec<PathBuf>,
    no_default: bool,
    dirs: Option<AppDirs>,
}

impl Loader {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            no_default: false,
            dirs: app_dirs(),
        }
    }

    pub fn no_default(mut self, val: bool) -> Self {
        self.no_default = val;
        self
    }

    pub fn load(self) -> Result<Settings> {
        if self.no_default {
            Settings::load(self.custom())
        } else {
            Settings::load(self.system().chain(self.user()).chain(self.custom()))
        }
    }

    fn system(&self) -> impl Iterator<Item = Source> + use<> {
        self.dirs
            .as_ref()
            .map(|dirs| dirs.system_config_dirs.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|dir| SourceFile::new(Self::config(&dir)).required(false).into())
    }

    fn user(&self) -> impl Iterator<Item = Source> + use<> {
        self.dirs
            .as_ref()
            .map(|dirs| SourceFile::new(Self::config(&dirs.config_dir)).required(false).into())
            .into_iter()
    }

    fn custom<'a>(&'a self) -> impl Iterator<Item = Source> + 'a {
        self.paths
            .iter()
            .map(|path| SourceFile::new(path).required(true).into())
    }

    fn config(dir: &Path) -> PathBuf {
        dir.join("config")
    }
}

// ---

/// Get the application platform-specific directories.
pub fn app_dirs() -> Option<AppDirs> {
    AppDirs::new(APP_NAME)
}

// ---

pub mod global {
    use super::*;

    static PENDING: Mutex<Option<Settings>> = Mutex::new(None);
    static RESOLVED: Lazy<Settings> = Lazy::new(|| PENDING.lock().unwrap().take().unwrap_or_default());

    /// Call initialize before any calls to get otherwise it will have no effect.
    pub fn initialize(settings: Settings) {
        *PENDING.lock().unwrap() = Some(settings);
    }

    /// Get the resolved settings.
    /// If initialized was called before, then a clone of those settings will be returned.
    /// Otherwise, the default settings will be returned.
    pub fn get() -> &'static Settings {
        &RESOLVED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use maplit::hashmap;

    use crate::level::{InfallibleLevel, Level};

    #[test]
    fn test_default() {
        assert_eq!(default().theme, "uni");
    }

    #[test]
    fn test_load_k8s() {
        let settings = super::at(["etc/defaults/config-k8s.yaml"]).load().unwrap();
        assert_eq!(settings.fields.predefined.time.0.names, &["ts"]);
        assert_eq!(settings.fields.predefined.message.0.names, &["msg"]);
        assert_eq!(settings.fields.predefined.level.variants.len(), 2);
    }

    #[test]
    fn test_issue_288() {
        let settings = super::at(["src/testing/assets/configs/issue-288.yaml"]).load().unwrap();
        assert_eq!(settings.fields.predefined.level.variants.len(), 1);
        let variant = &settings.fields.predefined.level.variants[0];
        assert_eq!(variant.names, vec!["level".to_owned()]);
        assert_eq!(
            variant.values,
            hashmap! {
                InfallibleLevel::new(Level::Debug) => vec!["dbg".to_owned()],
                InfallibleLevel::new(Level::Info) => vec!["INF".to_owned()],
                InfallibleLevel::new(Level::Warning) => vec!["wrn".to_owned()],
                InfallibleLevel::new(Level::Error) => vec!["ERR".to_owned()],
            }
        );
    }

    #[test]
    fn test_load_auto() {
        super::load().unwrap();
    }
}
