// std imports
use std::sync::Mutex;

// third-party imports
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;

// local imports
use crate::{error::Result, settings::Settings};

// ---

pub const APP_NAME: &str = "hl";

static INITIAL: Mutex<Option<Settings>> = Mutex::new(None);
static DEFAULT: Lazy<Settings> = Lazy::new(Settings::default);
static CURRENT: Lazy<Settings> = Lazy::new(|| INITIAL.lock().unwrap().take().unwrap_or(default().clone()));

pub fn set(settings: Settings) {
    *INITIAL.lock().unwrap() = Some(settings);
}

pub fn get() -> &'static Settings {
    &CURRENT
}

pub fn default() -> &'static Settings {
    &DEFAULT
}

pub fn load(path: Option<String>) -> Result<Settings> {
    let path = match path {
        Some(path) => path,
        None => match app_dirs() {
            Some(app_dirs) => app_dirs.config_dir.join("config.yaml").to_string_lossy().to_string(),
            None => {
                return Ok(default().clone());
            }
        },
    };

    Settings::load(&path)
}

pub fn app_dirs() -> Option<AppDirs> {
    AppDirs::new(Some(APP_NAME), true)
}
