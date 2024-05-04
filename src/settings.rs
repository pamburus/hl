// std imports
use std::collections::{BTreeMap, HashMap};
use std::include_str;

// third-party imports
use chrono_tz::Tz;
use config::{Config, File, FileFormat};
use derive_deref::Deref;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize, Serializer};
use strum::IntoEnumIterator;

// local imports
use crate::error::Error;
use crate::level::Level;

// ---

static DEFAULT_SETTINGS_RAW: &str = include_str!("../etc/defaults/config.yaml");
static DEFAULT_SETTINGS: Lazy<Settings> = Lazy::new(|| Settings::load_from_str("", FileFormat::Yaml));

// ---

#[derive(Debug, Deserialize, Clone)]
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
    pub fn load(filename: &str) -> Result<Self, Error> {
        Ok(Config::builder()
            .add_source(File::from_str(DEFAULT_SETTINGS_RAW, FileFormat::Yaml))
            .add_source(File::with_name(filename))
            .build()?
            .try_deserialize()?)
    }

    pub fn load_from_str(value: &str, format: FileFormat) -> Self {
        Config::builder()
            .add_source(File::from_str(DEFAULT_SETTINGS_RAW, FileFormat::Yaml))
            .add_source(File::from_str(value, format))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }
}

impl Default for Settings {
    fn default() -> Self {
        DEFAULT_SETTINGS.clone()
    }
}

impl Default for &'static Settings {
    fn default() -> Self {
        &DEFAULT_SETTINGS
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Fields {
    pub predefined: PredefinedFields,
    pub ignore: Vec<String>,
    pub hide: Vec<String>,
}

// ---

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Deref, Clone, PartialEq, Eq)]
pub struct TimeField(pub Field);

impl Default for TimeField {
    fn default() -> Self {
        Self(Field::new(vec!["time".into(), "ts".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelField {
    pub show: FieldShowOption,
    pub variants: Vec<LevelFieldVariant>,
}

impl Default for LevelField {
    fn default() -> Self {
        Self {
            show: FieldShowOption::default(),
            variants: vec![LevelFieldVariant {
                names: vec!["level".into()],
                values: Level::iter()
                    .map(|level| (level, vec![level.as_ref().to_lowercase().into()]))
                    .collect(),
                level: None,
            }],
        }
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelFieldVariant {
    pub names: Vec<String>,
    #[serde(default, serialize_with = "ordered_map_serialize")]
    pub values: HashMap<Level, Vec<String>>,
    pub level: Option<Level>,
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone)]
pub struct MessageField(Field);

impl Default for MessageField {
    fn default() -> Self {
        Self(Field::new(vec!["msg".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone)]
pub struct LoggerField(Field);

impl Default for LoggerField {
    fn default() -> Self {
        Self(Field::new(vec!["logger".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone)]
pub struct CallerField(Field);

impl Default for CallerField {
    fn default() -> Self {
        Self(Field::new(vec!["caller".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone)]
pub struct CallerFileField(Field);

impl Default for CallerFileField {
    fn default() -> Self {
        Self(Field::new(vec!["file".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone)]
pub struct CallerLineField(Field);

impl Default for CallerLineField {
    fn default() -> Self {
        Self(Field::new(vec!["line".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Field {
    pub names: Vec<String>,
    #[serde(default)]
    pub show: FieldShowOption,
}

impl Field {
    pub fn new(names: Vec<String>) -> Self {
        Self {
            names,
            show: FieldShowOption::Auto,
        }
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Formatting {
    pub punctuation: Punctuation,
    pub flatten: Option<FlattenOption>,
}

// ---

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlattenOption {
    Never,
    Always,
}

// ---

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum FieldShowOption {
    Auto,
    Always,
}

impl Default for FieldShowOption {
    fn default() -> Self {
        Self::Auto
    }
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
            field_key_value_separator: "=".into(),
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
            field_key_value_separator: "=".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let test = |settings: &Settings| {
            assert_eq!(settings.concurrency, None);
            assert_eq!(settings.time_format, "%b %d %T.%3N");
            assert_eq!(settings.time_zone, chrono_tz::UTC);
            assert_eq!(settings.theme, "universal");
        };

        let settings: &'static Settings = Default::default();
        test(settings);
        test(&Settings::default());
    }

    #[test]
    fn test_load_settings_k8s() {
        let settings = Settings::load("etc/defaults/config-k8s.yaml").unwrap();
        assert_eq!(
            settings.fields.predefined.time,
            TimeField(Field {
                names: vec!["ts".into()],
                show: FieldShowOption::Auto,
            })
        );
        assert_eq!(settings.time_format, "%b %d %T.%3N");
        assert_eq!(settings.time_zone, chrono_tz::UTC);
        assert_eq!(settings.theme, "universal");
    }
}
