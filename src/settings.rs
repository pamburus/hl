// std imports
use std::collections::{BTreeMap, HashMap};
use std::include_str;

// third-party imports
use chrono_tz::Tz;
use config::{Config, File, FileFormat};
use derive_deref::Deref;
use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize, Serializer};
use strum::IntoEnumIterator;

// local imports
use crate::error::Error;
use crate::level::Level;

// ---

static DEFAULT_SETTINGS: &str = include_str!("../etc/defaults/config.yaml");

// ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    pub fields: Fields,
    pub concurrency: Option<usize>,
    pub time_format: String,
    pub time_zone: Tz,
    pub formatting: Formatting,
    pub theme: String,
}

impl Settings {
    pub fn load(app_dirs: &AppDirs) -> Result<Self, Error> {
        let filename = std::env::var("HL_CONFIG")
            .unwrap_or_else(|_| app_dirs.config_dir.join("config.yaml").to_string_lossy().to_string());

        Ok(Config::builder()
            .add_source(File::from_str(DEFAULT_SETTINGS, FileFormat::Yaml))
            .add_source(File::with_name(&filename).required(false))
            .build()?
            .try_deserialize()?)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Config::builder()
            .add_source(File::from_str(DEFAULT_SETTINGS, FileFormat::Yaml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }
}

// ---

#[derive(Debug, Serialize, Deserialize)]
pub struct Fields {
    pub predefined: PredefinedFields,
    pub ignore: Vec<String>,
    pub hide: Vec<String>,
}

// ---

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct PredefinedFields {
    pub time: TimeField,
    pub level: LevelField,
    pub message: MessageField,
    pub logger: LoggerField,
    pub caller: CallerField,
    pub caller_file: CallerFileField,
    pub caller_line: CallerLineField,
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct TimeField(pub Field);

impl Default for TimeField {
    fn default() -> Self {
        Self(Field {
            names: vec!["time".into()],
        })
    }
}

// ---

#[derive(Debug, Serialize, Deserialize)]
pub struct LevelField {
    pub variants: Vec<LevelFieldVariant>,
}

impl Default for LevelField {
    fn default() -> Self {
        Self {
            variants: vec![LevelFieldVariant {
                names: vec!["level".into()],
                values: Level::iter()
                    .map(|level| (level, vec![level.as_ref().into()]))
                    .collect(),
                level: None,
            }],
        }
    }
}

// ---

#[derive(Debug, Serialize, Deserialize)]
pub struct LevelFieldVariant {
    pub names: Vec<String>,
    #[serde(default, serialize_with = "ordered_map_serialize")]
    pub values: HashMap<Level, Vec<String>>,
    pub level: Option<Level>,
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct MessageField(Field);

impl Default for MessageField {
    fn default() -> Self {
        Self(Field {
            names: vec!["msg".into()],
        })
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct LoggerField(Field);

impl Default for LoggerField {
    fn default() -> Self {
        Self(Field {
            names: vec!["logger".into()],
        })
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct CallerField(Field);

impl Default for CallerField {
    fn default() -> Self {
        Self(Field {
            names: vec!["caller".into()],
        })
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct CallerFileField(Field);

impl Default for CallerFileField {
    fn default() -> Self {
        Self(Field {
            names: vec!["file".into()],
        })
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct CallerLineField(Field);

impl Default for CallerLineField {
    fn default() -> Self {
        Self(Field {
            names: vec!["line".into()],
        })
    }
}

// ---

#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
    pub names: Vec<String>,
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Formatting {
    pub punctuation: Punctuation,
}

// ---

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Punctuation {
    pub logger_name_separator: String,
    pub field_key_value_separator: String,
    pub string_opening_quote: String,
    pub string_closing_quote: String,
    pub source_location_separator: String,
    pub hidden_fields_indicator: String,
    pub level_left_separator: String,
    pub level_right_separator: String,
    pub input_number_prefix: String,
    pub input_number_left_separator: String,
    pub input_number_right_separator: String,
    pub input_name_left_separator: String,
    pub input_name_right_separator: String,
    pub input_name_clipping: String,
    pub input_name_common_part: String,
    pub array_separator: String,
}

impl Default for Punctuation {
    fn default() -> Self {
        Self {
            logger_name_separator: ":".into(),
            field_key_value_separator: ":".into(),
            string_opening_quote: "'".into(),
            string_closing_quote: "'".into(),
            source_location_separator: "@ ".into(),
            hidden_fields_indicator: " ...".into(),
            level_left_separator: "|".into(),
            level_right_separator: "|".into(),
            input_number_prefix: "#".into(),
            input_number_left_separator: "".into(),
            input_number_right_separator: " | ".into(),
            input_name_left_separator: "".into(),
            input_name_right_separator: " | ".into(),
            input_name_clipping: "...".into(),
            input_name_common_part: "...".into(),
            array_separator: " ".into(),
        }
    }
}

impl Punctuation {
    #[cfg(test)]
    pub fn test_default() -> Self {
        Self {
            logger_name_separator: ":".into(),
            field_key_value_separator: ":".into(),
            string_opening_quote: "'".into(),
            string_closing_quote: "'".into(),
            source_location_separator: "@ ".into(),
            hidden_fields_indicator: " ...".into(),
            level_left_separator: "|".into(),
            level_right_separator: "|".into(),
            input_number_prefix: "#".into(),
            input_number_left_separator: "".into(),
            input_number_right_separator: " | ".into(),
            input_name_left_separator: "".into(),
            input_name_right_separator: " | ".into(),
            input_name_clipping: "...".into(),
            input_name_common_part: "...".into(),
            array_separator: ",".into(),
        }
    }
}

fn ordered_map_serialize<K: Eq + PartialEq + Ord + PartialOrd + Serialize, V: Serialize, S>(
    value: &HashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}
