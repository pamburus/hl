// public modules
pub mod app;
pub mod datefmt;
pub mod error;
pub mod fmtx;
pub mod input;
pub mod output;
pub mod settings;
pub mod theme;
pub mod themecfg;
pub mod timeparse;
pub mod timestamp;
pub mod types;

// private modules
mod console;
mod eseq;
mod filtering;
mod formatting;
mod model;
mod scanning;

// conditional public modules
#[cfg_attr(unix, path = "signal_unix.rs")]
#[cfg_attr(windows, path = "signal_windows.rs")]
pub mod signal;

// public uses
pub use app::{App, FieldOptions, Options, SegmentProcesor};
pub use datefmt::{DateTimeFormatter, LinuxDateFormat};
pub use filtering::DefaultNormalizing;
pub use formatting::RecordFormatter;
pub use model::{FieldFilterSet, Filter, Level, Parser, ParserSettings};
pub use settings::Settings;
pub use theme::Theme;

// public uses (platform-specific)
pub use console::enable_ansi_support;

// public type aliases
pub type IncludeExcludeKeyFilter = filtering::IncludeExcludeKeyFilter<DefaultNormalizing>;
pub type KeyMatchOptions = filtering::MatchOptions<DefaultNormalizing>;
