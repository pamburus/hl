use std::env;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use ansi_term::Colour;
use isatty::stdout_isatty;
use structopt::clap::arg_enum;
use structopt::StructOpt;

use hl::error::*;
use hl::input::ConcatReader;

/// JSON log converter to human readable representation.
#[derive(StructOpt)]
#[structopt()]
struct Opt {
    /// Color output options, one of { auto, always, never }
    #[structopt(long, default_value = "auto")]
    color: Color,

    /// Handful alias for --color=always, overrides --color option
    #[structopt(short)]
    color_always: bool,

    /// Color theme, one of { auto, dark, dark24, light }
    #[structopt(long, default_value = "auto")]
    theme: Theme,
    /// Do not unescape string fields.
    #[structopt(short, long)]
    raw_fields: bool,

    /// Buffer size, kibibytes.
    #[structopt(long, default_value = "2048")]
    buffer_size: usize,

    /// Number of processing threads. Zero means automatic selection.
    #[structopt(long, default_value = "0")]
    concurrency: usize,

    /// Filtering by field values in form <key>=<value> or <key>~=<value>.
    #[structopt(short, long)]
    filter: Vec<String>,

    /// Filtering by level, valid values: ['d', 'i', 'w', 'e'].
    #[structopt(short, long, default_value = "d")]
    level: char,

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

fn run() -> Result<()> {
    let opt = Opt::from_args();
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

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

    let concurrency = match opt.concurrency {
        0 => num_cpus::get(),
        _ => opt.concurrency,
    };
    let buffer_size = match opt.buffer_size {
        0 => 2 << 20,
        _ => opt.buffer_size << 10,
    };
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
    let filter = hl::Filter {
        fields: hl::FieldFilterSet::new(&opt.filter[..]),
        level: Some(level),
    };
    let app = hl::App::new(hl::Options {
        theme: Arc::new(theme),
        raw_fields: opt.raw_fields,
        buffer_size: buffer_size,
        concurrency: concurrency,
        filter: filter,
    });
    let mut files = ConcatReader::new(opt.files);
    match app.run(
        if !files.is_empty() {
            &mut files
        } else {
            &mut stdin
        },
        &mut stdout,
    ) {
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
