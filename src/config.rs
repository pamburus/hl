// std imports
use std::sync::Mutex;

// third-party imports
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;

// local imports
use crate::{error::Result, settings::Settings};

// ---

pub const APP_NAME: &str = "hl";

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

/// Get the default settings.
pub fn default() -> &'static Settings {
    Default::default()
}

/// Load settings from the given file or the default configuration file per platform.
pub fn load(path: String) -> Result<Settings> {
    if path.is_empty() {
        return Ok(Default::default());
    }

    Settings::load(&path)
}

/// Get the application platform-specific directories.
pub fn app_dirs() -> Option<AppDirs> {
    AppDirs::new(Some(APP_NAME), true)
}
