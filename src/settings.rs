// std imports
use std::collections::HashMap;

// third-party imports
use config::{Config, Environment, File};
use derive_deref::Deref;
use platform_dirs::AppDirs;
use serde::Deserialize;

// local imports
use crate::error::Error;
use crate::types::Level;

// ---

macro_rules! collection {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {
        std::iter::Iterator::collect(std::array::IntoIter::new([$(($k, $v),)*]))
    };
    // set-like
    ($($v:expr),* $(,)?) => {
        std::iter::Iterator::collect(std::array::IntoIter::new([$($v,)*]))
    };
}

macro_rules! into {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {
        std::iter::Iterator::collect(std::array::IntoIter::new([$(($k.into(), $v.into()),)*]))
    };
    // set-like
    ($($v:expr),* $(,)?) => {
        std::iter::Iterator::collect(std::array::IntoIter::new([$($v.into(),)*]))
    };
}

macro_rules! str_vec {
    ($($x:expr),* $(,)?) => (vec![$($x.to_owned()),*]);

}

// ---

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub fields: Fields,
}

impl Settings {
    pub fn load(app_dirs: &AppDirs) -> Result<Self, Error> {
        let mut s = Config::default();
        let filename = app_dirs.config_dir.join("config.yaml");

        s.merge(File::with_name(&filename.to_string_lossy()).required(false))?;
        s.merge(Environment::with_prefix("HL"))?;

        Ok(s.try_into()?)
    }
}

// ---

#[derive(Debug, Deserialize, Default)]
pub struct Fields {
    #[serde(default)]
    pub time: TimeField,
    #[serde(default)]
    pub level: LevelField,
    #[serde(default)]
    pub message: MessageField,
    #[serde(default)]
    pub logger: LoggerField,
    #[serde(default)]
    pub caller: CallerField,
}

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct TimeField(pub Field);

impl Default for TimeField {
    fn default() -> Self {
        Self(Field {
            names: into![
                "ts",
                "TS",
                "time",
                "TIME",
                "Time",
                "_SOURCE_REALTIME_TIMESTAMP",
                "__REALTIME_TIMESTAMP",
            ],
        })
    }
}

// ---

#[derive(Debug, Deserialize)]
pub struct LevelField {
    pub variants: Vec<LevelFieldVariant>,
}

impl Default for LevelField {
    fn default() -> Self {
        Self {
            variants: vec![
                LevelFieldVariant {
                    names: into!["level", "LEVEL", "Level"],
                    values: collection! {
                        Level::Debug => str_vec!["debug"],
                        Level::Info => str_vec!["info", "information"],
                        Level::Warning => str_vec!["warning", "warn"],
                        Level::Error => str_vec!["error", "err", "fatal", "critical", "panic"],
                    },
                },
                LevelFieldVariant {
                    names: into!["PRIORITY"],
                    values: collection! {
                        Level::Debug => str_vec!["7"],
                        Level::Info => str_vec!["6"],
                        Level::Warning => str_vec!["5", "4"],
                        Level::Error => str_vec!["3", "2", "1"],
                    },
                },
            ],
        }
    }
}

// ---

#[derive(Debug, Deserialize)]
pub struct LevelFieldVariant {
    pub names: Vec<String>,
    pub values: HashMap<Level, Vec<String>>,
}

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct MessageField(Field);

impl Default for MessageField {
    fn default() -> Self {
        Self(Field {
            names: into!["msg", "message", "MESSAGE", "Message"],
        })
    }
}

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct LoggerField(Field);

impl Default for LoggerField {
    fn default() -> Self {
        Self(Field {
            names: into!["logger", "LOGGER", "Logger"],
        })
    }
}

// ---

#[derive(Debug, Deserialize, Deref)]
pub struct CallerField(Field);

impl Default for CallerField {
    fn default() -> Self {
        Self(Field {
            names: into!["caller", "CALLER", "Caller"],
        })
    }
}

// ---

#[derive(Debug, Deserialize)]
pub struct Field {
    pub names: Vec<String>,
}
