use std::env;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use ansi_term::Colour;
use isatty::stdout_isatty;
use structopt::clap::arg_enum;
use structopt::StructOpt;

use hl::error::*;
use hl::input::{open, ConcatReader, Input, InputStream};
use hl::output::{OutputStream, Pager};

/// JSON log converter to human readable representation.
#[derive(StructOpt)]
#[structopt()]
struct Opt {
    /// Color output options, one of { auto, always, never }
    #[structopt(long, default_value = "auto")]
    color: Color,
    //
    /// Handful alias for --color=always, overrides --color option
    #[structopt(short)]
    color_always: bool,
    //
    /// Output paging options, one of { auto, always, never }
    #[structopt(long, default_value = "auto")]
    paging: Paging,
    //
    /// Color theme, one of { auto, dark, dark24, light }
    #[structopt(long, default_value = "auto")]
    theme: Theme,
    //
    /// Do not unescape string fields.
    #[structopt(short, long)]
    raw_fields: bool,
    //
    /// Buffer size, kibibytes.
    #[structopt(long, default_value = "2048")]
    buffer_size: usize,
    //
    /// Number of processing threads. Zero means automatic selection.
    #[structopt(long, default_value = "0")]
    concurrency: usize,
    //
    /// Filtering by field values in form <key>=<value> or <key>~=<value>.
    #[structopt(short, long)]
    filter: Vec<String>,
    //
    /// Filtering by level, valid values: ['d', 'i', 'w', 'e'].
    #[structopt(short, long, default_value = "d")]
    level: char,
    //
    /// Time format, see https://man7.org/linux/man-pages/man3/strftime.3.html.
    #[structopt(short, long, default_value = "%b %d %T.%3f")]
    time_format: String,
    //
    /// Files to process
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
}

arg_enum! {
    #[derive(Debug)]
    enum Color {
        Auto,
        Always,
        Never,
    }
}

arg_enum! {
    #[derive(Debug)]
    enum Theme {
        Auto,
        Dark,
        Dark24,
        Light,
    }
}

arg_enum! {
    #[derive(Debug)]
    enum Paging {
        Auto,
        Always,
        Never,
    }
}

fn run() -> Result<()> {
    let opt = Opt::from_args();

    // Configure color scheme.
    let color = if opt.color_always {
        Color::Always
    } else {
        opt.color
    };
    let truecolor = env::var("COLORTERM").unwrap_or_default() == "truecolor";
    let theme = |theme: Theme| match (theme, truecolor) {
        (Theme::Auto, false) | (Theme::Dark, _) => hl::theme::Theme::dark(),
        (Theme::Auto, true) | (Theme::Dark24, _) => hl::theme::Theme::dark24(),
        (Theme::Light, _) => hl::theme::Theme::light(),
    };
    let theme = match color {
        Color::Auto => match stdout_isatty() {
            true => theme(opt.theme),
            false => hl::theme::Theme::none(),
        },
        Color::Always => theme(opt.theme),
        Color::Never => hl::theme::Theme::none(),
    };

    // Configure concurrency.
    let concurrency = match opt.concurrency {
        0 => num_cpus::get(),
        _ => opt.concurrency,
    };

    // Configure buffer size.
    let buffer_size = match opt.buffer_size {
        0 => 2 << 20,
        _ => opt.buffer_size << 10,
    };
    // Configure level.
    let level = match opt.level {
        'e' | 'E' => hl::Level::Error,
        'w' | 'W' => hl::Level::Warning,
        'i' | 'I' => hl::Level::Info,
        'd' | 'D' => hl::Level::Debug,
        _ => {
            return Err(format!(
                "invalid level '{}': use any of ['{}', '{}', '{}', '{}']",
                Colour::Yellow.paint(opt.level.to_string()),
                Colour::Green.paint("e"),
                Colour::Green.paint("w"),
                Colour::Green.paint("i"),
                Colour::Green.paint("d"),
            )
            .into());
        }
    };
    // Configure filter.
    let filter = hl::Filter {
        fields: hl::FieldFilterSet::new(&opt.filter[..]),
        level: Some(level),
    };

    // Create app.
    let app = hl::App::new(hl::Options {
        theme: Arc::new(theme),
        raw_fields: opt.raw_fields,
        time_format: opt.time_format,
        buffer_size: buffer_size,
        concurrency: concurrency,
        filter: filter,
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
        Paging::Auto => {
            if stdout_isatty() {
                true
            } else {
                false
            }
        }
        Paging::Always => true,
        Paging::Never => false,
    };
    let mut output: OutputStream = if paging {
        if let Ok(pager) = Pager::new() {
            Box::new(pager)
        } else {
            Box::new(std::io::stdout())
        }
    } else {
        Box::new(std::io::stdout())
    };

    // Run app.
    match app.run(input.as_mut(), output.as_mut()) {
        Ok(()) => Ok(()),
        Err(Error(ErrorKind::Io(ref e), _)) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(err),
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}: {}", Colour::Red.paint("error"), err);
        process::exit(1);
    }
}
