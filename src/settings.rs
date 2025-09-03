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

// test imports
#[cfg(test)]
use crate::testing::Sample;

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

#[cfg(test)]
impl Sample for Formatting {
    fn sample() -> Self {
        Self {
            flatten: None,
            message: MessageFormatting {
                format: MessageFormat::AutoQuoted,
            },
            punctuation: Punctuation::sample(),
        }
    }
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

/// Configuration for various punctuation marks used in log formatting.
///
/// This struct defines how various separators, quotes, and indicators appear
/// in the formatted output. Many of these can be configured to display differently
/// when in ASCII mode versus Unicode mode through the use of `DisplayVariant`.
///
/// The configuration is used to create a `ResolvedPunctuation` instance when
/// the ASCII mode is determined at runtime.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Punctuation {
    pub logger_name_separator: DisplayVariant,
    pub field_key_value_separator: DisplayVariant,
    pub string_opening_quote: DisplayVariant,
    pub string_closing_quote: DisplayVariant,
    pub source_location_separator: DisplayVariant,
    pub caller_name_file_separator: DisplayVariant,
    pub hidden_fields_indicator: DisplayVariant,
    pub level_left_separator: DisplayVariant,
    pub level_right_separator: DisplayVariant,
    pub input_number_prefix: DisplayVariant,
    pub input_number_left_separator: DisplayVariant,
    pub input_number_right_separator: DisplayVariant,
    pub input_name_left_separator: DisplayVariant,
    pub input_name_right_separator: DisplayVariant,
    pub input_name_clipping: DisplayVariant,
    pub input_name_common_part: DisplayVariant,
    pub array_separator: DisplayVariant,
    pub message_delimiter: DisplayVariant,
}

impl Punctuation {
    pub fn resolve(&self, mode: AsciiMode) -> ResolvedPunctuation {
        ResolvedPunctuation {
            logger_name_separator: Self::resolve_field(&self.logger_name_separator, mode),
            field_key_value_separator: Self::resolve_field(&self.field_key_value_separator, mode),
            string_opening_quote: Self::resolve_field(&self.string_opening_quote, mode),
            string_closing_quote: Self::resolve_field(&self.string_closing_quote, mode),
            source_location_separator: Self::resolve_field(&self.source_location_separator, mode),
            caller_name_file_separator: Self::resolve_field(&self.caller_name_file_separator, mode),
            hidden_fields_indicator: Self::resolve_field(&self.hidden_fields_indicator, mode),
            level_left_separator: Self::resolve_field(&self.level_left_separator, mode),
            level_right_separator: Self::resolve_field(&self.level_right_separator, mode),
            input_number_prefix: Self::resolve_field(&self.input_number_prefix, mode),
            input_number_left_separator: Self::resolve_field(&self.input_number_left_separator, mode),
            input_number_right_separator: Self::resolve_field(&self.input_number_right_separator, mode),
            input_name_left_separator: Self::resolve_field(&self.input_name_left_separator, mode),
            input_name_right_separator: Self::resolve_field(&self.input_name_right_separator, mode),
            input_name_clipping: Self::resolve_field(&self.input_name_clipping, mode),
            input_name_common_part: Self::resolve_field(&self.input_name_common_part, mode),
            array_separator: Self::resolve_field(&self.array_separator, mode),
            message_delimiter: Self::resolve_field(&self.message_delimiter, mode),
        }
    }

    fn resolve_field(field: &DisplayVariant, mode: AsciiMode) -> String {
        String::from(field.resolve(mode))
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

#[cfg(test)]
impl Sample for Punctuation {
    fn sample() -> Self {
        Self {
            logger_name_separator: ":".into(),
            field_key_value_separator: "=".into(),
            string_opening_quote: "'".into(),
            string_closing_quote: "'".into(),
            source_location_separator: DisplayVariant::ascii("-> ").unicode("→ "),
            caller_name_file_separator: " @ ".into(),
            hidden_fields_indicator: DisplayVariant::ascii(" ...").unicode(" …"),
            level_left_separator: "|".into(),
            level_right_separator: "|".into(),
            input_number_prefix: "#".into(),
            input_number_left_separator: "".into(),
            input_number_right_separator: DisplayVariant::ascii(" | ").unicode(" │ "),
            input_name_left_separator: "".into(),
            input_name_right_separator: " | ".into(),
            input_name_clipping: DisplayVariant::ascii("..").unicode("··"),
            input_name_common_part: DisplayVariant::ascii("**").unicode("★★"),
            array_separator: ", ".into(),
            message_delimiter: "::".into(),
        }
    }
}

/// A structure that contains resolved punctuation marks for formatting log output.
/// This structure is created by resolving the `Punctuation` configuration
/// according to the current ASCII mode setting.
#[derive(Clone)]
pub struct ResolvedPunctuation {
    pub logger_name_separator: String,
    pub field_key_value_separator: String,
    pub string_opening_quote: String,
    pub string_closing_quote: String,
    pub source_location_separator: String,
    pub caller_name_file_separator: String,
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
    pub message_delimiter: String,
}

/// Configuration option for ASCII mode.
///
/// This enum allows users to control whether the output should use ASCII-only characters
/// or allow Unicode characters (when UTF-8 encoding is supported):
///
/// - `Auto`: Automatically choose based on terminal UTF-8 encoding support (default)
/// - `Always`: Always use ASCII-only characters
/// - `Never`: Always allow Unicode characters
///
/// When set to `Auto`, the program will detect whether the terminal supports UTF-8 encoding
/// and choose the appropriate mode automatically.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum AsciiModeOpt {
    #[default]
    Auto,
    Always,
    Never,
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

/// Controls whether ASCII-only characters should be used in formatted output.
///
/// The formatter can produce output in either ASCII-only mode or with full Unicode characters,
/// depending on terminal capabilities and user preferences.
///
/// * `Off` - Use full Unicode character set (default)
/// * `On` - Use ASCII-only characters
///
/// This mode is usually determined by resolving an `AsciiModeOpt` configuration
/// setting against the detected terminal capabilities.
#[derive(Default, Debug, Clone, PartialEq, Eq, Copy)]
pub enum AsciiMode {
    #[default]
    Off,
    On,
}

/// A configuration type that allows for different display styles in ASCII and Unicode modes.
///
/// This type can either contain a single string to be used in all contexts (`Uniform`),
/// or separate strings for ASCII and Unicode output modes (`Selective`).
///
/// # Examples
///
/// ```
/// use hl::settings::DisplayVariant;
///
/// // Uniform variant - same in both modes
/// let separator = DisplayVariant::Uniform(" | ".to_string());
///
/// // Selective variant - different representation in ASCII vs Unicode mode
/// let separator = DisplayVariant::Selective {
///     ascii: " | ".to_string(),
///     unicode: " │ ".to_string(),
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum DisplayVariant {
    Uniform(String),
    Selective {
        ascii: String,
        #[serde(rename = "unicode")]
        unicode: String,
    },
}

impl DisplayVariant {
    pub fn resolve(&self, mode: AsciiMode) -> &str {
        match self {
            Self::Uniform(s) => &s,
            Self::Selective { ascii, unicode } => match mode {
                AsciiMode::Off => &unicode,
                AsciiMode::On => &ascii,
            },
        }
    }

    pub fn ascii<S>(s: S) -> DisplayVariantAscii<S>
    where
        S: Into<String>,
    {
        DisplayVariantAscii { ascii: s }
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

// ---

pub struct DisplayVariantAscii<S> {
    ascii: S,
}

impl<S: Into<String>> DisplayVariantAscii<S> {
    pub fn unicode(self, s: impl Into<String>) -> DisplayVariant {
        DisplayVariant::Selective {
            ascii: self.ascii.into(),
            unicode: s.into(),
        }
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
            unicode: "unicode".to_string(),
        };

        // Selective variant should return the appropriate string based on mode
        assert_eq!(selective.resolve(AsciiMode::On), "ascii");
        assert_eq!(selective.resolve(AsciiMode::Off), "unicode");
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
        assert_eq!(from_str, DisplayVariant::Uniform("test".to_string()));
    }

    #[test]
    fn test_display_variant_resolve() {
        // Test with uniform variant
        let uniform = DisplayVariant::Uniform("test".to_string());
        assert_eq!(uniform.resolve(AsciiMode::On), "test");
        assert_eq!(uniform.resolve(AsciiMode::Off), "test");

        // Test with selective variant
        let selective = DisplayVariant::Selective {
            ascii: "ascii".to_string(),
            unicode: "unicode".to_string(),
        };
        assert_eq!(selective.resolve(AsciiMode::On), "ascii");
        assert_eq!(selective.resolve(AsciiMode::Off), "unicode");
    }

    #[test]
    fn test_punctuation_resolve() {
        // Use Punctuation::sample instead of Default::default to avoid dependency on default config
        let mut punctuation = Punctuation::sample();

        // Set up selective variants for multiple punctuation elements
        punctuation.input_number_right_separator = DisplayVariant::Selective {
            ascii: " | ".to_string(),
            unicode: " │ ".to_string(),
        };
        punctuation.source_location_separator = DisplayVariant::Selective {
            ascii: "-> ".to_string(),
            unicode: "→ ".to_string(),
        };
        punctuation.array_separator = DisplayVariant::Selective {
            ascii: ", ".to_string(),
            unicode: "· ".to_string(),
        };
        punctuation.hidden_fields_indicator = DisplayVariant::Selective {
            ascii: "...".to_string(),
            unicode: "…".to_string(),
        };

        // Test with direct resolve calls
        assert_eq!(punctuation.input_number_right_separator.resolve(AsciiMode::On), " | ");
        assert_eq!(punctuation.input_number_right_separator.resolve(AsciiMode::Off), " │ ");
        assert_eq!(punctuation.source_location_separator.resolve(AsciiMode::On), "-> ");
        assert_eq!(punctuation.source_location_separator.resolve(AsciiMode::Off), "→ ");
        assert_eq!(punctuation.array_separator.resolve(AsciiMode::On), ", ");
        assert_eq!(punctuation.array_separator.resolve(AsciiMode::Off), "· ");
        assert_eq!(punctuation.hidden_fields_indicator.resolve(AsciiMode::On), "...");
        assert_eq!(punctuation.hidden_fields_indicator.resolve(AsciiMode::Off), "…");

        // Test ASCII mode through Punctuation::resolve
        let resolved_ascii = punctuation.resolve(AsciiMode::On);
        let resolved_utf8 = punctuation.resolve(AsciiMode::Off);

        // Verify ASCII version of resolved punctuation
        assert_eq!(resolved_ascii.input_number_right_separator, " | ");
        assert_eq!(resolved_ascii.source_location_separator, "-> ");
        assert_eq!(resolved_ascii.array_separator, ", ");
        assert_eq!(resolved_ascii.hidden_fields_indicator, "...");

        // Verify Unicode version of resolved punctuation
        assert_eq!(resolved_utf8.input_number_right_separator, " │ ");
        assert_eq!(resolved_utf8.source_location_separator, "→ ");
        assert_eq!(resolved_utf8.array_separator, "· ");
        assert_eq!(resolved_utf8.hidden_fields_indicator, "…");

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
            assert_ne!(ascii_val, utf8_val, "ASCII and Unicode values should be different");
        }
    }
}
