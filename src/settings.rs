// std imports
use std::{
    collections::{BTreeMap, HashMap},
    include_str,
    path::{Path, PathBuf},
};

// third-party imports
use chrono_tz::Tz;
use config::{Config, File, FileFormat};
use derive_more::Deref;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize, Serializer};
use strum::{Display, IntoEnumIterator};

// local imports
use crate::{error::Error, level::Level};

// ---

static DEFAULT_SETTINGS_RAW: &str = include_str!("../etc/defaults/config.yaml");
static DEFAULT_SETTINGS: Lazy<Settings> = Lazy::new(|| Settings::load([Source::string("", FileFormat::Yaml)]).unwrap());

// ---

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
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
    pub fn load<I>(sources: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = Source>,
    {
        let mut builder = Config::builder().add_source(File::from_str(DEFAULT_SETTINGS_RAW, FileFormat::Yaml));

        for source in sources {
            builder = match source {
                Source::File(SourceFile { filename, required }) => {
                    log::debug!(
                        "added configuration file {} search path: {}",
                        if required { "required" } else { "optional" },
                        filename.display(),
                    );
                    builder.add_source(File::from(filename.as_path()).required(required))
                }
                Source::String(value, format) => builder.add_source(File::from_str(&value, format)),
            };
        }

        Ok(builder.build()?.try_deserialize()?)
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

pub enum Source {
    File(SourceFile),
    String(String, FileFormat),
}

impl Source {
    pub fn string<S>(value: S, format: FileFormat) -> Self
    where
        S: Into<String>,
    {
        Self::String(value.into(), format)
    }
}

impl From<SourceFile> for Source {
    fn from(file: SourceFile) -> Self {
        Self::File(file)
    }
}

// ---

pub struct SourceFile {
    filename: PathBuf,
    required: bool,
}

impl SourceFile {
    pub fn new<P>(filename: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            filename: filename.as_ref().into(),
            required: true,
        }
    }

    pub fn required(self, required: bool) -> Self {
        Self { required, ..self }
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fields {
    pub predefined: PredefinedFields,
    pub ignore: Vec<String>,
    pub hide: Vec<String>,
}

// ---

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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

impl Default for &PredefinedFields {
    fn default() -> Self {
        static DEFAULT: Lazy<PredefinedFields> = Lazy::new(|| PredefinedFields::default());
        &DEFAULT
    }
}

// ---

#[derive(Clone, Debug, Deref, Deserialize, Eq, PartialEq, Serialize)]
pub struct TimeField(pub Field);

impl Default for TimeField {
    fn default() -> Self {
        Self(Field::new(vec!["time".into(), "ts".into()]))
    }
}

// ---

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LevelFieldVariant {
    pub names: Vec<String>,
    #[serde(default, serialize_with = "ordered_map_serialize")]
    pub values: HashMap<Level, Vec<String>>,
    pub level: Option<Level>,
}

// ---

#[derive(Clone, Debug, Deref, Deserialize, Eq, PartialEq, Serialize)]
pub struct MessageField(pub Field);

impl Default for MessageField {
    fn default() -> Self {
        Self(Field::new(vec!["msg".into()]))
    }
}

// ---

#[derive(Clone, Debug, Deref, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoggerField(Field);

impl Default for LoggerField {
    fn default() -> Self {
        Self(Field::new(vec!["logger".into()]))
    }
}

// ---

#[derive(Clone, Debug, Deref, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallerField(Field);

impl Default for CallerField {
    fn default() -> Self {
        Self(Field::new(vec!["caller".into()]))
    }
}

// ---

#[derive(Clone, Debug, Deref, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallerFileField(Field);

impl Default for CallerFileField {
    fn default() -> Self {
        Self(Field::new(vec!["file".into()]))
    }
}

// ---

#[derive(Clone, Debug, Deref, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Formatting {
    pub punctuation: Punctuation,
    pub flatten: Option<FlattenOption>,
    pub expansion: ExpansionOptions,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ExpansionOptions {
    pub mode: Option<ExpansionMode>,
    pub profiles: ExpansionProfiles,
}

impl ExpansionOptions {
    pub fn profile(&self) -> Option<&ExpansionProfile> {
        self.mode.map(|mode| self.profiles.resolve(mode))
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ExpansionProfiles {
    pub low: ExpansionProfile,
    pub medium: ExpansionProfile,
    pub high: ExpansionProfile,
}

impl ExpansionProfiles {
    pub fn resolve(&self, mode: ExpansionMode) -> &ExpansionProfile {
        match mode {
            ExpansionMode::Never => &ExpansionProfile::NEVER,
            ExpansionMode::Inline => &ExpansionProfile::INLINE,
            ExpansionMode::Low => &self.low,
            ExpansionMode::Medium => &self.medium,
            ExpansionMode::High => &self.high,
            ExpansionMode::Always => &ExpansionProfile::ALWAYS,
        }
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ExpansionProfile {
    pub multiline: Option<MultilineExpansion>,
    pub thresholds: ExpansionThresholds,
}

impl ExpansionProfile {
    const NEVER: Self = Self {
        multiline: Some(MultilineExpansion::Disabled),
        thresholds: ExpansionThresholds {
            global: Some(usize::MAX),
            cumulative: Some(usize::MAX),
            message: Some(usize::MAX),
            field: Some(usize::MAX),
        },
    };

    const INLINE: Self = Self {
        multiline: Some(MultilineExpansion::Inline),
        thresholds: ExpansionThresholds {
            global: Some(usize::MAX),
            cumulative: Some(usize::MAX),
            message: Some(usize::MAX),
            field: Some(usize::MAX),
        },
    };

    const ALWAYS: Self = Self {
        multiline: Some(MultilineExpansion::Standard),
        thresholds: ExpansionThresholds {
            global: Some(0),
            cumulative: Some(0),
            message: Some(0),
            field: Some(0),
        },
    };
}

// ---

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ExpansionThresholds {
    pub global: Option<usize>,
    pub cumulative: Option<usize>,
    pub message: Option<usize>,
    pub field: Option<usize>,
}

// ---

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MultilineExpansion {
    #[default]
    Standard,
    Disabled,
    Inline,
}

// ---

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum FlattenOption {
    Never,
    Always,
}

// ---

#[derive(Clone, Copy, Debug, Default, Deserialize, Display, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ExpansionMode {
    Never,
    Inline,
    Low,
    #[default]
    Medium,
    High,
    Always,
}

// ---

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
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
            hidden_fields_indicator: "...".into(),
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
        let settings = Settings::load([SourceFile::new("etc/defaults/config-k8s.yaml").into()]).unwrap();
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

    #[test]
    fn test_expansion_options() {
        let mut profiles = ExpansionProfiles::default();
        profiles.low.thresholds.global = Some(1);
        profiles.low.thresholds.cumulative = Some(2);
        profiles.low.thresholds.message = Some(3);
        profiles.medium.thresholds.global = Some(4);
        profiles.medium.thresholds.field = Some(5);
        profiles.high.thresholds.global = Some(6);
        profiles.high.thresholds.cumulative = Some(7);
        let xo = |mode| ExpansionOptions {
            mode,
            profiles: profiles.clone(),
        };
        assert_eq!(xo(None).profile(), None);
        assert_eq!(xo(Some(ExpansionMode::Never)).profile(), Some(&ExpansionProfile::NEVER));
        assert_eq!(
            xo(Some(ExpansionMode::Inline)).profile(),
            Some(&ExpansionProfile::INLINE)
        );
        assert_eq!(xo(Some(ExpansionMode::Low)).profile(), Some(&profiles.low));
        assert_eq!(xo(Some(ExpansionMode::Medium)).profile(), Some(&profiles.medium));
        assert_eq!(xo(Some(ExpansionMode::High)).profile(), Some(&profiles.high));
        assert_eq!(
            xo(Some(ExpansionMode::Always)).profile(),
            Some(&ExpansionProfile::ALWAYS)
        );
    }
}
