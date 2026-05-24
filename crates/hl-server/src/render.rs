//! Wrap hl's pipeline so we can render an arbitrary byte chunk of a log file into a
//! sequence of `(byte_offset, ANSI bytes)` pairs that the handler can hand to the ANSI
//! segmenter.
//!
//! We deliberately push *single input line* slices through `SegmentProcessor` even
//! though it can chew on bigger chunks. The reason is byte offsets: SegmentProcessor
//! concatenates formatted records into one buffer separated by a configurable
//! delimiter, and the formatted output can in principle contain that delimiter inside a
//! record. Per-line invocation gives us a clean 1:1 correspondence between input line
//! and output record, and makes the byte offset trivially the running cursor in the
//! source slice. The cost is a fresh `Delimiter` searcher per line — negligible
//! compared to the formatter's own work.
//!
//! [`RenderConfig`] is the cloneable, shared, immutable config. Each request builds a
//! per-request [`Renderer`] which owns a freshly constructed `Parser` and
//! `RecordFormatter` plus a reusable output buffer.

use std::sync::Arc;

use anyhow::Context;
use hl::app::{RecordIgnorer, SegmentProcess, SegmentProcessor, SegmentProcessorOptions};
use hl::formatting::{Expansion, RecordFormatterBuilder};
use hl::settings::{AsciiMode, FieldShowOption, ResolvedPunctuation};
use hl::timezone::Tz;
use hl::{DateTimeFormatter, Filter, LinuxDateFormat, Parser, ParserSettings, RecordFormatter, Settings, Theme};

/// Shared rendering config. Construct one per server, clone into per-request handlers
/// (cloning is an Arc bump). Heavy setup — theme load, timezone resolution, derived
/// flags — happens once at construction time.
#[derive(Clone)]
pub struct RenderConfig {
    inner: Arc<Inner>,
}

struct Inner {
    settings: Settings,
    theme: Arc<Theme>,
    time_formatter: DateTimeFormatter,
    punctuation: Arc<ResolvedPunctuation>,
    ascii_mode: AsciiMode,
    expansion: Expansion,
    always_show_time: bool,
    always_show_level: bool,
}

impl RenderConfig {
    pub fn new() -> anyhow::Result<Self> {
        let settings = Settings::default();
        let theme = Theme::embedded("hl-dark")
            .map_err(|e| anyhow::anyhow!("failed to load embedded theme 'hl-dark': {e}"))
            .context("rendering config")?;
        let theme = Arc::new(theme);
        let ascii_mode = AsciiMode::Off;
        let punctuation = Arc::new(settings.formatting.punctuation.resolve(ascii_mode));
        let tz = Tz::IANA(settings.time_zone);
        let time_format = LinuxDateFormat::new(&settings.time_format).compile();
        let time_formatter = DateTimeFormatter::new(time_format, tz);
        let expansion = Expansion::from(settings.formatting.expansion.clone());
        let always_show_time = settings.fields.predefined.time.show == FieldShowOption::Always;
        let always_show_level = settings.fields.predefined.level.show == FieldShowOption::Always;
        Ok(Self {
            inner: Arc::new(Inner {
                settings,
                theme,
                time_formatter,
                punctuation,
                ascii_mode,
                expansion,
                always_show_time,
                always_show_level,
            }),
        })
    }

    /// Build a per-request `Renderer`. Construction is on the cheap side (constructs
    /// `ParserSettings` and a `RecordFormatter` from the cached inputs), but it does
    /// allocate — share a single `Renderer` across the lines of a single request.
    pub fn make_renderer(&self) -> Renderer {
        let parser_settings = ParserSettings::new(
            &self.inner.settings.fields.predefined,
            &self.inner.settings.fields.ignore,
            None,
        );
        let parser = Parser::new(parser_settings);
        let formatter = RecordFormatterBuilder::new()
            .with_theme(self.inner.theme.clone())
            .with_timestamp_formatter(self.inner.time_formatter.clone())
            .with_options(self.inner.settings.formatting.clone())
            .with_punctuation(self.inner.punctuation.clone())
            .with_ascii(self.inner.ascii_mode)
            .with_expansion(self.inner.expansion.clone())
            .with_always_show_time(self.inner.always_show_time)
            .with_always_show_level(self.inner.always_show_level)
            .build();
        Renderer {
            parser,
            formatter,
            line_buf: Vec::with_capacity(8 * 1024),
        }
    }
}

/// Per-request render state. Holds the parser and formatter for *this* request; the
/// per-line output buffer is reused across lines.
pub struct Renderer {
    parser: Parser,
    formatter: RecordFormatter,
    line_buf: Vec<u8>,
}

/// One rendered input line, as handed to the sink closure during `render_chunk`.
pub struct Rendered<'a> {
    /// Byte offset of the line's first byte, relative to wherever `render_chunk` was
    /// told to start counting from.
    pub start: u64,
    /// ANSI-encoded formatted output for this single record. Empty if hl produced no
    /// output (e.g. the line wasn't parseable and `allow_unparsed_data` happened to
    /// drop it).
    pub ansi: &'a [u8],
}

impl Renderer {
    /// Iterate over input bytes one line at a time, emitting a `Rendered` for each.
    /// The closure receives a borrowed view of the renderer's internal output buffer
    /// — copy/convert it before the next iteration if you need to keep it.
    pub fn render_chunk<F>(&mut self, bytes: &[u8], first_byte: u64, mut sink: F)
    where
        F: FnMut(Rendered<'_>),
    {
        let mut cursor: u64 = first_byte;
        for line in bytes.split_inclusive(|&b| b == b'\n') {
            let start = cursor;
            cursor += line.len() as u64;

            self.line_buf.clear();
            // SegmentProcessorOptions isn't Clone, so build a fresh one each iteration.
            // The construction is cheap — just default values.
            let opts = SegmentProcessorOptions {
                allow_prefix: false,
                allow_unparsed_data: true,
                ..SegmentProcessorOptions::default()
            };
            let mut sp = SegmentProcessor::new(&self.parser, &self.formatter, Filter::default(), opts);
            let mut observer = RecordIgnorer {};
            sp.process(line, &mut self.line_buf, "", None, &mut observer);

            // The formatter may emit a trailing '\n' on each record. We send
            // structured per-line records, not a stream with line terminators, so
            // strip it.
            let ansi = trim_trailing_newline(&self.line_buf);
            sink(Rendered { start, ansi });
        }
    }
}

fn trim_trailing_newline(s: &[u8]) -> &[u8] {
    if s.last() == Some(&b'\n') { &s[..s.len() - 1] } else { s }
}
