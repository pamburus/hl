//! Diagnostics about input content that structured output leaves out.
//!
//! Structured output holds nothing but records, so input content that is not a record is
//! dropped from it. Each dropped piece is counted per input and, with debug logging enabled,
//! reported with its byte range and a preview. At exit, one summary entry per input tells how
//! many pieces were dropped from it.

// std imports
use std::{
    fmt,
    sync::{
        Arc, Mutex, Once,
        atomic::{AtomicU64, Ordering},
    },
};

// third-party imports
use log::Level;

// local imports
use crate::{
    formatting::{DropSink, UnparsedData},
    input::InputDescriptor,
    model::Origin,
};

// ---

/// The maximum number of characters in a preview of dropped content.
const PREVIEW_LEN: usize = 100;

// ---

/// What kind of input text is not a record, by where it is in the input entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnparsedKind {
    /// An entry with no record in it.
    Entry,
    /// Text after the records in an entry.
    Remainder,
    /// Text before a record in its entry.
    Prefix,
    /// A part of an entry too long to be read as a record.
    Fragment,
}

impl UnparsedKind {
    fn describe(self) -> &'static str {
        match self {
            Self::Entry => "unparsed entry",
            Self::Fragment => "part of an entry too long to parse",
            Self::Remainder => "unparsed text after a record",
            Self::Prefix => "prefix before a record",
        }
    }
}

// ---

/// Receives diagnostic entries.
pub trait DiagnosticSink: Send + Sync {
    /// Returns `true` if entries of `level` are written.
    fn enabled(&self, level: Level) -> bool;
    /// Writes an entry of `level`.
    fn write(&self, level: Level, args: fmt::Arguments);
}

/// Writes diagnostic entries to the internal log, controlled by `HL_DEBUG_LOG`.
pub struct LogSink;

impl DiagnosticSink for LogSink {
    #[inline]
    fn enabled(&self, level: Level) -> bool {
        log::log_enabled!(level)
    }

    fn write(&self, level: Level, args: fmt::Arguments) {
        log::log!(level, "{args}");
    }
}

// ---

/// Counts and reports the content dropped from each input during a run.
pub struct Diagnostics {
    sink: Arc<dyn DiagnosticSink>,
    /// Name and number of dropped pieces of each input, by input index.
    inputs: Box<[(String, AtomicU64)]>,
    summary: Once,
}

impl Diagnostics {
    pub fn new(sink: Arc<dyn DiagnosticSink>, inputs: &[InputDescriptor]) -> Self {
        Self {
            sink,
            inputs: inputs.iter().map(|x| (x.name.clone(), AtomicU64::new(0))).collect(),
            summary: Once::new(),
        }
    }

    /// Counts a piece of dropped content and, if debug entries are enabled, reports it.
    pub(crate) fn report_dropped(&self, content: &[u8], origin: &Origin, kind: UnparsedKind) {
        if let Some((_, count)) = self.inputs.get(origin.input.index) {
            count.fetch_add(1, Ordering::Relaxed);
        }
        if self.sink.enabled(Level::Debug) {
            self.sink.write(
                Level::Debug,
                format_args!(
                    "{}: dropped {} at {:?}: {}",
                    origin.input.name,
                    kind.describe(),
                    origin.range,
                    Preview(content)
                ),
            );
        }
    }

    /// Writes one entry per input with dropped content, telling how many pieces were dropped.
    ///
    /// Only the first call writes anything, so every exit path can call it.
    pub fn write_summary(&self) {
        self.summary.call_once(|| {
            if !self.sink.enabled(Level::Info) {
                return;
            }
            for (name, count) in &self.inputs {
                match count.load(Ordering::Relaxed) {
                    0 => {}
                    1 => self
                        .sink
                        .write(Level::Info, format_args!("{name}: dropped 1 piece of unparsed text")),
                    n => self
                        .sink
                        .write(Level::Info, format_args!("{name}: dropped {n} pieces of unparsed text")),
                }
            }
        });
    }
}

impl DropSink for &Diagnostics {
    #[inline]
    fn take(&mut self, unparsed: UnparsedData) {
        self.report_dropped(unparsed.data, &unparsed.origin, unparsed.kind);
    }
}

// ---

/// Reports the keys dropped from a record while formatting it.
#[derive(Clone, Copy)]
pub(crate) struct RecordDiagnostics<'a> {
    sink: &'a dyn DiagnosticSink,
    origin: &'a Origin<'a>,
}

impl<'a> RecordDiagnostics<'a> {
    /// Returns the diagnostics for a record from `origin`, or `None` if debug entries are disabled.
    #[inline]
    pub(crate) fn new(sink: &'a dyn DiagnosticSink, origin: &'a Origin<'a>) -> Option<Self> {
        sink.enabled(Level::Debug).then_some(Self { sink, origin })
    }

    /// Reports a key that occurs more than once at a nesting level, whose other occurrences are dropped.
    pub(crate) fn report_repeat(&self, key: &str) {
        self.sink.write(
            Level::Debug,
            format_args!(
                "{}: dropped repeated key {key:?} of the record at {:?}, keeping its last value",
                self.origin.input.name, self.origin.range
            ),
        );
    }

    /// Reports a field that is dropped because a predefined field has its key.
    pub(crate) fn report_collision(&self, key: &str) {
        self.sink.write(
            Level::Debug,
            format_args!(
                "{}: dropped field {key:?} of the record at {:?}, its key is taken by a predefined field",
                self.origin.input.name, self.origin.range
            ),
        );
    }
}

// ---

static ACTIVE: Mutex<Option<Arc<Diagnostics>>> = Mutex::new(None);

/// Makes `diagnostics` the ones whose summary [`on_exit`] writes.
pub(crate) fn register(diagnostics: Arc<Diagnostics>) {
    *ACTIVE.lock().unwrap_or_else(|e| e.into_inner()) = Some(diagnostics);
}

/// Writes the summary of the registered diagnostics.
///
/// It is meant for exits that skip the normal end of a run, such as on a signal or a closed output.
pub fn on_exit() {
    let active = ACTIVE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(diagnostics) = active {
        diagnostics.write_summary();
    }
}

// ---

/// A quoted preview of content, cut to [`PREVIEW_LEN`] characters, with control characters escaped.
struct Preview<'a>(&'a [u8]);

impl fmt::Display for Preview<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        let mut chars = self.0.utf8_chunks().flat_map(|chunk| {
            chunk
                .valid()
                .chars()
                .chain((!chunk.invalid().is_empty()).then_some('\u{FFFD}'))
        });

        f.write_char('"')?;
        for c in chars.by_ref().take(PREVIEW_LEN) {
            match c {
                '"' | '\\' => write!(f, "\\{c}")?,
                c if c.is_control() => write!(f, "{}", c.escape_default())?,
                c => f.write_char(c)?,
            }
        }
        f.write_char('"')?;
        if chars.next().is_some() {
            f.write_char('…')?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
