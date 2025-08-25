// std imports
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    include_str,
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
};

// third-party imports
use chrono_tz::Tz;
use config::{Config, File, FileFormat};
use derive_more::{Deref, From};
use enumset::{EnumSet, EnumSetType, enum_set};
use enumset_ext::EnumSetExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::IntoDeserializer};
use strum::{Display, IntoEnumIterator};

// local imports
use crate::level::{InfallibleLevel, Level};
use crate::{error::Error, xerr::Suggestions};

// sub-modules
pub mod error;

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
    #[serde(deserialize_with = "enumset_serde::deserialize")]
    pub input_info: InputInfoSet,
    pub ascii: AsciiModeOpt,
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

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Fields {
    pub predefined: PredefinedFields,
    pub ignore: Vec<String>,
    pub hide: Vec<String>,
}

// ---

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
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

#[derive(Debug, Serialize, Deserialize, Deref, Clone, PartialEq, Eq, From)]
pub struct TimeField(pub Field);

impl Default for TimeField {
    fn default() -> Self {
        Self(Field::new(vec!["time".into(), "ts".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LevelField {
    pub show: FieldShowOption,
    pub variants: Vec<RawLevelFieldVariant>,
}

impl Default for LevelField {
    fn default() -> Self {
        Self {
            show: FieldShowOption::default(),
            variants: vec![RawLevelFieldVariant {
                names: vec!["level".into()],
                values: Level::iter()
                    .map(|level| (level.into(), vec![level.as_ref().to_lowercase().into()]))
                    .collect(),
                level: None,
            }],
        }
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct RawLevelFieldVariant {
    pub names: Vec<String>,
    #[serde(default, serialize_with = "ordered_map_serialize")]
    pub values: HashMap<InfallibleLevel, Vec<String>>,
    pub level: Option<InfallibleLevel>,
}

impl RawLevelFieldVariant {
    pub fn resolve(&self) -> Option<LevelFieldVariant> {
        let mut unknowns = HashSet::new();
        let mut values = HashMap::new();
        let mut valid = true;

        for (level, names) in &self.values {
            match level {
                InfallibleLevel::Valid(level) => {
                    values.insert(level.clone(), names.clone());
                }
                InfallibleLevel::Invalid(name) => {
                    unknowns.insert(name.clone());
                }
            }
        }

        let level = self.level.clone().and_then(|level| match level {
            InfallibleLevel::Valid(level) => Some(level),
            InfallibleLevel::Invalid(name) => {
                unknowns.insert(name);
                valid = false;
                None
            }
        });

        for name in unknowns {
            log::warn!("unknown level: {:?}", name);
        }

        if valid && !values.is_empty() {
            Some(LevelFieldVariant {
                names: self.names.clone(),
                values,
                level,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LevelFieldVariant {
    pub names: Vec<String>,
    #[serde(default, serialize_with = "ordered_map_serialize")]
    pub values: HashMap<Level, Vec<String>>,
    pub level: Option<Level>,
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone, PartialEq, Eq, From)]
pub struct MessageField(pub Field);

impl Default for MessageField {
    fn default() -> Self {
        Self(Field::new(vec!["msg".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone, PartialEq, Eq, From)]
pub struct LoggerField(Field);

impl Default for LoggerField {
    fn default() -> Self {
        Self(Field::new(vec!["logger".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone, PartialEq, Eq, From)]
pub struct CallerField(Field);

impl Default for CallerField {
    fn default() -> Self {
        Self(Field::new(vec!["caller".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone, PartialEq, Eq, From)]
pub struct CallerFileField(Field);

impl Default for CallerFileField {
    fn default() -> Self {
        Self(Field::new(vec!["file".into()]))
    }
}

// ---

#[derive(Debug, Serialize, Deserialize, Deref, Clone, PartialEq, Eq, From)]
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

pub type InputInfoSet = EnumSet<InputInfo>;

#[derive(Debug, Serialize, Deserialize, EnumSetType, Display)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum InputInfo {
    Auto,
    None,
    Minimal,
    Compact,
    Full,
}

impl InputInfo {
    pub fn resolve(set: enumset::EnumSet<InputInfo>) -> enumset::EnumSet<InputInfo> {
        if !set.intersects(enum_set!(InputInfo::Auto).complement()) {
            enumset::EnumSet::all()
        } else {
            set
        }
        .difference(InputInfo::Auto.into())
    }
}

impl FromStr for InputInfo {
    type Err = self::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str(s).map_err(|_| Self::Err::InvalidInputInfo {
            value: s.into(),
            suggestions: Suggestions::new(s, InputInfoSet::all().iter().map(|v| v.to_string())),
        })
    }
}

// ---

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Formatting {
    pub flatten: Option<FlattenOption>,
    pub message: MessageFormatting,
    pub punctuation: Punctuation,
}

// ---

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct MessageFormatting {
    pub format: MessageFormat,
}

// ---

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum MessageFormat {
    AutoQuoted,
    AlwaysQuoted,
    AlwaysDoubleQuoted,
    #[default]
    Delimited,
    Raw,
}

// ---

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FlattenOption {
    Never,
    Always,
}

// ---

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum FieldShowOption {
    #[default]
    Auto,
    Always,
}

// ---

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Punctuation<T: Clone + PartialEq + Eq = DisplayVariant> {
    pub logger_name_separator: T,
    pub field_key_value_separator: T,
    pub string_opening_quote: T,
    pub string_closing_quote: T,
    pub source_location_separator: T,
    pub caller_name_file_separator: T,
    pub hidden_fields_indicator: T,
    pub level_left_separator: T,
    pub level_right_separator: T,
    pub input_number_prefix: T,
    pub input_number_left_separator: T,
    pub input_number_right_separator: T,
    pub input_name_left_separator: T,
    pub input_name_right_separator: T,
    pub input_name_clipping: T,
    pub input_name_common_part: T,
    pub array_separator: T,
    pub message_delimiter: T,
}

impl Punctuation {
    pub fn resolve(&self, mode: AsciiMode) -> Punctuation<String> {
        Punctuation {
            logger_name_separator: self.logger_name_separator.resolve(mode).to_owned(),
            field_key_value_separator: self.field_key_value_separator.resolve(mode).to_owned(),
            string_opening_quote: self.string_opening_quote.resolve(mode).to_owned(),
            string_closing_quote: self.string_closing_quote.resolve(mode).to_owned(),
            source_location_separator: self.source_location_separator.resolve(mode).to_owned(),
            caller_name_file_separator: self.caller_name_file_separator.resolve(mode).to_owned(),
            hidden_fields_indicator: self.hidden_fields_indicator.resolve(mode).to_owned(),
            level_left_separator: self.level_left_separator.resolve(mode).to_owned(),
            level_right_separator: self.level_right_separator.resolve(mode).to_owned(),
            input_number_prefix: self.input_number_prefix.resolve(mode).to_owned(),
            input_number_left_separator: self.input_number_left_separator.resolve(mode).to_owned(),
            input_number_right_separator: self.input_number_right_separator.resolve(mode).to_owned(),
            input_name_left_separator: self.input_name_left_separator.resolve(mode).to_owned(),
            input_name_right_separator: self.input_name_right_separator.resolve(mode).to_owned(),
            input_name_clipping: self.input_name_clipping.resolve(mode).to_owned(),
            input_name_common_part: self.input_name_common_part.resolve(mode).to_owned(),
            array_separator: self.array_separator.resolve(mode).to_owned(),
            message_delimiter: self.message_delimiter.resolve(mode).to_owned(),
        }
    }
}

impl Default for Punctuation {
    fn default() -> Self {
        Self {
            logger_name_separator: ":".into(),
            field_key_value_separator: "=".into(),
            string_opening_quote: "'".into(),
            string_closing_quote: "'".into(),
            source_location_separator: "@ ".into(),
            caller_name_file_separator: " ".into(),
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
            message_delimiter: "::".into(),
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
            caller_name_file_separator: " :: ".into(),
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
            message_delimiter: "::".into(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum AsciiModeOpt {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum AsciiMode {
    On,
    Off,
}

impl AsciiModeOpt {
    pub fn resolve(&self, utf8_supported: bool) -> AsciiMode {
        match self {
            Self::Auto => {
                if utf8_supported {
                    AsciiMode::Off
                } else {
                    AsciiMode::On
                }
            }
            Self::Always => AsciiMode::On,
            Self::Never => AsciiMode::Off,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum DisplayVariant {
    Uniform(String),
    Selective {
        ascii: String,
        #[serde(rename = "utf-8")]
        utf8: String,
    },
}

impl DisplayVariant {
    pub fn resolve(&self, mode: AsciiMode) -> &str {
        match self {
            Self::Uniform(s) => &s,
            Self::Selective { ascii, utf8: unicode } => match mode {
                AsciiMode::Off => &unicode,
                AsciiMode::On => &ascii,
            },
        }
    }
}

impl From<String> for DisplayVariant {
    fn from(value: String) -> Self {
        Self::Uniform(value)
    }
}

impl From<&str> for DisplayVariant {
    fn from(value: &str) -> Self {
        Self::Uniform(value.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum ListOrCommaSeparatedList<T> {
    List(Vec<T>),
    #[serde(deserialize_with = "csl_deserialize")]
    CommaSeparatedList(Vec<T>),
}

impl<T> Deref for ListOrCommaSeparatedList<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            ListOrCommaSeparatedList::List(list) => list,
            ListOrCommaSeparatedList::CommaSeparatedList(list) => list,
        }
    }
}

impl<T> From<ListOrCommaSeparatedList<T>> for Vec<T> {
    fn from(value: ListOrCommaSeparatedList<T>) -> Self {
        match value {
            ListOrCommaSeparatedList::List(list) => list,
            ListOrCommaSeparatedList::CommaSeparatedList(list) => list,
        }
    }
}

impl<T> From<Vec<T>> for ListOrCommaSeparatedList<T> {
    fn from(value: Vec<T>) -> Self {
        ListOrCommaSeparatedList::List(value)
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

fn csl_deserialize<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let seq = String::deserialize(deserializer)?;
    if seq.is_empty() {
        Ok(Vec::new())
    } else {
        seq.split(',')
            .map(|item| T::deserialize(item.trim().into_deserializer()))
            .collect()
    }
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
            assert_eq!(settings.theme, "uni");
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
        assert_eq!(settings.theme, "uni");
    }

    #[test]
    fn test_unknown_level_values() {
        let variant = RawLevelFieldVariant {
            names: vec!["level".into()],
            values: vec![
                (InfallibleLevel::Valid(Level::Info), vec!["info".into()]),
                (InfallibleLevel::Invalid("unknown".into()), vec!["unknown".into()]),
            ]
            .into_iter()
            .collect(),
            level: None,
        };

        assert_eq!(
            variant.resolve(),
            Some(LevelFieldVariant {
                names: vec!["level".into()],
                values: vec![(Level::Info, vec!["info".into()])].into_iter().collect(),
                level: None,
            })
        );
    }

    #[test]
    fn test_unknown_level_main() {
        let variant = RawLevelFieldVariant {
            names: vec!["level".into()],
            values: vec![(InfallibleLevel::Valid(Level::Info), vec!["info".into()])]
                .into_iter()
                .collect(),
            level: Some(InfallibleLevel::Invalid("unknown".into())),
        };

        assert_eq!(variant.resolve(), None);
    }

    #[test]
    fn test_unknown_level_all_unknown() {
        let variant = RawLevelFieldVariant {
            names: vec!["level".into()],
            values: vec![(InfallibleLevel::Invalid("unknown".into()), vec!["unknown".into()])]
                .into_iter()
                .collect(),
            level: Some(InfallibleLevel::Valid(Level::Info)),
        };

        assert_eq!(variant.resolve(), None);
    }

    #[test]
    fn test_csl() {
        let csl = ListOrCommaSeparatedList::from(vec!["a", "b", "c"]);
        assert_eq!(csl.deref(), vec!["a", "b", "c"]);
        assert_eq!(Vec::from(csl), vec!["a", "b", "c"]);

        let csl: ListOrCommaSeparatedList<String> = serde_plain::from_str("a,b,c").unwrap();
        assert_eq!(csl.deref(), vec!["a", "b", "c"]);
        assert_eq!(Vec::from(csl), vec!["a", "b", "c"]);

        let csl: ListOrCommaSeparatedList<String> = serde_plain::from_str("").unwrap();
        assert_eq!(csl.deref(), Vec::<String>::new());

        let csl = serde_json::from_str::<ListOrCommaSeparatedList<String>>(r#""a,b,c""#).unwrap();
        assert_eq!(csl.deref(), vec!["a", "b", "c"]);

        let csl = serde_json::from_str::<ListOrCommaSeparatedList<String>>(r#""""#).unwrap();
        assert_eq!(csl.deref(), Vec::<String>::new());

        let res = serde_json::from_str::<ListOrCommaSeparatedList<String>>(r#"12"#);
        assert!(res.is_err());
    }

    #[test]
    fn test_ascii_mode_opt() {
        // Default value should be Auto
        assert_eq!(AsciiModeOpt::default(), AsciiModeOpt::Auto);
    }

    #[test]
    fn test_ascii_mode_opt_resolve() {
        // Test resolve with utf8_supported = true
        assert_eq!(AsciiModeOpt::Auto.resolve(true), AsciiMode::Off);
        assert_eq!(AsciiModeOpt::Always.resolve(true), AsciiMode::On);
        assert_eq!(AsciiModeOpt::Never.resolve(true), AsciiMode::Off);

        // Test resolve with utf8_supported = false
        assert_eq!(AsciiModeOpt::Auto.resolve(false), AsciiMode::On);
        assert_eq!(AsciiModeOpt::Always.resolve(false), AsciiMode::On);
        assert_eq!(AsciiModeOpt::Never.resolve(false), AsciiMode::Off);
    }

    #[test]
    fn test_display_variant_uniform() {
        let uniform = DisplayVariant::Uniform("test".to_string());

        // Uniform variant should return the same string regardless of mode
        assert_eq!(uniform.resolve(AsciiMode::On), "test");
        assert_eq!(uniform.resolve(AsciiMode::Off), "test");
    }

    #[test]
    fn test_display_variant_selective() {
        let selective = DisplayVariant::Selective {
            ascii: "ascii".to_string(),
            utf8: "utf8".to_string(),
        };

        // Selective variant should return the appropriate string based on mode
        assert_eq!(selective.resolve(AsciiMode::On), "ascii");
        assert_eq!(selective.resolve(AsciiMode::Off), "utf8");
    }

    #[test]
    fn test_display_variant_from_string() {
        let from_string = DisplayVariant::from("test".to_string());
        assert!(matches!(from_string, DisplayVariant::Uniform(_)));
        assert_eq!(from_string.resolve(AsciiMode::Off), "test");
    }

    #[test]
    fn test_display_variant_from_str() {
        let from_str = DisplayVariant::from("test");
        assert!(matches!(from_str, DisplayVariant::Uniform(_)));
        assert_eq!(from_str.resolve(AsciiMode::Off), "test");
    }

    #[test]
    fn test_punctuation_resolve() {
        // Use test_default instead of Default::default to avoid dependency on default config
        let mut punctuation = Punctuation::test_default();
        punctuation.input_number_right_separator = DisplayVariant::Selective {
            ascii: " | ".to_string(),
            utf8: " │ ".to_string(),
        };
        punctuation.source_location_separator = DisplayVariant::Selective {
            ascii: "-> ".to_string(),
            utf8: "→ ".to_string(),
        };

        // Test with direct resolve calls
        assert_eq!(punctuation.input_number_right_separator.resolve(AsciiMode::On), " | ");
        assert_eq!(punctuation.input_number_right_separator.resolve(AsciiMode::Off), " │ ");

        // Test ASCII mode through Punctuation::resolve
        let resolved_ascii = punctuation.resolve(AsciiMode::On);
        assert_eq!(resolved_ascii.input_number_right_separator, " | ");
        assert_eq!(resolved_ascii.source_location_separator, "-> ");

        // Test UTF-8 mode through Punctuation::resolve
        let resolved_utf8 = punctuation.resolve(AsciiMode::Off);
        assert_eq!(resolved_utf8.input_number_right_separator, " │ ");
        assert_eq!(resolved_utf8.source_location_separator, "→ ");

        // Test that all fields are correctly resolved
        for (ascii_val, utf8_val) in [
            (
                resolved_ascii.input_number_right_separator.as_str(),
                resolved_utf8.input_number_right_separator.as_str(),
            ),
            (
                resolved_ascii.source_location_separator.as_str(),
                resolved_utf8.source_location_separator.as_str(),
            ),
        ] {
            assert_ne!(ascii_val, utf8_val, "ASCII and UTF-8 values should be different");
        }
    }
}
