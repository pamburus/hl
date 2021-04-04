// public modules
pub mod app;
pub mod datefmt;
pub mod error;
pub mod fmtx;
pub mod input;
pub mod output;
pub mod theme;
pub mod timestamp;
pub mod types;

// private modules
mod chop;
mod eseq;
mod filtering;
mod formatting;
mod model;
mod pool;
// mod scanning;

// conditional public modules
#[cfg_attr(unix, path = "signal_unix.rs")]
#[cfg_attr(windows, path = "signal_windows.rs")]
pub mod signal;

// public uses
pub use app::App;
pub use app::Options;
pub use filtering::DefaultNormalizing;
pub use model::FieldFilterSet;
pub use model::Filter;
pub use model::Level;

// public type aliases
pub type IncludeExcludeKeyFilter = filtering::IncludeExcludeKeyFilter<DefaultNormalizing>;
pub type KeyMatchOptions = filtering::MatchOptions<DefaultNormalizing>;
