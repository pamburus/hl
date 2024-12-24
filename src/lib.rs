// public modules
pub mod app;
pub mod appdirs;
pub mod cli;
pub mod config;
pub mod datefmt;
pub mod error;
pub mod fmtx;
pub mod formatting;
pub mod index;
pub mod index_capnp;
pub mod input;
pub mod iox;
pub mod level;
pub mod output;
pub mod query;
pub mod settings;
pub mod syntax;
pub mod theme;
pub mod themecfg;
pub mod timeparse;
pub mod timestamp;
pub mod timezone;
pub mod types;

// private modules
mod console;
mod eseq;
mod filtering;
mod fsmon;
mod model;
mod number;
mod replay;
mod scanning;
mod serdex;
mod tee;
mod vfs;
mod xerr;

// test utilities
#[cfg(test)]
pub(crate) mod testing;

// conditional public modules
#[cfg_attr(unix, path = "signal_unix.rs")]
#[cfg_attr(windows, path = "signal_windows.rs")]
pub mod signal;

// public uses
pub use app::{App, FieldOptions, Options, SegmentProcessor};
pub use datefmt::{DateTimeFormatter, LinuxDateFormat};
pub use filtering::DefaultNormalizing;
pub use formatting::RecordFormatter;
pub use model::{FieldFilterSet, Filter, Level, Parser, ParserSettings, RecordFilter};
pub use query::Query;
pub use scanning::{Delimit, Delimiter, SearchExt};
pub use settings::Settings;
pub use theme::Theme;

// public uses (platform-specific)
pub use console::enable_ansi_support;

// public type aliases
pub type IncludeExcludeKeyFilter = filtering::IncludeExcludeKeyFilter<DefaultNormalizing>;
pub type KeyMatchOptions = filtering::MatchOptions<DefaultNormalizing>;
pub type QueryNone = model::RecordFilterNone;
