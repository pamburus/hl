// std imports
use std::path::PathBuf;

// third-party imports
use clap::{value_parser, ArgAction, Args, Parser, ValueEnum};
use clap_complete::Shell;
use std::num::NonZeroUsize;

// local imports
use crate::{
    config,
    error::*,
    level::{LevelValueParser, RelaxedLevel},
    settings,
};

// ---

#[derive(Args)]
pub struct BootstrapArgs {
    /// Configuration file path.
    #[arg(
        long,
        overrides_with = "config",
        value_name = "FILE",
        env = "HL_CONFIG",
        num_args = 1
    )]
    pub config: Option<String>,
}

/// JSON and logfmt log converter to human readable representation.
#[derive(Parser)]
#[clap(version, disable_help_flag = true)]
pub struct BootstrapOpt {
    #[command(flatten)]
    pub args: BootstrapArgs,
}

impl BootstrapOpt {
    pub fn parse() -> Self {
        Self::parse_from(Self::args())
    }

    pub fn args() -> Vec<String> {
        let mut args = std::env::args();
        let Some(first) = args.next() else {
            return vec![];
        };

        let mut result = vec![first];
        let mut follow_up = false;

        while let Some(arg) = args.next() {
            match (arg.as_bytes(), follow_up) {
                (b"--", _) => {
                    break;
                }
                ([b'-', b'-', b'c', b'o', b'n', b'f', b'i', b'g', b'=', ..], _) => {
                    result.push(arg);
                    follow_up = false;
                }
                (b"--config", _) => {
                    result.push(arg);
                    follow_up = true;
                }
                ([b'-'], true) => {
                    result.push(arg);
                    follow_up = false;
                }
                ([b'-', ..], true) => {
                    follow_up = false;
                }
                (_, true) => {
                    result.push(arg);
                    follow_up = false;
                }
                _ => {}
            }
        }

        result
    }
}

// ---

/// JSON and logfmt log converter to human readable representation.
#[derive(Parser)]
#[clap(version, disable_help_flag = true)]
pub struct Opt {
    #[command(flatten)]
    pub bootstrap: BootstrapArgs,

    /// Sort messages chronologically.
    #[arg(long, short = 's', overrides_with = "sort")]
    pub sort: bool,

    /// Follow input streams and sort messages chronologically during time frame set by --sync-interval-ms option.
    #[arg(long, short = 'F', overrides_with = "follow")]
    pub follow: bool,

    /// Number of last messages to preload from each file in --follow mode.
    #[arg(long, default_value = "10", overrides_with = "tail", value_name = "N")]
    pub tail: u64,

    /// Synchronization interval for live streaming mode enabled by --follow option.
    #[arg(
        long,
        default_value = "100",
        overrides_with = "sync_interval_ms",
        value_name = "MILLISECONDS"
    )]
    pub sync_interval_ms: u64,

    /// Control pager usage (HL_PAGER or PAGER).
    #[arg(
        long,
        default_value = "auto",
        env = "HL_PAGING",
        overrides_with = "paging",
        value_name = "WHEN",
        value_enum
    )]
    pub paging: PagingOption,

    /// Handful alias for --paging=never, overrides --paging option.
    #[arg(short = 'P')]
    pub paging_never: bool,

    /// Filter messages by level.
    #[arg(
        short,
        long,
        env = "HL_LEVEL",
        overrides_with="level",
        ignore_case=true,
        value_parser = LevelValueParser,
        value_enum,
        help_heading = heading::FILTERING
    )]
    pub level: Option<RelaxedLevel>,

    /// Filter messages by timestamp >= <TIME> (--time-zone and --local options are honored).
    #[arg(
        long,
        allow_hyphen_values = true,
        overrides_with = "since",
        value_name = "TIME",
        help_heading = heading::FILTERING
    )]
    pub since: Option<String>,

    /// Filter messages by timestamp <= <TIME> (--time-zone and --local options are honored).
    #[arg(
        long,
        allow_hyphen_values = true,
        overrides_with = "until",
        value_name = "TIME",
        help_heading = heading::FILTERING
    )]
    pub until: Option<String>,

    /// Filter messages by field values
    /// [k=v, k~=v, k~~=v, 'k!=v', 'k!~=v', 'k!~~=v']
    /// where ~ does substring match and ~~ does regular expression match.
    #[arg(short, long, number_of_values = 1, help_heading = heading::FILTERING)]
    pub filter: Vec<String>,

    /// Filter using query, accepts expressions from --filter
    /// and supports '(', ')', 'and', 'or', 'not', 'in', 'contain', 'like', '<', '>', '<=', '>=', etc.
    #[arg(short, long, number_of_values = 1, help_heading = heading::FILTERING)]
    pub query: Vec<String>,

    /// Color output control.
    #[arg(
        long,
        default_value = "auto",
        env = "HL_COLOR",
        overrides_with_all = ["color", "color_always"],
        default_missing_value = "always",
        num_args = 0..=1,
        value_name = "WHEN",
        value_enum,
        help_heading = heading::OUTPUT
    )]
    pub color: ColorOption,

    /// Handful alias for --color=always, overrides --color option.
    #[arg(
        short,
        overrides_with_all = ["color", "color_always"],
        help_heading = heading::OUTPUT
    )]
    pub color_always: bool,

    /// Color theme.
    #[arg(
        long,
        default_value_t = config::global::get().theme.clone(),
        env = "HL_THEME",
        overrides_with="theme",
        help_heading = heading::OUTPUT
    )]
    pub theme: String,

    /// Output raw source messages instead of formatted messages, which can be useful for applying filters and saving results in their original format.
    #[arg(short, long, overrides_with = "raw", help_heading = heading::OUTPUT)]
    pub raw: bool,

    /// Disable raw source messages output, overrides --raw option.
    #[arg(long, overrides_with = "raw", help_heading = heading::OUTPUT)]
    _no_raw: bool,

    /// Output field values as is, without unescaping or prettifying.
    #[arg(long, overrides_with = "raw_fields", help_heading = heading::OUTPUT)]
    pub raw_fields: bool,

    /// Hide or reveal fields with the specified keys, prefix with ! to reveal, specify '!*' to reveal all.
    #[arg(
        long,
        short = 'h',
        number_of_values = 1,
        value_name = "KEY",
        help_heading = heading::OUTPUT
    )]
    pub hide: Vec<String>,

    /// Whether to flatten objects.
    #[arg(
        long,
        env = "HL_FLATTEN",
        value_name = "WHEN",
        value_enum,
        default_value_t = config::global::get().formatting.flatten.into(),
        overrides_with = "flatten",
        help_heading = heading::OUTPUT
    )]
    pub flatten: FlattenOption,

    /// Time format, see https://man7.org/linux/man-pages/man1/date.1.html.
    #[arg(
        short,
        long,
        env="HL_TIME_FORMAT",
        default_value_t = config::global::get().time_format.clone(),
        overrides_with = "time_format",
        value_name = "FORMAT",
        help_heading = heading::OUTPUT
    )]
    pub time_format: String,

    /// Time zone name, see column "TZ identifier" at https://en.wikipedia.org/wiki/List_of_tz_database_time_zones.
    #[arg(
        long,
        short = 'Z',
        env="HL_TIME_ZONE",
        default_value = config::global::get().time_zone.name(),
        overrides_with="time_zone",
        value_name = "TZ",
        help_heading = heading::OUTPUT
    )]
    pub time_zone: chrono_tz::Tz,

    /// Use local time zone, overrides --time-zone option.
    #[arg(long, short = 'L', overrides_with = "local", help_heading = heading::OUTPUT)]
    pub local: bool,

    /// Disable local time zone, overrides --local option.
    #[arg(long, overrides_with = "local", help_heading = heading::OUTPUT)]
    _no_local: bool,

    /// Hide empty fields, applies for null, string, object and array fields only.
    #[arg(
        long,
        short = 'e',
        env = "HL_HIDE_EMPTY_FIELDS",
        overrides_with = "hide_empty_fields",
        help_heading = heading::OUTPUT
    )]
    pub hide_empty_fields: bool,

    /// Show empty fields, overrides --hide-empty-fields option.
    #[arg(
        long,
        short = 'E',
        env = "HL_SHOW_EMPTY_FIELDS",
        overrides_with = "show_empty_fields",
        help_heading = heading::OUTPUT
    )]
    pub show_empty_fields: bool,

    /// Whether to expand fields and messages.
    #[arg(
        long,
        short = 'x',
        env = "HL_EXPAND",
        value_name = "WHEN",
        value_enum,
        default_value_t = config::global::get().formatting.expansion.mode.into(),
        overrides_with = "expand",
        default_missing_value = "always",
        help_heading = heading::OUTPUT,
    )]
    pub expand: ExpandOption,

    /// Show input number and/or input filename before each message.
    #[arg(
        long,
        default_value = "auto",
        overrides_with = "input_info",
        value_name = "VARIANT",
        value_enum,
        help_heading = heading::OUTPUT
    )]
    pub input_info: InputInfoOption,

    /// Output file.
    #[arg(long, short = 'o', overrides_with = "output", value_name = "FILE", help_heading = heading::OUTPUT)]
    pub output: Option<String>,

    /// Input format.
    #[arg(
        long,
        env = "HL_INPUT_FORMAT",
        default_value = "auto",
        overrides_with = "input_format",
        value_name = "FORMAT",
        help_heading = heading::INPUT
    )]
    pub input_format: InputFormat,

    /// Unix timestamp unit.
    #[arg(
        long,
        default_value = "auto",
        overrides_with = "unix_timestamp_unit",
        env = "HL_UNIX_TIMESTAMP_UNIT",
        value_name = "UNIT",
        help_heading = heading::INPUT
    )]
    pub unix_timestamp_unit: UnixTimestampUnit,

    /// Allow non-JSON prefixes before JSON messages.
    #[arg(long, env = "HL_ALLOW_PREFIX", overrides_with = "allow_prefix", help_heading = heading::INPUT)]
    pub allow_prefix: bool,

    /// Log message delimiter, [NUL, CR, LF, CRLF] or any custom string.
    #[arg(long, overrides_with = "delimiter", help_heading = heading::INPUT)]
    pub delimiter: Option<String>,

    /// Number of interrupts to ignore, i.e. Ctrl-C (SIGINT).
    #[arg(
        long,
        default_value = "3",
        env = "HL_INTERRUPT_IGNORE_COUNT",
        overrides_with = "interrupt_ignore_count",
        value_name = "N",
        help_heading = heading::ADVANCED
    )]
    pub interrupt_ignore_count: usize,

    /// Buffer size.
    #[arg(
        long,
        default_value = "256 KiB",
        env="HL_BUFFER_SIZE",
        value_parser = parse_non_zero_size,
        overrides_with="buffer_size",
        value_name = "SIZE",
        help_heading = heading::ADVANCED
    )]
    pub buffer_size: NonZeroUsize,

    /// Maximum message size.
    #[arg(
        long,
        default_value = "64 MiB",
        env="HL_MAX_MESSAGE_SIZE",
        value_parser = parse_non_zero_size,
        overrides_with="max_message_size",
        value_name = "SIZE",
        help_heading = heading::ADVANCED
    )]
    pub max_message_size: NonZeroUsize,

    /// Number of processing threads.
    #[arg(
        long,
        short = 'C',
        env = "HL_CONCURRENCY",
        overrides_with = "concurrency",
        value_name = "N",
        help_heading = heading::ADVANCED
    )]
    pub concurrency: Option<usize>,

    /// Print shell auto-completion script and exit.
    #[arg(
        long,
        value_parser = value_parser!(Shell),
        value_name = "SHELL",
        help_heading = heading::ADVANCED,
    )]
    pub shell_completions: Option<Shell>,

    /// Print man page and exit.
    #[arg(long, help_heading = heading::ADVANCED)]
    pub man_page: bool,

    /// Print available themes and exit.
    #[arg(long, help_heading = heading::ADVANCED)]
    pub list_themes: bool,

    /// Print debug index metadata (in --sort mode) and exit.
    #[arg(long, requires = "sort", help_heading = heading::ADVANCED)]
    pub dump_index: bool,

    /// Print debug error messages that can help with troubleshooting.
    #[arg(long, help_heading = heading::ADVANCED)]
    pub debug: bool,

    /// Print help.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub help: bool,

    /// Files to process
    #[arg(name = "FILE")]
    pub files: Vec<PathBuf>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ColorOption {
    Auto,
    Always,
    Never,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum PagingOption {
    Auto,
    Always,
    Never,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum InputInfoOption {
    Auto,
    None,
    Full,
    Compact,
    Minimal,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum InputFormat {
    Auto,
    Json,
    Logfmt,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum UnixTimestampUnit {
    Auto,
    S,
    Ms,
    Us,
    Ns,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlattenOption {
    Never,
    #[default]
    Always,
}

impl From<settings::FlattenOption> for FlattenOption {
    fn from(value: settings::FlattenOption) -> Self {
        match value {
            settings::FlattenOption::Never => Self::Never,
            settings::FlattenOption::Always => Self::Always,
        }
    }
}

impl From<Option<settings::FlattenOption>> for FlattenOption {
    fn from(value: Option<settings::FlattenOption>) -> Self {
        value.map(|x| x.into()).unwrap_or_default()
    }
}

impl Into<settings::FlattenOption> for FlattenOption {
    fn into(self) -> settings::FlattenOption {
        match self {
            Self::Never => settings::FlattenOption::Never,
            Self::Always => settings::FlattenOption::Always,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpandOption {
    #[default]
    Auto,
    Never,
    Always,
}

impl From<settings::ExpandOption> for ExpandOption {
    fn from(value: settings::ExpandOption) -> Self {
        match value {
            settings::ExpandOption::Auto => Self::Auto,
            settings::ExpandOption::Never => Self::Never,
            settings::ExpandOption::Always => Self::Always,
        }
    }
}

impl From<Option<settings::ExpandOption>> for ExpandOption {
    fn from(value: Option<settings::ExpandOption>) -> Self {
        Self::from(value.unwrap_or_default())
    }
}

impl Into<settings::ExpandOption> for ExpandOption {
    fn into(self) -> settings::ExpandOption {
        match self {
            Self::Auto => settings::ExpandOption::Auto,
            Self::Never => settings::ExpandOption::Never,
            Self::Always => settings::ExpandOption::Always,
        }
    }
}

mod heading {
    pub const FILTERING: &str = "Filtering Options";
    pub const INPUT: &str = "Input Options";
    pub const OUTPUT: &str = "Output Options";
    pub const ADVANCED: &str = "Advanced Options";
}

fn parse_size(s: &str) -> std::result::Result<usize, SizeParseError> {
    match bytefmt::parse(s) {
        Ok(value) => Ok(usize::try_from(value)?),
        Err(_) => {
            if let Ok(value) = bytefmt::parse(s.to_owned() + "ib") {
                return Ok(usize::try_from(value)?);
            }
            Err(SizeParseError::InvalidSize(s.into()))
        }
    }
}

fn parse_non_zero_size(s: &str) -> std::result::Result<NonZeroUsize, NonZeroSizeParseError> {
    if let Some(value) = NonZeroUsize::new(parse_size(s)?) {
        Ok(NonZeroUsize::from(value))
    } else {
        Err(NonZeroSizeParseError::ZeroSize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_option() {
        assert_eq!(FlattenOption::from(None), FlattenOption::Always);
        assert_eq!(
            FlattenOption::from(Some(settings::FlattenOption::Never)),
            FlattenOption::Never
        );
        assert_eq!(
            FlattenOption::from(Some(settings::FlattenOption::Always)),
            FlattenOption::Always
        );
        assert_eq!(
            FlattenOption::from(settings::FlattenOption::Never),
            FlattenOption::Never
        );
        assert_eq!(
            FlattenOption::from(settings::FlattenOption::Always),
            FlattenOption::Always
        );
        assert_eq!(
            Into::<settings::FlattenOption>::into(FlattenOption::Never),
            settings::FlattenOption::Never
        );
        assert_eq!(
            Into::<settings::FlattenOption>::into(FlattenOption::Always),
            settings::FlattenOption::Always
        );
    }

    #[test]
    fn test_expand_option() {
        assert_eq!(ExpandOption::from(None), ExpandOption::Auto);
        assert_eq!(
            ExpandOption::from(Some(settings::ExpandOption::Auto)),
            ExpandOption::Auto
        );
        assert_eq!(
            ExpandOption::from(Some(settings::ExpandOption::Never)),
            ExpandOption::Never
        );
        assert_eq!(
            ExpandOption::from(Some(settings::ExpandOption::Always)),
            ExpandOption::Always
        );
        assert_eq!(ExpandOption::from(settings::ExpandOption::Auto), ExpandOption::Auto);
        assert_eq!(ExpandOption::from(settings::ExpandOption::Never), ExpandOption::Never);
        assert_eq!(ExpandOption::from(settings::ExpandOption::Always), ExpandOption::Always);
        assert_eq!(
            Into::<settings::ExpandOption>::into(ExpandOption::Auto),
            settings::ExpandOption::Auto
        );
        assert_eq!(
            Into::<settings::ExpandOption>::into(ExpandOption::Never),
            settings::ExpandOption::Never
        );
        assert_eq!(
            Into::<settings::ExpandOption>::into(ExpandOption::Always),
            settings::ExpandOption::Always
        );
    }
}
