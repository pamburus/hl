// std imports
use std::convert::TryFrom;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

// third-party imports
use chrono::{FixedOffset, Local, TimeZone};
use chrono_tz::{Tz, UTC};
use clap::{ArgEnum, Parser};
use itertools::Itertools;
use nu_ansi_term::Color;
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;
use std::num::NonZeroUsize;

// local imports
use hl::datefmt::LinuxDateFormat;
use hl::error::*;
use hl::input::{open, ConcatReader, Input, InputStream};
use hl::output::{OutputStream, Pager};
use hl::settings::Settings;
use hl::signal::SignalHandler;
use hl::theme::{Theme, ThemeOrigin};
use hl::timeparse::parse_time;
use hl::Level;
use hl::{IncludeExcludeKeyFilter, KeyMatchOptions};

// ---

const APP_NAME: &str = "hl";

// ---

/// JSON log converter to human readable representation.
#[derive(Parser)]
#[clap(version)]
struct Opt {
    /// Color output options.
    #[clap(
        long,
        default_value = "auto",
        env = "HL_COLOR",
        overrides_with = "color"
    )]
    #[clap(arg_enum)]
    color: ColorOption,
    //
    /// Handful alias for --color=always, overrides --color option.
    #[clap(short)]
    color_always: bool,
    //
    /// Output paging options.
    #[clap(
        long,
        default_value = "auto",
        env = "HL_PAGING",
        overrides_with = "paging"
    )]
    #[clap(arg_enum)]
    paging: PagingOption,
    //
    /// Handful alias for --paging=never, overrides --paging option.
    #[clap(short = 'P')]
    paging_never: bool,
    //
    //
    /// Color theme.
    #[clap(
        long,
        default_value = &CONFIG.theme,
        env = "HL_THEME",
        overrides_with = "theme"
    )]
    theme: String,
    //
    /// Disable unescaping and prettifying of field values.
    #[clap(short, long)]
    raw_fields: bool,
    //
    /// Number of interrupts to ignore, i.e. Ctrl-C (SIGINT).
    #[clap(
        long,
        default_value = "3",
        env = "HL_INTERRUPT_IGNORE_COUNT",
        overrides_with = "interrupt-ignore-count"
    )]
    interrupt_ignore_count: usize,
    //
    /// Buffer size.
    #[clap(long, default_value = "2 MiB", env="HL_BUFFER_SIZE", overrides_with = "buffer-size", parse(try_from_str = parse_non_zero_size))]
    buffer_size: NonZeroUsize,
    //
    /// Maximum message size.
    #[clap(long, default_value = "64 MiB", env="HL_MAX_MESSAGE_SIZE", overrides_with = "max-message-size", parse(try_from_str = parse_non_zero_size))]
    max_message_size: NonZeroUsize,
    //
    /// Number of processing threads.
    #[clap(
        long,
        short = 'C',
        env = "HL_CONCURRENCY",
        overrides_with = "concurrency"
    )]
    concurrency: Option<usize>,
    //
    /// Filtering by field values in one of forms [<key>=<value>, <key>~=<value>, <key>~~=<value>, <key>!=<value>, <key>!~=<value>, <key>!~~=<value>] where ~ denotes substring match and ~~ denotes regular expression match.
    #[clap(short, long, number_of_values = 1)]
    filter: Vec<String>,
    //
    /// Hide or unhide fields with the specified keys, prefix with ! to unhide, specify !* to unhide all.
    #[clap(long, short = 'h', number_of_values = 1)]
    hide: Vec<String>,
    //
    /// Filtering by level.
    #[clap(short, long, env = "HL_LEVEL", overrides_with = "level")]
    #[clap(arg_enum)]
    level: Option<Level>,
    //
    /// Filtering by timestamp >= the value (--time-zone and --local options are honored).
    #[clap(long, allow_hyphen_values = true)]
    since: Option<String>,
    //
    /// Filtering by timestamp <= the value (--time-zone and --local options are honored).
    #[clap(long, allow_hyphen_values = true)]
    until: Option<String>,
    //
    /// Time format, see https://man7.org/linux/man-pages/man1/date.1.html.
    #[clap(
        short,
        long,
        env="HL_TIME_FORMAT",
        default_value = &CONFIG.time_format,
        overrides_with = "time-format"
    )]
    time_format: String,
    //
    /// Time zone name, see column "TZ database name" at https://en.wikipedia.org/wiki/List_of_tz_database_time_zones.
    #[clap(long, short = 'Z', env="HL_TIME_ZONE", default_value = &CONFIG.time_zone.name(), overrides_with = "time-zone")]
    time_zone: Tz,
    //
    /// Use local time zone, overrides --time-zone option.
    #[clap(long, short = 'L')]
    local: bool,
    //
    /// Files to process
    #[clap(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
    //
    /// Hide empty fields, applies for null, string, object and array fields only.
    #[clap(long, short = 'e', env = "HL_HIDE_EMPTY_FIELDS")]
    hide_empty_fields: bool,
    //
    /// Show empty fields, overrides --hide-empty-fields option.
    #[clap(long, short = 'E', env = "HL_SHOW_EMPTY_FIELDS")]
    show_empty_fields: bool,
    //
    /// List available themes and exit.
    #[clap(long)]
    list_themes: bool,
}

#[derive(ArgEnum, Debug, Clone, Copy)]
enum ColorOption {
    Auto,
    Always,
    Never,
}

#[derive(ArgEnum, Debug, Clone, Copy)]
enum PagingOption {
    Auto,
    Always,
    Never,
}

// ---

static CONFIG: Lazy<Settings> = Lazy::new(|| load_config());

// ---

fn load_config() -> Settings {
    let app_dirs = AppDirs::new(Some(APP_NAME), true).unwrap();
    Settings::load(&app_dirs).unwrap()
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
    let app_dirs = AppDirs::new(Some("hl"), true).unwrap();
    let settings = Settings::load(&app_dirs)?;
    let opt = Opt::parse();
    let stdout_is_atty = || atty::is(atty::Stream::Stdout);
    let color_supported = if stdout_is_atty() {
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
        ColorOption::Auto => stdout_is_atty() && color_supported,
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
    let tz = if opt.local {
        *Local.timestamp(0, 0).offset()
    } else {
        let tz = opt.time_zone;
        let offset = UTC.ymd(1970, 1, 1).and_hms(0, 0, 0) - tz.ymd(1970, 1, 1).and_hms(0, 0, 0);
        FixedOffset::east(offset.num_seconds() as i32)
    };
    // Configure time format.
    let time_format = LinuxDateFormat::new(&opt.time_format).compile();
    // Configure filter.
    let filter = hl::Filter {
        fields: hl::FieldFilterSet::new(opt.filter)?,
        level: opt.level,
        since: if let Some(v) = &opt.since {
            Some(parse_time(v, &tz, &time_format)?.into())
        } else {
            None
        },
        until: if let Some(v) = &opt.until {
            Some(parse_time(v, &tz, &time_format)?.into())
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

    // Create app.
    let app = hl::App::new(hl::Options {
        theme: Arc::new(theme),
        raw_fields: opt.raw_fields,
        time_format: time_format,
        buffer_size,
        max_message_size,
        concurrency,
        filter,
        fields: hl::FieldOptions {
            settings: settings.fields,
            filter: Arc::new(fields),
        },
        time_zone: tz,
        hide_empty_fields,
    });

    // Configure input.
    let inputs = opt
        .files
        .iter()
        .map(|x| {
            if x.to_str() == Some("-") {
                Ok(Input::new("<stdin>".into(), Box::new(std::io::stdin())))
            } else {
                open(&x)
            }
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    let mut input: InputStream = if inputs.len() == 0 {
        Box::new(std::io::stdin())
    } else {
        Box::new(ConcatReader::new(inputs.into_iter().map(|x| Ok(x))))
    };
    let paging = match opt.paging {
        PagingOption::Auto => {
            if stdout_is_atty() {
                true
            } else {
                false
            }
        }
        PagingOption::Always => true,
        PagingOption::Never => false,
    };
    let paging = if opt.paging_never { false } else { paging };
    let mut output: OutputStream = if paging {
        if let Ok(pager) = Pager::new() {
            Box::new(pager)
        } else {
            Box::new(std::io::stdout())
        }
    } else {
        Box::new(std::io::stdout())
    };

    // Run the app.
    let run = || match app.run(input.as_mut(), output.as_mut()) {
        Ok(()) => Ok(()),
        Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(err),
    };

    // Run the app with signal handling.
    SignalHandler::run(
        opt.interrupt_ignore_count,
        std::time::Duration::from_secs(1),
        run,
    )
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}: {}", Color::Red.paint("error"), err);
        process::exit(1);
    }
}
