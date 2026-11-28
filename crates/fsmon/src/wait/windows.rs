use std::{ffi::OsString, fs::File, io, sync::Arc, time::Duration};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use super::{Entry, Wake};
use crate::stop::{Stop, windows::Signal};

// ---

/// Watches the directories of the entries with a watcher of its own.
///
/// The notifications of a file that is written to may be delayed until the writer closes it,
/// so the caller also checks the open file with a timeout.
pub struct Watch {
    _watcher: RecommendedWatcher,
    signal: Arc<Signal>,
    stop: Arc<Stop>,
}

impl Watch {
    pub fn new(entries: &[Entry], _file: Option<&File>, stop: Arc<Stop>) -> io::Result<Self> {
        let signal = stop.signal();
        let names: Vec<OsString> = entries.iter().map(|(_, name)| name.clone()).collect();
        let wake = signal.clone();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let relevant = match event {
                Ok(event) => {
                    event.need_rescan()
                        || event
                            .paths
                            .iter()
                            .any(|path| path.file_name().is_some_and(|name| names.iter().any(|n| n == name)))
                }
                // A failed watch is no reason to stop following, so the caller checks the file at once.
                Err(_) => true,
            };
            if relevant {
                wake.wake();
            }
        })
        .map_err(io::Error::other)?;
        for (dir, _) in entries {
            watcher
                .watch(dir, RecursiveMode::NonRecursive)
                .map_err(io::Error::other)?;
        }
        Ok(Self {
            _watcher: watcher,
            signal,
            stop,
        })
    }

    pub fn wait(&mut self, timeout: Option<Duration>) -> io::Result<Wake> {
        Ok(self.signal.wait(timeout, &self.stop))
    }
}
