// std imports
use std::{
    default::Default,
    io::{IsTerminal, Write, stdin, stdout},
    path::PathBuf,
    process,
    sync::Arc,
    time::Duration,
};

// third-party imports
use chrono::Utc;
use clap::{CommandFactory, Parser};
use enumset::enum_set;
use enumset_ext::EnumSetExt;
use env_logger::{self as logger};
use itertools::Itertools;
use terminal_size::terminal_size_of;
use utf8_supported::{Utf8Support, utf8_supported};

// local imports
use hl::{
    Delimiter, IncludeExcludeKeyFilter, KeyMatchOptions, app,
    appdirs::AppDirs,
    cli::{self, ThemeTagSet},
    config,
    datefmt::LinuxDateFormat,
    error::*,
    input::InputReference,
    output::{OutputStream, Pager},
    query::Query,
    settings::{AsciiModeOpt, InputInfo, Settings},
    signal::SignalHandler,
    theme::Theme,
    themecfg,
    timeparse::parse_time,
    timezone::Tz,
};

// private modules
mod help;

const HL_DEBUG_LOG: &str = "HL_DEBUG_LOG";
const HL_DEBUG_LOG_STYLE: &str = "HL_DEBUG_LOG_STYLE";

// ---

fn bootstrap() -> Result<Settings> {
    if std::env::var(HL_DEBUG_LOG).is_ok() {
        logger::Builder::from_env(logger::Env::new().filter(HL_DEBUG_LOG).write_style(HL_DEBUG_LOG_STYLE))
            .format_timestamp_micros()
            .init();
        log::debug!("logging initialized");
    } else {
        logger::Builder::new()
            .filter_level(log::LevelFilter::Error)
            .format_timestamp_millis()
            .init()
    }

    let opt = cli::BootstrapOpt::parse().args;

    let (offset, no_default_configs) = opt
        .config
        .iter()
        .rposition(|x| x.is_empty() || x == "-")
        .map(|x| (x + 1, true))
        .unwrap_or_default();
    let configs = &opt.config[offset..];

    let settings = config::at(configs).no_default(no_default_configs).load()?;
    config::global::initialize(settings.clone());

    Ok(settings)
}

fn run() -> Result<()> {
    // On Windows, force console to UTF-8 for the duration of the program.
    // This works around PowerShell resetting the output code page.
    #[cfg(windows)]
    let _utf8_guard = utf8_supported::set_console_utf8().ok();

    let settings = bootstrap()?;

    let opt = cli::Opt::parse_from(wild::args());

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

    let help_color = match use_colors {
        true => cli::ColorOption::Always,
        false => cli::ColorOption::Never,
    };

    let command = || cli::Opt::command().color(help_color);

    let paging = match opt.paging {
        cli::PagingOption::Auto => stdout().is_terminal(),
        cli::PagingOption::Always => true,
        cli::PagingOption::Never => false,
    };
    let paging = if opt.paging_never || opt.follow { false } else { paging };
    let pager = || -> OutputStream {
        if paging {
            if let Ok(pager) = Pager::new() {
                Box::new(pager)
            } else {
                Box::new(stdout())
            }
        } else {
            Box::new(stdout())
        }
    };

    if let Some(verbosity) = opt.help {
        let color_when = match use_colors {
            true => anstream::ColorChoice::Always,
            false => anstream::ColorChoice::Never,
        };
        let mut out = anstream::AutoStream::new(pager(), color_when);
        let help = match verbosity {
            cli::HelpVerbosity::Short => command().render_help(),
            cli::HelpVerbosity::Long => command().render_long_help(),
        };
        return write!(&mut out, "{}", help.ansi()).map_err(Error::Io);
    }

    if let Some(shell) = opt.shell_completions {
        let mut cmd = cli::Opt::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut stdout());
        return Ok(());
    }

    if opt.man_page {
        let man = clap_mangen::Man::new(cli::Opt::command()).verbatim(true);
        man.render(&mut stdout())?;
        return Ok(());
    }

    let app_dirs = config::app_dirs().ok_or(Error::AppDirs)?;

    if let Some(tags) = opt.list_themes {
        return list_themes(&app_dirs, tags);
    }

    let theme = if use_colors {
        let theme = &opt.theme;
        Theme::load_with_overlays(&app_dirs, theme, &settings.theme_overlays)?
    } else {
        Theme::none()
    };

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
        } else if let Some(stripped) = key.strip_prefix("!") {
            if i == 0 {
                fields = none();
            }
            fields.entry(stripped).include();
        } else if key.starts_with("\\!") || key.starts_with("\\\\") {
            fields.entry(&key[1..]).exclude();
        } else {
            fields.entry(key).exclude();
        }
    }

    let max_message_size = opt.max_message_size;
    let buffer_size = std::cmp::min(max_message_size, opt.buffer_size);

    let mut query: Option<Query> = None;
    for q in &opt.query {
        let right = Query::parse(q)?;
        if let Some(left) = query {
            query = Some(left.and(right));
        } else {
            query = Some(right);
        }
    }

    let delimiter = match opt.delimiter {
        cli::Delimiter::Nul => Delimiter::Byte(0),
        cli::Delimiter::Lf => Delimiter::Byte(b'\n'),
        cli::Delimiter::Cr => Delimiter::Byte(b'\r'),
        cli::Delimiter::Crlf => Delimiter::Newline,
        cli::Delimiter::Auto => {
            if opt.allow_prefix || opt.input_format == cli::InputFormat::Logfmt {
                Delimiter::Newline
            } else if opt.input_format == cli::InputFormat::Json {
                Delimiter::Json
            } else {
                Delimiter::PrettyCompatible
            }
        }
    };

    let mut input_info = *opt.input_info;
    if input_info.contains(InputInfo::Auto) {
        log::debug!("configured input info layouts: {input_info}");
        input_info = InputInfo::resolve(input_info);
        log::debug!("* resolved input info layouts: {input_info}");
        match terminal_size_of(stdout()).map(|(w, _)| w.0) {
            None => {
                log::debug!("* no terminal detected");
            }
            Some(200..) => {
                log::debug!("* terminal is wide enough to show full input info");
            }
            Some(160..) => {
                log::debug!("* terminal is wide enough to show compact input info");
                if input_info.intersects(enum_set!(InputInfo::Minimal | InputInfo::Compact)) {
                    input_info.remove(InputInfo::Full);
                }
            }
            _ => {
                log::debug!("* terminal is too narrow to show any input info except minimal");
                if input_info.intersects(enum_set!(InputInfo::Minimal | InputInfo::Compact)) {
                    input_info.remove(InputInfo::Full);
                }
                if input_info.contains(InputInfo::Minimal) {
                    input_info.remove(InputInfo::Compact);
                }
            }
        }
    }

    // Convert cli::AsciiOption to AsciiModeOpt, then resolve to concrete AsciiMode
    let ascii_opt = AsciiModeOpt::from(opt.ascii);
    let utf8_is_supported = matches!(utf8_supported(), Utf8Support::UTF8);
    let ascii = ascii_opt.resolve(utf8_is_supported);

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
        input_info,
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
        ascii,
        expand: opt.expansion.into(),
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
    if inputs.is_empty() {
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

    let mut output: OutputStream = match opt.output {
        Some(output) => Box::new(std::fs::File::create(PathBuf::from(&output))?),
        None => pager(),
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

fn list_themes(dirs: &AppDirs, tags: Option<cli::ThemeTagSet>) -> Result<()> {
    let items = Theme::list(dirs)?;
    let mut formatter = help::Formatter::new(stdout());

    let tags = tags.unwrap_or_default();
    let mut exclude = ThemeTagSet::default();
    if !tags.contains(cli::ThemeTag::Base) {
        exclude.insert(cli::ThemeTag::Base);
    }
    if !tags.contains(cli::ThemeTag::Overlay) {
        exclude.insert(cli::ThemeTag::Overlay);
    }

    formatter.format_grouped_list(
        items
            .into_iter()
            .filter(|(name, _)| {
                themecfg::Theme::load(dirs, name)
                    .ok()
                    .map(|theme| theme.tags.includes(*tags) && !theme.tags.intersects(*exclude))
                    .unwrap_or(false)
            })
            .sorted_by_key(|x| (x.1.origin, x.0.clone()))
            .chunk_by(|x| x.1.origin)
            .into_iter()
            .map(|(origin, group)| (origin, group.map(|x| x.0))),
    )?;
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        err.log(&AppInfo);
        process::exit(1);
    }
}

struct AppInfo;

impl AppInfoProvider for AppInfo {
    fn usage_suggestion(&self, request: UsageRequest) -> Option<UsageResponse> {
        match request {
            UsageRequest::ListThemes => Some(("--list-themes".into(), "".into())),
            UsageRequest::ListThemeOverlays => Some(("--list-themes=overlay".into(), "".into())),
        }
    }
}
