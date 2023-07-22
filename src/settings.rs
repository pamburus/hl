// std imports
use std::collections::{BTreeMap, HashMap};
use std::include_str;

// third-party imports
use chrono_tz::Tz;
use config::{Config, File, FileFormat};
use derive_deref::Deref;
use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize, Serializer};

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
        let filename = app_dirs.config_dir.join("config.yaml");

        Ok(Config::builder()
            .add_source(File::from_str(DEFAULT_SETTINGS, FileFormat::Yaml))
            .add_source(File::with_name(&filename.to_string_lossy()).required(false))
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PredefinedFields {
    pub time: TimeField,
    pub level: LevelField,
    pub message: MessageField,
    pub logger: LoggerField,
    pub caller: CallerField,
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct TimeField(pub Field);

// ---

#[derive(Debug, Serialize, Deserialize)]
pub struct LevelField {
    pub variants: Vec<LevelFieldVariant>,
}

// ---

#[derive(Debug, Serialize, Deserialize)]
pub struct LevelFieldVariant {
    pub names: Vec<String>,
    #[serde(serialize_with = "ordered_map_serialize")]
    pub values: HashMap<Level, Vec<String>>,
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct MessageField(Field);

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct LoggerField(Field);

// ---

#[derive(Debug, Serialize, Deserialize, Deref)]
pub struct CallerField(Field);

// ---

#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
    pub names: Vec<String>,
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
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
