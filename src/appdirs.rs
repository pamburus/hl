// std imports
use std::path::PathBuf;

pub struct AppDirs {
    pub cache_dir: PathBuf,
    pub config_dir: PathBuf,
    pub system_config_dirs: Vec<PathBuf>,
}

impl AppDirs {
    pub fn new(name: &str) -> Option<Self> {
        let cache_dir = sys::cache_dir()?.join(name);
        let config_dir = sys::config_dir()?.join(name);
        let system_config_dirs = sys::system_config_dirs().into_iter().map(|d| d.join(name)).collect();
        Some(Self {
            cache_dir,
            config_dir,
            system_config_dirs,
        })
    }
}

#[cfg(target_os = "macos")]
mod sys {
    use super::*;
    use std::env;

    pub(crate) fn cache_dir() -> Option<PathBuf> {
        env::var_os("XDG_CACHE_HOME")
            .and_then(dirs_sys::is_absolute_path)
            .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
    }

    pub(crate) fn config_dir() -> Option<PathBuf> {
        env::var_os("XDG_CONFIG_HOME")
            .and_then(dirs_sys::is_absolute_path)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
    }

    pub(crate) fn system_config_dirs() -> Vec<PathBuf> {
        vec![PathBuf::from("/etc")]
    }
}

#[cfg(not(target_os = "macos"))]
mod sys {
    use super::*;

    pub(crate) fn config_dir() -> Option<PathBuf> {
        dirs::config_dir()
    }

    pub(crate) fn cache_dir() -> Option<PathBuf> {
        dirs::cache_dir()
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn system_config_dirs() -> Vec<PathBuf> {
        vec![PathBuf::from("/etc")]
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn system_config_dirs() -> Vec<PathBuf> {
        use known_folders::{KnownFolder, get_known_folder_path};

        get_known_folder_path(KnownFolder::ProgramData).into_iter().collect()
    }
}
