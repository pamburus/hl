// std imports
use std::{
    default::Default,
    io::{stdin, stdout, IsTerminal},
    path::PathBuf,
    process,
    sync::Arc,
    time::Duration,
};

// third-party imports
use chrono::Utc;
use clap::{CommandFactory, Parser};
use env_logger::{self as logger};
use itertools::Itertools;

// local imports
use hl::{
    app, cli, config,
    datefmt::LinuxDateFormat,
    error::*,
    input::InputReference,
    output::{OutputStream, Pager},
    query::Query,
    settings::Settings,
    signal::SignalHandler,
    theme::{Theme, ThemeOrigin},
    timeparse::parse_time,
    timezone::Tz,
    Delimiter, {IncludeExcludeKeyFilter, KeyMatchOptions},
};

const HL_DEBUG_LOG: &str = "HL_DEBUG_LOG";

// ---

fn bootstrap() -> Result<Settings> {
    if std::env::var(HL_DEBUG_LOG).is_ok() {
        logger::Builder::from_env(HL_DEBUG_LOG).format_timestamp_micros().init();
        log::debug!("logging initialized");
    } else {
        logger::Builder::new()
            .filter_level(log::LevelFilter::Warn)
            .format_timestamp_millis()
            .init()
    }

    let opt = cli::BootstrapOpt::parse().args;

    let (offset, no_default_configs) = opt
        .config
        .iter()
        .rposition(|x| x == "" || x == "-")
        .map(|x| (x + 1, true))
        .unwrap_or_default();
    let configs = &opt.config[offset..];

    let settings = config::at(configs).no_default(no_default_configs).load()?;
    config::global::initialize(settings.clone());

    Ok(settings)
}

fn run() -> Result<()> {
    let settings = bootstrap()?;

    let opt = cli::Opt::parse_from(wild::args());
    if opt.help {
        return cli::Opt::command().print_help().map_err(Error::Io);
    }

    if let Some(shell) = opt.shell_completions {
        let mut cmd = cli::Opt::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut stdout());
        return Ok(());
    }

    if opt.man_page {
        let man = clap_mangen::Man::new(cli::Opt::command());
        man.render(&mut stdout())?;
        return Ok(());
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
        cli::ColorOption::Always
    } else {
        opt.color
    };
    let use_colors = match color {
        cli::ColorOption::Auto => stdout().is_terminal() && color_supported,
        cli::ColorOption::Always => true,
        cli::ColorOption::Never => false,
    };

    let app_dirs = config::app_dirs().ok_or(Error::AppDirs)?;
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
        for (origin, group) in themes.chunk_by(|(_, info)| info.origin).into_iter() {
            let origin = match origin {
                ThemeOrigin::Stock => "stock",
                ThemeOrigin::Custom => "custom",
            };
            if stdout().is_terminal() {
                println!("{}:", origin);
            }
            for (name, _) in group {
                if stdout().is_terminal() {
                    println!("  - {}", name);
                } else {
                    println!("{}", name);
                }
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
        fields: hl::FieldFilterSet::new(&opt.filter)?,
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
    for (i, key) in settings.fields.hide.iter().chain(&opt.hide).enumerate() {
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
    for q in &opt.query {
        let right = Query::parse(&q)?;
        if let Some(left) = query {
            query = Some(left.and(right));
        } else {
            query = Some(right);
        }
    }

    let mut delimiter = Delimiter::default();
    if let Some(d) = opt.delimiter {
        delimiter = match d.to_lowercase().as_str() {
            "nul" => Delimiter::Byte(0),
            "lf" => Delimiter::Byte(b'\n'),
            "cr" => Delimiter::Byte(b'\r'),
            "crlf" => Delimiter::default(),
            _ => {
                if d.len() == 1 {
                    Delimiter::Byte(d.as_bytes()[0])
                } else if d.len() > 1 {
                    Delimiter::Str(d)
                } else {
                    Delimiter::default()
                }
            }
        };
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
        filter: app::AdvancedFilter::new(filter, query).into(),
        fields: hl::FieldOptions {
            settings: settings.fields.clone(),
            filter: Arc::new(fields),
        },
        formatting: settings.formatting.clone(),
        time_zone: tz,
        hide_empty_fields,
        sort: opt.sort,
        follow: opt.follow,
        sync_interval: Duration::from_millis(opt.sync_interval_ms),
        input_info: match opt.input_info {
            cli::InputInfoOption::Auto => Some(app::InputInfo::Auto),
            cli::InputInfoOption::None => None,
            cli::InputInfoOption::Full => Some(app::InputInfo::Full),
            cli::InputInfoOption::Compact => Some(app::InputInfo::Compact),
            cli::InputInfoOption::Minimal => Some(app::InputInfo::Minimal),
        },
        input_format: match opt.input_format {
            cli::InputFormat::Auto => None,
            cli::InputFormat::Json => Some(app::InputFormat::Json),
            cli::InputFormat::Logfmt => Some(app::InputFormat::Logfmt),
        },
        dump_index: opt.dump_index,
        app_dirs: Some(app_dirs),
        tail: opt.tail,
        delimiter,
        unix_ts_unit: match opt.unix_timestamp_unit {
            cli::UnixTimestampUnit::Auto => None,
            cli::UnixTimestampUnit::S => Some(app::UnixTimestampUnit::Seconds),
            cli::UnixTimestampUnit::Ms => Some(app::UnixTimestampUnit::Milliseconds),
            cli::UnixTimestampUnit::Us => Some(app::UnixTimestampUnit::Microseconds),
            cli::UnixTimestampUnit::Ns => Some(app::UnixTimestampUnit::Nanoseconds),
        },
        flatten: opt.flatten != cli::FlattenOption::Never,
    });

    // Configure the input.
    let mut inputs = opt
        .files
        .iter()
        .map(|x| {
            if x.to_str() == Some("-") {
                Ok::<_, std::io::Error>(InputReference::Stdin)
            } else {
                Ok(InputReference::File(x.clone().try_into()?))
            }
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if inputs.len() == 0 {
        if stdin().is_terminal() {
            let mut cmd = cli::Opt::command();
            return cmd.print_help().map_err(Error::Io);
        }
        inputs.push(InputReference::Stdin);
    }

    let n = inputs.len();
    log::debug!("hold {n} inputs");
    let inputs = inputs
        .into_iter()
        .map(|input| input.hold().map_err(Error::Io))
        .collect::<Result<Vec<_>>>()?;

    let paging = match opt.paging {
        cli::PagingOption::Auto => {
            if stdout().is_terminal() {
                true
            } else {
                false
            }
        }
        cli::PagingOption::Always => true,
        cli::PagingOption::Never => false,
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

    log::debug!("run the app");

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
        err.log();
        process::exit(1);
    }
}
