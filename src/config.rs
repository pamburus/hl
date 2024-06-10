// std imports
use std::sync::Mutex;

// third-party imports
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;

// local imports
use crate::{
    error::Result,
    settings::{Settings, SourceFile},
};

// ---

pub const APP_NAME: &str = "hl";

/// Get the default settings.
pub fn default() -> &'static Settings {
    Default::default()
}

/// Load settings from the given file or the default configuration file per platform.
pub fn load(path: Option<&str>) -> Result<Settings> {
    let mut default = None;
    let (filename, required) = path.map(|p| (p, true)).unwrap_or_else(|| {
        (
            if let Some(dirs) = app_dirs() {
                default = Some(dirs.config_dir.join("config").to_string_lossy().to_string());
                default.as_deref().unwrap()
            } else {
                ""
            },
            false,
        )
    });

    if filename.is_empty() {
        return Ok(Default::default());
    }

    Settings::load(SourceFile::new(filename).required(required).into())
}

/// Get the application platform-specific directories.
pub fn app_dirs() -> Option<AppDirs> {
    AppDirs::new(Some(APP_NAME), true)
}

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

    use crate::{level::Level, settings::Settings};

    #[test]
    fn test_default() {
        assert_eq!(default().theme, "universal");
    }

    #[test]
    fn test_load_empty_filename() {
        let settings = super::load(Some("")).unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn test_load_k8s() {
        let settings = super::load(Some("etc/defaults/config-k8s.yaml")).unwrap();
        assert_eq!(settings.fields.predefined.time.0.names, &["ts"]);
        assert_eq!(settings.fields.predefined.message.0.names, &["msg"]);
        assert_eq!(settings.fields.predefined.level.variants.len(), 2);
    }

    #[test]
    fn test_issue_288() {
        let settings = super::load(Some("src/testing/assets/configs/issue-288.yaml")).unwrap();
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
        super::load(None).unwrap();
    }
}
