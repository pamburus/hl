//! Wrap hl's pipeline so we can render an arbitrary byte chunk of a log file into a
//! sequence of `(byte_offset, ANSI bytes)` pairs that the handler can hand to the ANSI
//! segmenter.
//!
//! Allocation rules: the immutable pipeline state (`Parser`, `RecordFormatter`) is
//! built once at server startup and shared via `Arc`; the per-call output buffer comes
//! from a process-wide pool so steady-state request traffic does not allocate. The
//! pool is unbounded — buffers go back in on `Drop` with their capacity intact, which
//! amortises to zero allocations after warmup.
//!
//! We deliberately push *single input line* slices through `SegmentProcessor` even
//! though it can chew on bigger chunks. The reason is byte offsets: SegmentProcessor
//! concatenates formatted records into one buffer separated by a configurable
//! delimiter, and the formatted output can in principle contain that delimiter inside
//! a record. Per-line invocation gives us a clean 1:1 correspondence between input
//! line and output record and makes the byte offset trivially the running cursor in
//! the source slice. The processor itself is built once per `render_chunk` (one per
//! HTTP request) and reused across the chunk's lines, so the `Delimiter` searcher
//! isn't rebuilt per line.

use std::sync::Arc;

use anyhow::Context;
use crossbeam_queue::SegQueue;
use hl::app::{RecordIgnorer, SegmentProcess, SegmentProcessor, SegmentProcessorOptions};
use hl::formatting::{Expansion, RecordFormatterBuilder};
use hl::settings::{AsciiMode, FieldShowOption};
use hl::timezone::Tz;
use hl::{DateTimeFormatter, LinuxDateFormat, Parser, ParserSettings, RecordFilter, RecordFormatter, Settings, Theme};

/// Shared, immutable rendering state plus the buffer pool. Cloning a `RenderConfig`
/// is a single Arc bump, so passing it through `AppState` is free.
#[derive(Clone)]
pub struct RenderConfig {
    inner: Arc<Inner>,
}

struct Inner {
    parser: Parser,
    formatter: RecordFormatter,
    /// Pool of reusable output buffers. Lock-free MPMC queue (same primitive hl's
    /// internal `scanning::BufFactory` uses), so concurrent /api/render handlers
    /// don't contend on a mutex for checkout/return.
    buf_pool: SegQueue<Vec<u8>>,
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

        let formatter = RecordFormatterBuilder::new()
            .with_theme(theme)
            .with_timestamp_formatter(time_formatter)
            .with_options(settings.formatting.clone())
            .with_punctuation(punctuation)
            .with_ascii(ascii_mode)
            .with_expansion(expansion)
            .with_always_show_time(always_show_time)
            .with_always_show_level(always_show_level)
            .build();

        let parser_settings = ParserSettings::new(&settings.fields.predefined, &settings.fields.ignore, None);
        let parser = Parser::new(parser_settings);

        Ok(Self {
            inner: Arc::new(Inner {
                parser,
                formatter,
                buf_pool: SegQueue::new(),
            }),
        })
    }

    /// Check out a per-request renderer. The returned guard holds a pooled output
    /// buffer that is returned to the pool on `Drop`.
    pub fn renderer(&self) -> Renderer {
        let buf = self.inner.buf_pool.pop().unwrap_or_default();
        Renderer {
            inner: self.inner.clone(),
            buf: Some(buf),
        }
    }
}

/// Per-request render guard. Owns a pooled output buffer; on drop returns the buffer
/// to the pool (with its capacity intact).
pub struct Renderer {
    inner: Arc<Inner>,
    buf: Option<Vec<u8>>,
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
    /// Iterate over input bytes one line at a time, emitting a `Rendered` for each
    /// input line (regardless of whether the record passed the filter). The closure
    /// receives a borrowed view of the renderer's output buffer — copy or convert it
    /// before the next iteration if you need to keep it. An empty `ansi` slice means
    /// either the line wasn't parseable as a record OR the filter rejected it; the
    /// caller can disambiguate by tracking the input line count vs filtered count if
    /// it cares.
    pub fn render_chunk<Fil, F>(&mut self, bytes: &[u8], first_byte: u64, filter: Fil, mut sink: F)
    where
        Fil: RecordFilter,
        F: FnMut(Rendered<'_>),
    {
        let buf = self.buf.as_mut().expect("renderer guard already dropped");
        let opts = SegmentProcessorOptions {
            allow_prefix: false,
            allow_unparsed_data: true,
            ..SegmentProcessorOptions::default()
        };
        let mut sp = SegmentProcessor::new(&self.inner.parser, &self.inner.formatter, filter, opts);
        let mut observer = RecordIgnorer {};

        let mut cursor: u64 = first_byte;
        for line in bytes.split_inclusive(|&b| b == b'\n') {
            let start = cursor;
            cursor += line.len() as u64;
            buf.clear();
            sp.process(line, buf, "", None, &mut observer);
            let ansi = trim_trailing_newline(buf);
            sink(Rendered { start, ansi });
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Some(mut buf) = self.buf.take() {
            buf.clear();
            self.inner.buf_pool.push(buf);
        }
    }
}

fn trim_trailing_newline(s: &[u8]) -> &[u8] {
    if s.last() == Some(&b'\n') { &s[..s.len() - 1] } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time check that the rendering config can be shared across tokio task
    /// boundaries. If `Parser` or `RecordFormatter` ever become `!Sync`, this fails
    /// to compile here rather than mysteriously at the axum::State boundary.
    #[allow(dead_code)]
    fn assert_send_sync<T: Send + Sync>() {}
    const _: fn() = || {
        assert_send_sync::<RenderConfig>();
    };

    #[test]
    fn pool_recycles_buffer_capacity() {
        let cfg = RenderConfig::new().expect("render config");
        {
            let mut r = cfg.renderer();
            // Force the buf to grow.
            r.buf.as_mut().unwrap().resize(4096, 0);
        }
        // Second checkout should reuse that buf (non-zero capacity).
        let r2 = cfg.renderer();
        assert!(
            r2.buf.as_ref().unwrap().capacity() >= 4096,
            "expected pooled buffer to retain capacity; got cap={}",
            r2.buf.as_ref().unwrap().capacity()
        );
    }
}
