// std imports
use std::{num::NonZeroUsize, path::PathBuf};

// third-party imports
use clap::{
    Args, Parser, ValueEnum,
    builder::{Styles, styling::AnsiColor},
    value_parser,
};
use clap_complete::Shell;
use const_str::concat;
use styled_help::styled_help;

// local imports
use crate::{
    config,
    error::*,
    level::{LevelValueParser, RelaxedLevel},
    output::OutputDelimiter,
    settings::{self, AsciiModeOpt, ExpansionMode, InputInfo},
    themecfg,
};
use enumset_ext::convert::str::EnumSet;

// ---

const VERSION: &str = env!("VERSION");

// ---

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default())
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Yellow.on_default())
    .context(AnsiColor::Cyan.on_default().dimmed())
    .context_value(AnsiColor::Cyan.on_default());

#[styled_help]
#[derive(Args)]
pub struct BootstrapArgs {
    /// Configuration file path
    #[arg(long, value_name = "FILE", env = "HL_CONFIG", num_args = 1)]
    pub config: Vec<String>,
}

/// JSON and logfmt log converter to human readable representation.
#[derive(Parser)]
#[command(version, styles = STYLES, disable_help_flag = true)]
pub struct BootstrapOpt {
    #[command(flatten)]
    pub args: BootstrapArgs,
}

impl BootstrapOpt {
    pub fn parse() -> Self {
        Self::parse_from(Self::args())
    }

    pub fn args() -> Vec<String> {
        let mut args = wild::args();
        let Some(first) = args.next() else {
            return vec![];
        };

        let mut result = vec![first];
        let mut follow_up = false;

        for arg in args {
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
#[styled_help]
#[derive(Parser)]
#[command(version = VERSION, styles = STYLES, disable_help_flag = true)]
pub struct Opt {
    #[command(flatten)]
    pub bootstrap: BootstrapArgs,

    /// Sort entries chronologically
    #[arg(long, short = 's', overrides_with = "sort")]
    pub sort: bool,

    /// Follow input streams and sort entries chronologically within time frame set by <c>--sync-interval-ms</> option
    #[arg(long, short = 'F', overrides_with = "follow")]
    pub follow: bool,

    /// Number of last entries to preload from each file in <c>--follow</> mode
    #[arg(long, default_value = "10", overrides_with = "tail", value_name = "N")]
    pub tail: u64,

    /// Synchronization interval for live streaming mode enabled by <c>--follow</> option
    #[arg(
        long,
        default_value = "100",
        overrides_with = "sync_interval_ms",
        value_name = "MILLISECONDS"
    )]
    pub sync_interval_ms: u64,

    /// Control pager usage (HL_PAGER or PAGER)
    #[arg(
        long,
        default_value = "auto",
        env = "HL_PAGING",
        overrides_with = "paging",
        value_name = "WHEN",
        value_enum
    )]
    pub paging: PagingOption,

    /// Handful alias for <c>--paging=never</>, overrides <c>--paging</> option
    #[arg(short = 'P')]
    pub paging_never: bool,

    /// Display entries with level <s>>>=</> <c><<LEVEL>></>
    #[arg(
        short,
        long,
        env = "HL_LEVEL",
        overrides_with = "level",
        ignore_case = true,
        value_parser = LevelValueParser,
        value_enum,
        help_heading = heading::FILTERING
    )]
    pub level: Option<RelaxedLevel>,

    /// Display entries with timestamp <s>>>=</> <c><<TIME>></>
    ///
    /// Note that <c>--time-zone</> and <c>--local</> options are honored.
    #[arg(
        long,
        allow_hyphen_values = true,
        overrides_with = "since",
        value_name = "TIME",
        help_heading = heading::FILTERING
    )]
    pub since: Option<String>,

    /// Display entries with timestamp <s><<=</> <c><<TIME>></>
    ///
    /// Note that <c>--time-zone</> and <c>--local</> options are honored.
    #[arg(
        long,
        allow_hyphen_values = true,
        overrides_with = "until",
        value_name = "TIME",
        help_heading = heading::FILTERING
    )]
    pub until: Option<String>,

    /// Filter entries by matching field values <c><dim>[</>k=v<dim>, </>k~=v<dim>, </>k~~=v<dim>, </>'k!=v'<dim>, </>'k?!=v'<dim>, etc]</></>
    ///
    /// The format is <c><<key>> <<operator>> <<value>></>.
    /// The <c><<key>></> is the field name to match.
    /// The <c><<value>></> is the string to match against (case-sensitive).
    ///
    /// Operators:
    /// •   <c>= <dim>:</></> perform exact string match
    /// •  <c>~= <dim>:</></> perform sub-string match
    /// • <c>~~= <dim>:</></> perform regular expression match
    ///
    /// Modifiers:
    /// •   <c>! <dim>:</></> negate the match (placed before operator) <c><dim>[</>'k!=v'<dim>, </>'k!~=v'<dim>, etc]</></>
    /// •   <c>? <dim>:</></> include entry if the field is missing (placed after the key) <c><dim>[</>'k?=v'<dim>, </>'k?!~=v'<dim>, etc]</></>
    #[arg(
        short,
        long,
        num_args = 1,
        help_heading = heading::FILTERING
    )]
    pub filter: Vec<String>,

    /// Filter entries using a query expression <c><dim>[</>'status>>=400 or duration>>=15'<dim>, etc]</></>
    ///
    /// Query expression supports all operators and modifiers from <c>--filter</> and additionally
    /// • Logical: <c>'and'</>, <c>'or'</>, <c>'not'</> (aliases: <c>'&&'</>, <c>'||'</>, <c>'!'</>)
    /// • Comparison: <c>'<<'</>, <c>'>>'</>, <c>'<<='</>, <c>'>>='</>, <c>'='</>, <c>'!='</>
    /// • Sets: <c>'status in (500,503,504)'</>, <c>'id in @ids.txt'</> (values from file), <c>'id in @-'</> (values from stdin)
    /// • Strings: <c>'message contains "timeout"'</> (substring), <c>'message matches "^Error.*timeout$"'</> (regular expression)
    /// • Existence: <c>'exists(user-id)'</>, <c>'not exists(user-id)'</>
    /// • Grouping: <c>'(status>>=500 and status<<=504) or (status==404)'</>
    #[arg(short, long, num_args = 1, help_heading = heading::FILTERING)]
    pub query: Vec<String>,

    /// Whether to use ANSI colors and styles
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

    /// Handful alias for <c>--color=always</>, overrides <c>--color</> option
    #[arg(
        short,
        overrides_with_all = ["color", "color_always"],
        help_heading = heading::OUTPUT
    )]
    pub color_always: bool,

    /// Color theme
    ///
    /// Run <c>hl --list-themes</> to see available themes.
    #[arg(
        long,
        default_value_t = config::global::get().theme.clone(),
        env = "HL_THEME",
        overrides_with="theme",
        help_heading = heading::OUTPUT
    )]
    pub theme: String,

    /// Output raw source entries instead of formatted entries
    ///
    /// This can be useful for applying filters and saving results in their original format.
    #[arg(short, long, overrides_with = "raw", help_heading = heading::OUTPUT)]
    pub raw: bool,

    /// Disable raw source entries output, overrides <c>--raw</> option
    #[arg(long, overrides_with = "raw", help_heading = heading::OUTPUT)]
    _no_raw: bool,

    /// Output field values as is, without unescaping or prettifying
    #[arg(long, overrides_with = "raw_fields", help_heading = heading::OUTPUT)]
    pub raw_fields: bool,

    /// Hide or reveal fields with the specified keys, prefix with <c>!</> to reveal, provide <c>'!*'</> to reveal all
    #[arg(
        long,
        short = 'h',
        num_args = 1,
        value_name = "KEY",
        help_heading = heading::OUTPUT
    )]
    pub hide: Vec<String>,

    /// Whether to flatten objects
    #[arg(
        long,
        env = "HL_FLATTEN",
        value_name = "WHEN",
        value_enum,
        default_value_t = config::global::get().formatting.flatten.as_ref().map(|x| match x{
            settings::FlattenOption::Never => FlattenOption::Never,
            settings::FlattenOption::Always => FlattenOption::Always,
        }).unwrap_or(FlattenOption::Always),
        overrides_with = "flatten",
        help_heading = heading::OUTPUT
    )]
    pub flatten: FlattenOption,

    /// Time format, see <b>https://man7.org/linux/man-pages/man1/date.1.html</>
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

    /// Time zone name, see column "TZ identifier" at <b>https://en.wikipedia.org/wiki/List_of_tz_database_time_zones</>
    ///
    /// Examples: <c>'UTC'</>, <c>'America/New_York'</>, <c>'Asia/Shanghai'</>, <c>'Europe/Berlin'</>, etc.
    #[arg(
        long,
        short = 'Z',
        env="HL_TIME_ZONE",
        default_value = config::global::get().time_zone.name(),
        overrides_with="time_zone",
        value_name = "TZ",
        help_heading = heading::OUTPUT,
    )]
    pub time_zone: chrono_tz::Tz,

    /// Use local time zone, overrides <c>--time-zone</> option
    #[arg(long, short = 'L', overrides_with = "local", help_heading = heading::OUTPUT)]
    pub local: bool,

    /// Disable local time zone, overrides <c>--local</> option
    #[arg(long, overrides_with = "local", help_heading = heading::OUTPUT)]
    _no_local: bool,

    /// Hide empty fields, applies for null, string, object and array fields only
    #[arg(
        long,
        short = 'e',
        env = "HL_HIDE_EMPTY_FIELDS",
        overrides_with = "hide_empty_fields",
        help_heading = heading::OUTPUT
    )]
    pub hide_empty_fields: bool,

    /// Show empty fields, overrides <c>--hide-empty-fields</> option
    #[arg(
        long,
        short = 'E',
        env = "HL_SHOW_EMPTY_FIELDS",
        overrides_with = "show_empty_fields",
        help_heading = heading::OUTPUT
    )]
    pub show_empty_fields: bool,

    /// Input number and filename layouts
    #[arg(
        long,
        overrides_with = "input_info",
        default_value_t = config::global::get().input_info.into(),
        value_parser = InputInfoSet::clap_parser(),
        value_name = "LAYOUTS",
        help_heading = heading::OUTPUT
    )]
    pub input_info: InputInfoSet,

    /// Whether to restrict punctuation to ASCII characters only
    ///
    /// When enabled, unicode punctuation (like fancy quotes) will be replaced with ASCII equivalents.
    /// The actual characters can be configured in the configuration file.
    #[arg(
        long,
        env = "HL_ASCII",
        value_name = "WHEN",
        value_enum,
        default_value_t = AsciiOption::from(config::global::get().ascii),
        default_missing_value = "always",
        num_args = 0..=1,
        overrides_with = "ascii",
        help_heading = heading::OUTPUT
    )]
    pub ascii: AsciiOption,

    /// Whether to expand fields and messages
    ///
    /// Controls how large field values and messages are formatted.
    /// Higher expansion levels will break up long content into multiple lines.
    #[arg(
        long,
        short = 'x',
        env = "HL_EXPANSION",
        value_name = "MODE",
        value_enum,
        default_value_t = ExpansionOption::from(config::global::get().formatting.expansion.mode),
        default_missing_value = "always",
        num_args = 0..=1,
        overrides_with = "expansion",
        help_heading = heading::OUTPUT
    )]
    pub expansion: ExpansionOption,

    /// Output file
    #[arg(long, short = 'o', overrides_with = "output", value_name = "FILE", help_heading = heading::OUTPUT)]
    pub output: Option<String>,

    /// Output entry delimiter
    ///
    /// Possible values:
    /// • <c>newline</>: Either lf or crlf, depends on the platform (default)
    /// • <c>nul</>: Null character (\0)
    #[arg(long, env = "HL_OUTPUT_DELIMITER", default_value = "newline", overrides_with = "output_delimiter", help_heading = heading::OUTPUT)]
    pub output_delimiter: OutputDelimiter,

    /// Input format
    #[arg(
        long,
        env = "HL_INPUT_FORMAT",
        default_value = "auto",
        overrides_with = "input_format",
        value_name = "FORMAT",
        help_heading = heading::INPUT
    )]
    pub input_format: InputFormat,

    /// Unix timestamp unit
    #[arg(
        long,
        default_value = "auto",
        overrides_with = "unix_timestamp_unit",
        env = "HL_UNIX_TIMESTAMP_UNIT",
        value_name = "UNIT",
        help_heading = heading::INPUT
    )]
    pub unix_timestamp_unit: UnixTimestampUnit,

    /// Allow non-JSON prefixes before JSON log entries
    #[arg(long, env = "HL_ALLOW_PREFIX", overrides_with = "allow_prefix", help_heading = heading::INPUT)]
    pub allow_prefix: bool,

    /// Log entry delimiter
    ///
    /// By default, the delimiter is considered to be LF or CRLF followed by an empty line or a line that
    /// starts with a character other than '}', a space, or a tab.
    /// This works well for most JSON and logfmt logs, including series of pretty-printed JSON objects.
    ///
    /// Possible values:
    /// • <c>auto</>: Auto-select delimiter based on input format (default)
    /// • <c>cr</>: Carriage return (\r)
    /// • <c>lf</>: Line feed (\n)
    /// • <c>crlf</>: Carriage return followed by line feed (\r\n)
    /// • <c>newline</>: Either lf or crlf, whichever comes first
    /// • <c>nul</>: Null character (\0)
    #[arg(long, env = "HL_DELIMITER", default_value = "auto", overrides_with = "delimiter", help_heading = heading::INPUT)]
    pub delimiter: Delimiter,

    /// Number of interrupts to ignore, i.e. Ctrl-C (SIGINT)
    #[arg(
        long,
        default_value = "3",
        env = "HL_INTERRUPT_IGNORE_COUNT",
        overrides_with = "interrupt_ignore_count",
        value_name = "N",
        help_heading = heading::ADVANCED
    )]
    pub interrupt_ignore_count: usize,

    /// Buffer size
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

    /// Maximum log entry size
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

    /// Number of processing threads
    #[arg(
        long,
        short = 'C',
        env = "HL_CONCURRENCY",
        overrides_with = "concurrency",
        value_name = "N",
        help_heading = heading::ADVANCED
    )]
    pub concurrency: Option<usize>,

    /// Print shell auto-completion script and exit
    #[arg(
        long,
        value_parser = value_parser!(Shell),
        value_name = "SHELL",
        help_heading = heading::ADVANCED,
    )]
    pub shell_completions: Option<Shell>,

    /// Print man page and exit
    #[arg(long, help_heading = heading::ADVANCED)]
    pub man_page: bool,

    /// Print available themes optionally filtered by tags
    #[arg(
        long,
        num_args=0..=1,
        value_name = "TAGS",
        require_equals = true,
        value_parser = ThemeTagSet::clap_parser(),
        help_heading = heading::ADVANCED)
    ]
    pub list_themes: Option<Option<ThemeTagSet>>,

    /// Print debug index metadata (in <c>--sort</> mode) and exit
    #[arg(long, requires = "sort", help_heading = heading::ADVANCED)]
    pub dump_index: bool,

    /// Print help
    #[arg(
        long,
        num_args=0..=1,
        value_name = "VERBOSITY",
        default_missing_value = "short",
        require_equals = true,
    )]
    pub help: Option<HelpVerbosity>,

    /// Files to process
    #[arg(name = "FILE")]
    pub files: Vec<PathBuf>,
}

pub type ColorOption = clap::ColorChoice;

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingOption {
    Auto,
    Always,
    Never,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Auto,
    Json,
    Logfmt,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
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

impl From<FlattenOption> for settings::FlattenOption {
    fn from(value: FlattenOption) -> settings::FlattenOption {
        match value {
            FlattenOption::Never => settings::FlattenOption::Never,
            FlattenOption::Always => settings::FlattenOption::Always,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpVerbosity {
    Short,
    Long,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    Auto,
    Cr,
    Lf,
    Crlf,
    Newline,
    Nul,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiOption {
    Auto,
    Never,
    Always,
}

impl From<AsciiModeOpt> for AsciiOption {
    fn from(value: AsciiModeOpt) -> Self {
        match value {
            AsciiModeOpt::Auto => Self::Auto,
            AsciiModeOpt::Never => Self::Never,
            AsciiModeOpt::Always => Self::Always,
        }
    }
}

impl From<AsciiOption> for AsciiModeOpt {
    fn from(value: AsciiOption) -> Self {
        match value {
            AsciiOption::Auto => Self::Auto,
            AsciiOption::Never => Self::Never,
            AsciiOption::Always => Self::Always,
        }
    }
}

pub type InputInfoSet = EnumSet<InputInfo>;
pub type ThemeTag = themecfg::Tag;
pub type ThemeTagSet = EnumSet<ThemeTag>;

mod heading {
    pub const FILTERING: &str = "Filtering Options";
    pub const INPUT: &str = "Input Options";
    pub const OUTPUT: &str = "Output Options";
    pub const ADVANCED: &str = "Advanced Options";
}

// ---

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ExpansionOption {
    Never,
    Inline,
    #[default]
    Auto,
    Always,
}

impl From<ExpansionMode> for ExpansionOption {
    fn from(value: ExpansionMode) -> Self {
        match value {
            ExpansionMode::Never => Self::Never,
            ExpansionMode::Inline => Self::Inline,
            ExpansionMode::Auto => Self::Auto,
            ExpansionMode::Always => Self::Always,
        }
    }
}

impl From<Option<ExpansionMode>> for ExpansionOption {
    fn from(value: Option<ExpansionMode>) -> Self {
        Self::from(value.unwrap_or_default())
    }
}

impl From<ExpansionOption> for ExpansionMode {
    fn from(value: ExpansionOption) -> Self {
        match value {
            ExpansionOption::Never => ExpansionMode::Never,
            ExpansionOption::Inline => ExpansionMode::Inline,
            ExpansionOption::Auto => ExpansionMode::Auto,
            ExpansionOption::Always => ExpansionMode::Always,
        }
    }
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
mod tests;
