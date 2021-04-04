// std imports
use std::convert::TryFrom;
use std::env;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::Duration;

// third-party imports
use ansi_term::Colour;
use async_std::{
    io::stdin,
    stream::{self, Stream},
    task,
};
use chrono::{FixedOffset, Local, TimeZone};
use chrono_tz::{Tz, UTC};
use structopt::clap::arg_enum;
use structopt::StructOpt;

// local imports
use hl::datefmt::LinuxDateFormat;
use hl::error::*;
use hl::input::{open, ConcatReader, Input, InputStream};
use hl::output::{OutputStream, Pager};
use hl::signal::SignalHandler;
use hl::theme::Theme;
use hl::Level;
use hl::{IncludeExcludeKeyFilter, KeyMatchOptions};

// ---

/// JSON log converter to human readable representation.
#[derive(StructOpt)]
#[structopt()]
struct Opt {
    /// Color output options, one of { auto, always, never }.
    #[structopt(long, default_value = "auto", overrides_with = "color")]
    color: ColorOption,
    //
    /// Handful alias for --color=always, overrides --color option.
    #[structopt(short)]
    color_always: bool,
    //
    /// Output paging options, one of { auto, always, never }.
    #[structopt(long, default_value = "auto", overrides_with = "paging")]
    paging: PagingOption,
    //
    /// Handful alias for --paging=never, overrides --paging option.
    #[structopt(short = "P")]
    paging_never: bool,
    //
    //
    /// Color theme, one of { auto, dark, dark24, light }.
    #[structopt(long, default_value = "dark", overrides_with = "theme")]
    theme: ThemeOption,
    //
    /// Disable unescaping and prettifying of field values.
    #[structopt(short, long)]
    raw_fields: bool,
    //
    /// Number of interrupts to ignore, i.e. Ctrl-C (SIGINT).
    #[structopt(long, default_value = "3", overrides_with = "interrupt-ignore-count")]
    interrupt_ignore_count: usize,
    //
    /// Buffer size.
    #[structopt(long, default_value = "2 MiB", overrides_with = "buffer-size", parse(try_from_str = parse_non_zero_size))]
    buffer_size: usize,
    //
    /// Maximum message size.
    #[structopt(long, default_value = "64 MiB", overrides_with = "max-message-size", parse(try_from_str = parse_non_zero_size))]
    max_message_size: usize,
    //
    /// Number of processing threads.
    #[structopt(long, short = "C", overrides_with = "concurrency")]
    concurrency: Option<usize>,
    //
    /// Filtering by field values in one of forms <key>=<value>, <key>~=<value>, <key>!=<value>, <key>!~=<value>.
    #[structopt(short, long, number_of_values = 1)]
    filter: Vec<String>,
    //
    /// Hide fields with the specified keys.
    #[structopt(long, short = "h", number_of_values = 1)]
    hide: Vec<String>,
    //
    /// Hide all fields except fields with the specified keys.
    #[structopt(long, short = "H", number_of_values = 1)]
    show: Vec<String>,
    //
    /// Filtering by level, valid values: ['d', 'i', 'w', 'e'].
    #[structopt(short, long, default_value = "d", overrides_with = "level")]
    level: Level,
    //
    /// Time format, see https://man7.org/linux/man-pages/man1/date.1.html.
    #[structopt(
        short,
        long,
        default_value = "%b %d %T.%3N",
        overrides_with = "time-format"
    )]
    time_format: String,
    //
    /// Time zone name, see column "TZ database name" at https://en.wikipedia.org/wiki/List_of_tz_database_time_zones.
    #[structopt(long, short = "Z", default_value = "UTC", overrides_with = "time-zone")]
    time_zone: Tz,
    //
    /// Use local time zone, overrides --time-zone option.
    #[structopt(long, short = "L")]
    local: bool,
    //
    /// Files to process
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
    //
    /// Hide empty fields, applies for null, string, object and array fields only.
    #[structopt(long, short = "e")]
    hide_empty_fields: bool,
    //
    /// Show empty fields, overrides --hide-empty-fields option.
    #[structopt(long, short = "E")]
    show_empty_fields: bool,
}

arg_enum! {
    #[derive(Debug)]
    enum ColorOption {
        Auto,
        Always,
        Never,
    }
}

arg_enum! {
    #[derive(Debug)]
    enum ThemeOption {
        Auto,
        Dark,
        Dark24,
        Light,
    }
}

arg_enum! {
    #[derive(Debug)]
    enum PagingOption {
        Auto,
        Always,
        Never,
    }
}

fn parse_size(s: &str) -> Result<usize> {
    match bytefmt::parse(s) {
        Ok(value) => Ok(usize::try_from(value)?),
        Err(_) => {
            if let Ok(value) = bytefmt::parse(s.to_owned() + "ib") {
                return Ok(usize::try_from(value)?);
            }
            Err(Error::InvalidSize(s.into()))
        }
    }
}

fn parse_non_zero_size(s: &str) -> Result<usize> {
    let value = parse_size(s)?;
    if value == 0 {
        Err(Error::ZeroSize)
    } else {
        Ok(value)
    }
}

async fn run() -> Result<()> {
    let opt = Opt::from_args();
    let stdout_is_atty = || atty::is(atty::Stream::Stdout);

    // Configure color scheme.
    let color = if opt.color_always {
        ColorOption::Always
    } else {
        opt.color
    };
    let truecolor = env::var("COLORTERM").unwrap_or_default() == "truecolor";
    let theme = |theme: ThemeOption| match (theme, truecolor) {
        (ThemeOption::Auto, false) | (ThemeOption::Dark, _) => Theme::dark(),
        (ThemeOption::Auto, true) | (ThemeOption::Dark24, _) => Theme::dark24(),
        (ThemeOption::Light, _) => Theme::light(),
    };
    let theme = match color {
        ColorOption::Auto => match stdout_is_atty() {
            true => theme(opt.theme),
            false => Theme::none(),
        },
        ColorOption::Always => theme(opt.theme),
        ColorOption::Never => Theme::none(),
    };

    // Configure concurrency.
    let concurrency = match opt.concurrency {
        None | Some(0) => num_cpus::get(),
        Some(value) => value,
    };
    // Configure filter.
    let filter = hl::Filter {
        fields: hl::FieldFilterSet::new(opt.filter),
        level: Some(opt.level),
    };
    // Configure hide_empty_fields
    let hide_empty_fields = !opt.show_empty_fields && opt.hide_empty_fields;

    // Configure field filter.
    let mut fields = IncludeExcludeKeyFilter::new(KeyMatchOptions::default());
    if opt.hide.len() == 0 && opt.show.len() != 0 {
        fields.exclude();
    }
    for key in opt.hide {
        fields.entry(&key).exclude();
    }
    for key in opt.show {
        fields.entry(&key).include();
    }

    let max_message_size = opt.max_message_size;
    let buffer_size = std::cmp::min(max_message_size, opt.buffer_size);

    // Create app.
    let app = hl::App::new(hl::Options {
        theme: Arc::new(theme),
        raw_fields: opt.raw_fields,
        time_format: LinuxDateFormat::new(&opt.time_format).compile(),
        buffer_size,
        max_message_size,
        concurrency,
        filter,
        fields: Arc::new(fields),
        time_zone: if opt.local {
            *Local.timestamp(0, 0).offset()
        } else {
            let tz = opt.time_zone;
            let offset = UTC.ymd(1970, 1, 1).and_hms(0, 0, 0) - tz.ymd(1970, 1, 1).and_hms(0, 0, 0);
            FixedOffset::east(offset.num_seconds() as i32)
        },
        hide_empty_fields,
    });

    // Configure input.
    let inputs = opt
        .files
        .iter()
        .map(|x| {
            if x.to_str() == Some("-") {
                Ok(Input::new("<stdin>".into(), Box::new(stdin())))
            } else {
                task::block_on(open(&x))
            }
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    let mut input: InputStream = if inputs.len() == 0 {
        Box::new(stdin())
    } else {
        Box::new(ConcatReader::new(inputs.into_iter().map(|x| x.stream)))
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

    #[cfg(windows)]
    if stdout_is_atty() {
        let _ = ansi_term::enable_ansi_support();
    }

    // Run the app.
    let run = || match app.run(input.as_mut(), output.as_mut()) {
        Ok(()) => Ok(()),
        Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(err),
    };

    // Run the app with signal handling.
    SignalHandler::run(opt.interrupt_ignore_count, Duration::from_secs(1), run)
}

fn main() {
    if let Err(err) = task::block_on(run()) {
        eprintln!("{}: {}", Colour::Red.paint("error"), err);
        process::exit(1);
    }
}
