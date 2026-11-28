//! Noticing that the reader of the standard output has gone while nothing is written to it.

use std::thread;

// ---

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use other as imp;

// ---

/// Calls `f` from a thread of its own once the reader of the standard output closes it, if it is a pipe or a socket.
///
/// A process that writes rarely, such as one following files, would otherwise find it closed only at its next write.
///
/// Returns false if the standard output is not watched, because it is something else, or because the platform offers
/// no way to wait for that.
pub fn on_stdout_closed(f: impl FnOnce() + Send + 'static) -> bool {
    imp::watchable()
        && thread::Builder::new()
            .name("stdout-watch".into())
            .spawn(move || {
                if imp::wait_closed() {
                    f();
                }
            })
            .is_ok()
}
