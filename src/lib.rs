// public modules
pub mod app;
pub mod cli;
pub mod config;
pub mod datefmt;
pub mod error;
pub mod fmtx;
pub mod index;
pub mod index_capnp;
pub mod input;
pub mod iox;
pub mod level;
pub mod output;
pub mod query;
pub mod settings;
pub mod theme;
pub mod themecfg;
pub mod timeparse;
pub mod timestamp;
pub mod timezone;
pub mod types;

// private modules
mod appdirs;
mod console;
mod eseq;
mod filtering;
pub mod format;
pub mod formatting;
mod fsmon;
pub mod model;
pub mod processing;
mod replay;
mod scanning;
mod serdex;
mod tee;
mod vfs;

// conditional public modules
#[cfg_attr(unix, path = "signal_unix.rs")]
#[cfg_attr(windows, path = "signal_windows.rs")]
pub mod signal;

// public uses
pub use app::{App, FieldOptions, Options, SegmentProcessor};
pub use datefmt::{DateTimeFormatter, LinuxDateFormat};
pub use filtering::DefaultNormalizing;
pub use formatting::{RecordFormatter, RecordWithSourceFormatter};
pub use model::{FieldFilterSet, Filter, Level, Parser, ParserSettings, RecordFilter, RecordWithSource};
pub use query::Query;
pub use scanning::Delimiter;
pub use settings::Settings;
pub use theme::Theme;

// public uses (platform-specific)
pub use console::enable_ansi_support;

// public type aliases
pub type IncludeExcludeKeyFilter = filtering::IncludeExcludeKeyFilter<DefaultNormalizing>;
pub type KeyMatchOptions = filtering::MatchOptions<DefaultNormalizing>;
pub type QueryNone = model::RecordFilterNone;
