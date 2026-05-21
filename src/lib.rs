// public modules
#[cfg(feature = "native")]
pub mod app;
pub mod appdirs;
#[cfg(feature = "native")]
pub mod cli;
pub mod condition;
#[cfg(feature = "native")]
pub mod config;
pub mod datefmt;
pub mod error;
pub mod fmtx;
pub mod formatting;
#[cfg(feature = "native")]
pub mod help;
#[cfg(feature = "native")]
pub mod index;
#[cfg(feature = "native")]
pub mod index_capnp;
#[cfg(feature = "native")]
pub mod input;
pub mod iox;
pub mod level;
pub mod output;
pub mod pager;
pub mod query;
pub mod settings;
pub mod syntax;
pub mod theme;
pub mod themecfg;
#[cfg(feature = "native")]
pub mod timeparse;
pub mod timestamp;
pub mod timezone;
pub mod types;

// private modules
#[cfg(feature = "native")]
mod console;
mod eseq;
mod filtering;
#[cfg(feature = "native")]
mod fsmon;
mod model;
mod number;
#[cfg(feature = "native")]
mod replay;
mod scanning;
mod serdex;
#[cfg(feature = "native")]
mod tee;
#[cfg(feature = "native")]
mod vfs;
mod xerr;

// test utilities
#[cfg(test)]
pub(crate) mod testing;

// conditional public modules
#[cfg(all(unix, feature = "native"))]
#[path = "signal_unix.rs"]
pub mod signal;
#[cfg(all(windows, feature = "native"))]
#[path = "signal_windows.rs"]
pub mod signal;

// public uses
#[cfg(feature = "native")]
pub use app::{App, FieldOptions, Options, SegmentProcessor};
pub use datefmt::{DateTimeFormatter, LinuxDateFormat};
pub use filtering::DefaultNormalizing;
pub use formatting::{RecordFormatter, RecordFormatterBuilder};
pub use model::{
    AnnotatedRawRecord, FieldFilterSet, Filter, Level, Parser, ParserSettings, RawRecord, RawRecordIterator,
    RawRecordParser, RawRecordStream, Record, RecordFilter,
};
pub use query::Query;
pub use scanning::{Delimit, Delimiter, SearchExt};
pub use settings::Settings;
pub use theme::Theme;

// public uses (platform-specific)
#[cfg(feature = "native")]
pub use console::enable_ansi_support;

// public type aliases
pub type IncludeExcludeKeyFilter = filtering::IncludeExcludeKeyFilter<DefaultNormalizing>;
pub type ExactIncludeExcludeKeyFilter = filtering::IncludeExcludeKeyFilter<filtering::NoNormalizing>;
pub type KeyMatchOptions = filtering::MatchOptions<DefaultNormalizing>;
pub type QueryNone = model::RecordFilterNone;
