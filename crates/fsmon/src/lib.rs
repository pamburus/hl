//! Follows files by name across rotation, truncation and replacement, as `tail -F` does.
//!
//! A [`Follow`] takes inputs by path, by path with a file already open at it, or as the standard input.
//! Each [`Input`] delivers a sequence of byte [`Stream`]s, one per file that appears at its path, and a
//! [`Stopper`] ends them all at once.
//!
//! Changes are waited for with native notifications of the operating system on file systems known to be local,
//! and by polling elsewhere, at an interval that adapts to how often the file changes.

mod classify;
mod identity;
mod interval;
mod options;
mod source;
mod stop;
mod wait;

pub use options::{FallbackPolicy, FollowOptions};
pub use source::{Follow, Input, Stream};
pub use stop::Stopper;
