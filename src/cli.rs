// std imports
use std::path::PathBuf;

// third-party imports
use clap::{ArgAction, Parser, ValueEnum};
use std::num::NonZeroUsize;

// local imports
use crate::{
    config,
    error::*,
    level::{LevelValueParser, RelaxedLevel},
};

// ---

/// JSON log converter to human readable representation.
#[derive(Parser)]
#[clap(version, disable_help_flag = true)]
pub struct Opt {
    /// Color output options.
    #[arg(long, default_value = "auto", env = "HL_COLOR", overrides_with = "color")]
    #[arg(value_enum)]
    pub color: ColorOption,

    /// Handful alias for --color=always, overrides --color option.
    #[arg(short)]
    pub color_always: bool,

    /// Output paging options.
    #[arg(long, default_value = "auto", env = "HL_PAGING", overrides_with = "paging")]
    #[arg(value_enum)]
    pub paging: PagingOption,

    /// Handful alias for --paging=never, overrides --paging option.
    #[arg(short = 'P')]
    pub paging_never: bool,

    /// Color theme.
    #[arg(
        long,
        default_value_t = config::get().theme.clone(),
        env = "HL_THEME",
        overrides_with="theme",
    )]
    pub theme: String,

    /// Output raw JSON messages instead of formatter messages, it can be useful for applying filters and saving results in original format.
    #[arg(short, long, overrides_with = "raw")]
    pub raw: bool,

    /// Disable raw JSON messages output, overrides --raw option.
    #[arg(long, overrides_with = "raw")]
    _no_raw: bool,

    /// Disable unescaping and prettifying of field values.
    #[arg(long, overrides_with = "raw_fields")]
    pub raw_fields: bool,

    /// Allow non-JSON prefixes before JSON messages.
    #[arg(long, env = "HL_ALLOW_PREFIX", overrides_with = "allow_prefix")]
    pub allow_prefix: bool,

    /// Number of interrupts to ignore, i.e. Ctrl-C (SIGINT).
    #[arg(
        long,
        default_value = "3",
        env = "HL_INTERRUPT_IGNORE_COUNT",
        overrides_with = "interrupt_ignore_count"
    )]
    pub interrupt_ignore_count: usize,

    /// Buffer size.
    #[arg(long, default_value = "256 KiB", env="HL_BUFFER_SIZE",  value_parser = parse_non_zero_size, overrides_with="buffer_size")]
    pub buffer_size: NonZeroUsize,

    /// Maximum message size.
    #[arg(long, default_value = "64 MiB", env="HL_MAX_MESSAGE_SIZE",  value_parser = parse_non_zero_size, overrides_with="max_message_size")]
    pub max_message_size: NonZeroUsize,

    /// Number of processing threads.
    #[arg(long, short = 'C', env = "HL_CONCURRENCY", overrides_with = "concurrency")]
    pub concurrency: Option<usize>,

    /// Filtering by field values in one of forms [k=v, k~=v, k~~=v, 'k!=v', 'k!~=v', 'k!~~=v'] where ~ does substring match and ~~ does regular expression match.
    #[arg(short, long, number_of_values = 1)]
    pub filter: Vec<String>,

    /// Custom query, accepts expressions from --filter and supports '(', ')', 'and', 'or', 'not', 'in', 'contain', 'like', '<', '>', '<=', '>=', etc.
    #[arg(short, long, number_of_values = 1)]
    pub query: Vec<String>,

    /// Hide or reveal fields with the specified keys, prefix with ! to reveal, specify '!*' to reveal all.
    #[arg(long, short = 'h', number_of_values = 1)]
    pub hide: Vec<String>,

    /// Filtering by level.
    #[arg(short, long, env = "HL_LEVEL", overrides_with="level", ignore_case=true, value_parser = LevelValueParser)]
    #[arg(value_enum)]
    pub level: Option<RelaxedLevel>,

    /// Filtering by timestamp >= the value (--time-zone and --local options are honored).
    #[arg(long, allow_hyphen_values = true, overrides_with = "since")]
    pub since: Option<String>,

    /// Filtering by timestamp <= the value (--time-zone and --local options are honored).
    #[arg(long, allow_hyphen_values = true, overrides_with = "until")]
    pub until: Option<String>,

    /// Time format, see https://man7.org/linux/man-pages/man1/date.1.html.
    #[arg(
        short,
        long,
        env="HL_TIME_FORMAT",
        default_value_t = config::get().time_format.clone(),
        overrides_with = "time_format",
    )]
    pub time_format: String,

    /// Time zone name, see column "TZ identifier" at https://en.wikipedia.org/wiki/List_of_tz_database_time_zones.
    #[arg(long, short = 'Z', env="HL_TIME_ZONE", default_value = config::get().time_zone.name(), overrides_with="time_zone")]
    pub time_zone: chrono_tz::Tz,

    /// Use local time zone, overrides --time-zone option.
    #[arg(long, short = 'L', overrides_with = "local")]
    pub local: bool,

    /// Disable local time zone, overrides --local option.
    #[arg(long, overrides_with = "local")]
    _no_local: bool,

    /// Unix timestamp unit.
    #[arg(
        long,
        default_value = "auto",
        overrides_with = "unix_timestamp_unit",
        env = "HL_UNIX_TIMESTAMP_UNIT"
    )]
    pub unix_timestamp_unit: UnixTimestampUnit,

    /// Files to process
    #[arg(name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Hide empty fields, applies for null, string, object and array fields only.
    #[arg(
        long,
        short = 'e',
        env = "HL_HIDE_EMPTY_FIELDS",
        overrides_with = "hide_empty_fields"
    )]
    pub hide_empty_fields: bool,

    /// Show empty fields, overrides --hide-empty-fields option.
    #[arg(
        long,
        short = 'E',
        env = "HL_SHOW_EMPTY_FIELDS",
        overrides_with = "show_empty_fields"
    )]
    pub show_empty_fields: bool,

    /// Show input number and/or input filename before each message.
    #[arg(long, default_value = "auto", overrides_with = "input_info")]
    #[arg(value_enum)]
    pub input_info: InputInfoOption,

    /// List available themes and exit.
    #[arg(long)]
    pub list_themes: bool,

    /// Sort messages chronologically.
    #[arg(long, short = 's', overrides_with = "sort")]
    pub sort: bool,

    /// Follow input streams and sort messages chronologically during time frame set by --sync-interval-ms option.
    #[arg(long, short = 'F', overrides_with = "follow")]
    pub follow: bool,

    /// Number of last messages to preload from each file in --follow mode.
    #[arg(long, default_value = "10", overrides_with = "tail")]
    pub tail: u64,

    /// Synchronization interval for live streaming mode enabled by --follow option.
    #[arg(long, default_value = "100", overrides_with = "sync_interval_ms")]
    pub sync_interval_ms: u64,

    /// Output file.
    #[arg(long, short = 'o', overrides_with = "output")]
    pub output: Option<String>,

    /// Log message delimiter, [NUL, CR, LF, CRLF] or any custom string.
    #[arg(long, overrides_with = "delimiter")]
    pub delimiter: Option<String>,

    /// Input format.
    #[arg(
        long,
        env = "HL_INPUT_FORMAT",
        default_value = "auto",
        overrides_with = "input_format"
    )]
    pub input_format: InputFormat,

    /// Dump index metadata and exit.
    #[arg(long)]
    pub dump_index: bool,

    /// Print debug error messages that can help with troubleshooting.
    #[arg(long)]
    pub debug: bool,

    /// Print help.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub help: bool,
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
