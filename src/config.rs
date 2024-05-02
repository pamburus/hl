// third-party imports
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;

// local imports
use crate::{error::Result, settings::Settings};

// ---

pub const APP_NAME: &str = "hl";

static CONFIG: Lazy<Settings> = Lazy::new(safe_load);
static DEFAULT: Lazy<Settings> = Lazy::new(Settings::default);

pub fn get() -> &'static Settings {
    &CONFIG
}

pub fn default() -> &'static Settings {
    &DEFAULT
}

pub fn load() -> Result<Settings> {
    Settings::load(&app_dirs())
}

pub fn app_dirs() -> AppDirs {
    AppDirs::new(Some(APP_NAME), true).unwrap()
}

// ---

fn safe_load() -> Settings {
    match load() {
        Ok(settings) => settings,
        Err(err) => {
            crate::error::log(&err);
            default().clone()
        }
    }
}
