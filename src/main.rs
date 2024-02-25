// std imports
use std::convert::TryFrom;
use std::default::Default;
use std::io::{stdin, stdout, IsTerminal};
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::Duration;

// third-party imports
use chrono::Utc;
use clap::{ArgAction, CommandFactory, Parser, ValueEnum};
use itertools::Itertools;
use nu_ansi_term::Color;
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;
use std::num::NonZeroUsize;

// local imports
use hl::datefmt::LinuxDateFormat;
use hl::error::*;
use hl::input::InputReference;
use hl::level::{LevelValueParser, RelaxedLevel};
use hl::output::{OutputStream, Pager};
use hl::query::Query;
use hl::settings::Settings;
use hl::signal::SignalHandler;
use hl::theme::{Theme, ThemeOrigin};
use hl::timeparse::parse_time;
use hl::timezone::Tz;
use hl::{IncludeExcludeKeyFilter, KeyMatchOptions};

// ---

const APP_NAME: &str = "hl";

// ---

/// JSON log converter to human readable representation.
#[derive(Parser)]
#[clap(version, disable_help_flag = true)]
struct Opt {
    /// Color output options.
    #[arg(long, default_value = "auto", env = "HL_COLOR", overrides_with = "color")]
    #[arg(value_enum)]
    color: ColorOption,
    //
    /// Handful alias for --color=always, overrides --color option.
    #[arg(short)]
    color_always: bool,
    //
    /// Output paging options.
    #[arg(long, default_value = "auto", env = "HL_PAGING", overrides_with = "paging")]
    #[arg(value_enum)]
    paging: PagingOption,
    //
    /// Handful alias for --paging=never, overrides --paging option.
    #[arg(short = 'P')]
    paging_never: bool,
    //
    //
    /// Color theme.
    #[arg(
        long,
        default_value_t = CONFIG.theme.clone(),
        env = "HL_THEME",
        overrides_with="theme",
    )]
    theme: String,
    //
    /// Output raw JSON messages instead of formatter messages, it can be useful for applying filters and saving results in original format.
    #[arg(short, long, overrides_with = "raw")]
    raw: bool,
    //
    /// Disable raw JSON messages output, overrides --raw option.
    #[arg(long, overrides_with = "raw")]
    _no_raw: bool,
    //
    /// Disable unescaping and prettifying of field values.
    #[arg(long, overrides_with = "raw_fields")]
    raw_fields: bool,
    //
    /// Allow non-JSON prefixes before JSON messages.
    #[arg(long, env = "HL_ALLOW_PREFIX", overrides_with = "allow_prefix")]
    allow_prefix: bool,
    //
    /// Number of interrupts to ignore, i.e. Ctrl-C (SIGINT).
    #[arg(
        long,
        default_value = "3",
        env = "HL_INTERRUPT_IGNORE_COUNT",
        overrides_with = "interrupt_ignore_count"
    )]
    interrupt_ignore_count: usize,
    //
    /// Buffer size.
    #[arg(long, default_value = "256 KiB", env="HL_BUFFER_SIZE",  value_parser = parse_non_zero_size, overrides_with="buffer_size")]
    buffer_size: NonZeroUsize,
    //
    /// Maximum message size.
    #[arg(long, default_value = "64 MiB", env="HL_MAX_MESSAGE_SIZE",  value_parser = parse_non_zero_size, overrides_with="max_message_size")]
    max_message_size: NonZeroUsize,
    //
    /// Number of processing threads.
    #[arg(long, short = 'C', env = "HL_CONCURRENCY", overrides_with = "concurrency")]
    concurrency: Option<usize>,
    //
    /// Filtering by field values in one of forms [k=v, k~=v, k~~=v, 'k!=v', 'k!~=v', 'k!~~=v'] where ~ does substring match and ~~ does regular expression match.
    #[arg(short, long, number_of_values = 1)]
    filter: Vec<String>,
    //
    /// Custom query, accepts expressions from --filter and supports '(', ')', 'and', 'or', 'not', 'in', 'contain', 'like', '<', '>', '<=', '>=', etc.
    #[arg(short, long, number_of_values = 1)]
    query: Vec<String>,
    //
    /// Hide or unhide fields with the specified keys, prefix with ! to unhide, specify !* to unhide all.
    #[arg(long, short = 'h', number_of_values = 1)]
    hide: Vec<String>,
    //
    /// Filtering by level.
    #[arg(short, long, env = "HL_LEVEL", overrides_with="level", ignore_case=true, value_parser = LevelValueParser)]
    #[arg(value_enum)]
    level: Option<RelaxedLevel>,
    //
    /// Filtering by timestamp >= the value (--time-zone and --local options are honored).
    #[arg(long, allow_hyphen_values = true, overrides_with = "since")]
    since: Option<String>,
    //
    /// Filtering by timestamp <= the value (--time-zone and --local options are honored).
    #[arg(long, allow_hyphen_values = true, overrides_with = "until")]
    until: Option<String>,
    //
    /// Time format, see https://man7.org/linux/man-pages/man1/date.1.html.
    #[arg(
        short,
        long,
        env="HL_TIME_FORMAT",
        default_value_t = CONFIG.time_format.clone(),
        overrides_with = "time_format",
    )]
    time_format: String,
    //
    /// Time zone name, see column "TZ identifier" at https://en.wikipedia.org/wiki/List_of_tz_database_time_zones.
    #[arg(long, short = 'Z', env="HL_TIME_ZONE", default_value = &CONFIG.time_zone.name(), overrides_with="time_zone")]
    time_zone: chrono_tz::Tz,
    //
    /// Use local time zone, overrides --time-zone option.
    #[arg(long, short = 'L', overrides_with = "local")]
    local: bool,
    //
    /// Disable local time zone, overrides --local option.
    #[arg(long, overrides_with = "local")]
    _no_local: bool,
    //
    /// Files to process
    #[arg(name = "FILE")]
    files: Vec<PathBuf>,
    //
    /// Hide empty fields, applies for null, string, object and array fields only.
    #[arg(
        long,
        short = 'e',
        env = "HL_HIDE_EMPTY_FIELDS",
        overrides_with = "hide_empty_fields"
    )]
    hide_empty_fields: bool,
    //
    /// Show empty fields, overrides --hide-empty-fields option.
    #[arg(
        long,
        short = 'E',
        env = "HL_SHOW_EMPTY_FIELDS",
        overrides_with = "show_empty_fields"
    )]
    show_empty_fields: bool,

    /// Show input number and/or input filename before each message.
    #[arg(long, default_value = "auto", overrides_with = "input_info")]
    #[arg(value_enum)]
    input_info: InputInfoOption,
    //
    /// List available themes and exit.
    #[arg(long)]
    list_themes: bool,

    /// Sort messages chronologically.
    #[arg(long, short = 's', overrides_with = "sort")]
    sort: bool,

    /// Follow input streams and sort messages chronologically during time frame set by --sync-interval-ms option.
    #[arg(long, short = 'F', overrides_with = "follow")]
    follow: bool,

    /// Number of last messages to preload from each file in --follow mode.
    #[arg(long, default_value = "10", overrides_with = "tail")]
    tail: u64,

    /// Synchronization interval for live streaming mode enabled by --follow option.
    #[arg(long, default_value = "100", overrides_with = "sync_interval_ms")]
    sync_interval_ms: u64,

    /// Output file.
    #[arg(long, short = 'o', overrides_with = "output")]
    output: Option<String>,

    /// Dump index metadata and exit.
    #[arg(long)]
    dump_index: bool,

    //
    /// Print help.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    help: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum ColorOption {
    Auto,
    Always,
    Never,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum PagingOption {
    Auto,
    Always,
    Never,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum InputInfoOption {
    Auto,
    None,
    Full,
    Compact,
    Minimal,
}

// ---

static CONFIG: Lazy<Settings> = Lazy::new(|| load_config());

// ---

fn app_dirs() -> AppDirs {
    AppDirs::new(Some(APP_NAME), true).unwrap()
}

fn load_config() -> Settings {
    Settings::load(&app_dirs()).unwrap()
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

// ---

fn run() -> Result<()> {
    let app_dirs = app_dirs();
    let settings = Settings::load(&app_dirs)?;
    let opt = Opt::parse();
    if opt.help {
        return Opt::command().print_help().map_err(Error::Io);
    }

    let color_supported = if stdout().is_terminal() {
        if let Err(err) = hl::enable_ansi_support() {
            eprintln!("failed to enable ansi support: {}", err);
            false
        } else {
            true
        }
    } else {
        false
    };

    // Configure color scheme.
    let color = if opt.color_always {
        ColorOption::Always
    } else {
        opt.color
    };
    let use_colors = match color {
        ColorOption::Auto => stdout().is_terminal() && color_supported,
        ColorOption::Always => true,
        ColorOption::Never => false,
    };
    let theme = if use_colors {
        let theme = &opt.theme;
        Theme::load(&app_dirs, theme)?
    } else {
        Theme::none()
    };

    if opt.list_themes {
        let themes = Theme::list(&app_dirs)?
            .into_iter()
            .sorted_by_key(|(name, info)| (info.origin, name.clone()));
        for (origin, group) in themes.group_by(|(_, info)| info.origin).into_iter() {
            let origin = match origin {
                ThemeOrigin::Stock => "stock",
                ThemeOrigin::Custom => "custom",
            };
            println!("{}:", origin);
            for (name, _) in group {
                println!("  {}", name);
            }
        }
        return Ok(());
    }

    // Configure concurrency.
    let concurrency = match opt.concurrency.or(settings.concurrency) {
        None | Some(0) => num_cpus::get(),
        Some(value) => value,
    };
    // Configure timezone.
    let tz = if opt.local { Tz::Local } else { Tz::IANA(opt.time_zone) };
    // Configure time format.
    let time_format = LinuxDateFormat::new(&opt.time_format).compile();
    // Configure filter.
    let filter = hl::Filter {
        fields: hl::FieldFilterSet::new(opt.filter)?,
        level: opt.level.map(|x| x.into()),
        since: if let Some(v) = &opt.since {
            Some(parse_time(v, &tz, &time_format)?.with_timezone(&Utc))
        } else {
            None
        },
        until: if let Some(v) = &opt.until {
            Some(parse_time(v, &tz, &time_format)?.with_timezone(&Utc))
        } else {
            None
        },
    };
    // Configure hide_empty_fields
    let hide_empty_fields = !opt.show_empty_fields && opt.hide_empty_fields;

    // Configure field filter.
    let all = || IncludeExcludeKeyFilter::new(KeyMatchOptions::default());
    let none = || all().excluded();
    let mut fields = all();
    for (i, key) in CONFIG.fields.hide.iter().chain(&opt.hide).enumerate() {
        if key == "*" {
            fields = none();
        } else if key == "!*" {
            fields = all();
        } else if key.starts_with("!") {
            if i == 0 {
                fields = none();
            }
            fields.entry(&key[1..]).include();
        } else if key.starts_with("\\!") {
            fields.entry(&key[1..]).exclude();
        } else if key.starts_with("\\\\") {
            fields.entry(&key[1..]).exclude();
        } else {
            fields.entry(&key).exclude();
        }
    }

    let max_message_size = opt.max_message_size;
    let buffer_size = std::cmp::min(max_message_size, opt.buffer_size);

    let mut query: Option<Query> = None;
    for q in opt.query {
        let right = Query::parse(&q)?;
        if let Some(left) = query {
            query = Some(left.and(right));
        } else {
            query = Some(right);
        }
    }

    // Create app.
    let app = hl::App::new(hl::Options {
        theme: Arc::new(theme),
        raw: opt.raw,
        raw_fields: opt.raw_fields,
        allow_prefix: opt.allow_prefix,
        time_format,
        buffer_size,
        max_message_size,
        concurrency,
        filter,
        query,
        fields: hl::FieldOptions {
            settings: settings.fields,
            filter: Arc::new(fields),
        },
        formatting: settings.formatting,
        time_zone: tz,
        hide_empty_fields,
        sort: opt.sort,
        follow: opt.follow,
        sync_interval: Duration::from_millis(opt.sync_interval_ms),
        input_info: match opt.input_info {
            InputInfoOption::Auto => Some(hl::app::InputInfo::Auto),
            InputInfoOption::None => None,
            InputInfoOption::Full => Some(hl::app::InputInfo::Full),
            InputInfoOption::Compact => Some(hl::app::InputInfo::Compact),
            InputInfoOption::Minimal => Some(hl::app::InputInfo::Minimal),
        },
        dump_index: opt.dump_index,
        app_dirs: Some(app_dirs),
        tail: opt.tail,
    });

    // Configure input.
    let mut inputs = opt
        .files
        .iter()
        .map(|x| {
            if x.to_str() == Some("-") {
                InputReference::Stdin
            } else {
                InputReference::File(x.clone())
            }
        })
        .collect::<Vec<_>>();
    if inputs.len() == 0 {
        if stdin().is_terminal() {
            let mut cmd = Opt::command();
            return cmd.print_help().map_err(Error::Io);
        }
        inputs.push(InputReference::Stdin);
    }

    if opt.sort {
        for input in &inputs {
            if let InputReference::File(path) = input {
                if let Some(Some("gz")) = path.extension().map(|x| x.to_str()) {
                    return Err(Error::UnsupportedFormatForIndexing {
                        path: path.clone(),
                        format: "gzip".into(),
                    });
                }
            }
        }
    }

    let inputs = inputs
        .into_iter()
        .map(|input| input.hold().map_err(Error::Io))
        .collect::<Result<Vec<_>>>()?;

    let paging = match opt.paging {
        PagingOption::Auto => {
            if stdout().is_terminal() {
                true
            } else {
                false
            }
        }
        PagingOption::Always => true,
        PagingOption::Never => false,
    };
    let paging = if opt.paging_never || opt.follow { false } else { paging };
    let mut output: OutputStream = match opt.output {
        Some(output) => Box::new(std::fs::File::create(PathBuf::from(&output))?),
        None => {
            if paging {
                if let Ok(pager) = Pager::new() {
                    Box::new(pager)
                } else {
                    Box::new(stdout())
                }
            } else {
                Box::new(stdout())
            }
        }
    };

    // Run the app.
    let run = || match app.run(inputs, output.as_mut()) {
        Ok(()) => Ok(()),
        Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(err),
    };

    let interrupt_ignore_count = if opt.follow { 0 } else { opt.interrupt_ignore_count };

    // Run the app with signal handling.
    SignalHandler::run(interrupt_ignore_count, std::time::Duration::from_secs(1), run)
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}: {}", Color::Red.paint("error"), err);
        process::exit(1);
    }
}
