// third-party imports
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;

// local imports
use crate::settings::Settings;

// ---

pub const APP_NAME: &str = "hl";

static CONFIG: Lazy<Settings> = Lazy::new(load);
static DEFAULT: Lazy<Settings> = Lazy::new(Settings::default);

pub fn get() -> &'static Settings {
    &CONFIG
}

pub fn default() -> &'static Settings {
    &DEFAULT
}

pub fn app_dirs() -> AppDirs {
    AppDirs::new(Some(APP_NAME), true).unwrap()
}

// ---

fn load() -> Settings {
    Settings::load(&app_dirs()).unwrap()
}
