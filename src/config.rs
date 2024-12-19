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
pub fn at<P>(path: P) -> Loader
where
    P: AsRef<Path>,
{
    Loader::new(Some(path.as_ref().into()))
}

/// Load settings from the given file or the default configuration file per platform.
pub fn optional_at<P>(path: Option<P>) -> Loader
where
    P: AsRef<Path>,
{
    Loader::new(path.map(|path| path.as_ref().into()))
}

/// Load settings from the default configuration file per platform.
pub fn load() -> Result<Settings> {
    Loader::new(None).load()
}

// ---

pub struct Loader {
    path: Option<PathBuf>,
    no_extra: bool,
    dirs: Option<AppDirs>,
}

impl Loader {
    fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            no_extra: false,
            dirs: app_dirs(),
        }
    }

    pub fn no_extra(mut self, val: bool) -> Self {
        self.no_extra = val;
        self
    }

    pub fn load(self) -> Result<Settings> {
        if self.no_extra {
            Settings::load(self.custom())
        } else {
            Settings::load(self.system().chain(self.user()).chain(self.custom()))
        }
    }

    fn system(&self) -> impl Iterator<Item = Source> {
        self.dirs
            .as_ref()
            .map(|dirs| dirs.system_config_dirs.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|dir| SourceFile::new(&Self::config(&dir)).required(false).into())
    }

    fn user(&self) -> impl Iterator<Item = Source> {
        self.dirs
            .as_ref()
            .map(|dirs| SourceFile::new(&Self::config(&dirs.config_dir)).required(false).into())
            .into_iter()
    }

    fn custom(&self) -> impl Iterator<Item = Source> {
        self.path
            .as_ref()
            .map(|path| SourceFile::new(path).required(true).into())
            .into_iter()
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

    use crate::level::Level;

    #[test]
    fn test_default() {
        assert_eq!(default().theme, "universal");
    }

    #[test]
    fn test_load_k8s() {
        let settings = super::at("etc/defaults/config-k8s.yaml").load().unwrap();
        assert_eq!(settings.fields.predefined.time.0.names, &["ts"]);
        assert_eq!(settings.fields.predefined.message.0.names, &["msg"]);
        assert_eq!(settings.fields.predefined.level.variants.len(), 2);
    }

    #[test]
    fn test_issue_288() {
        let settings = super::at("src/testing/assets/configs/issue-288.yaml").load().unwrap();
        assert_eq!(settings.fields.predefined.level.variants.len(), 1);
        let variant = &settings.fields.predefined.level.variants[0];
        assert_eq!(variant.names, vec!["level".to_owned()]);
        assert_eq!(
            variant.values,
            hashmap! {
                Level::Debug => vec!["dbg".to_owned()],
                // TODO: replace `"inf"` with `"INF"` when https://github.com/mehcode/config-rs/issues/568 is fixed
                Level::Info => vec!["inf".to_owned()],
                Level::Warning => vec!["wrn".to_owned()],
                Level::Error => vec!["ERR".to_owned()],
            }
        );
    }

    #[test]
    fn test_load_auto() {
        super::load().unwrap();
    }
}
