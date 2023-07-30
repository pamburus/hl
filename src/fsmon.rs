// std imports
use std::path::PathBuf;
use std::sync::mpsc::{self};
use std::time::Duration;

// third-party imports
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

// local imports
use crate::error::{Error, Result};

// ---

pub type EventKind = notify::EventKind;
pub type Event = notify::Event;

// ---

const FALLBACK_POLLING_TIMEOUT: Duration = Duration::from_secs(1);

pub fn run<H>(mut paths: Vec<PathBuf>, mut handle: H) -> Result<()>
where
    H: FnMut(Event) -> Result<()>,
{
    if paths.is_empty() {
        return Ok(());
    }

    paths.retain(|path| path.metadata().map_or(false, |metadata| metadata.file_type().is_file()));

    for i in 0..paths.len() {
        if let Ok(canonical_path) = paths[i].canonicalize() {
            match paths[i].symlink_metadata() {
                Ok(metadata) if metadata.file_type().is_symlink() => paths.push(canonical_path),
                _ => paths[i] = canonical_path,
            }
        }
    }

    paths.sort_unstable();
    paths.dedup();

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(FALLBACK_POLLING_TIMEOUT))?;
    let mut parents = paths
        .iter()
        .map(|path| {
            let mut path = path.clone();
            path.pop();
            path
        })
        .collect::<Vec<PathBuf>>();
    parents.sort_unstable();
    parents.dedup();

    for parent in &parents {
        if let Err(err) = watcher.watch(parent, RecursiveMode::NonRecursive) {
            return Err(err.into());
        }
    }

    loop {
        match rx.recv().map_err(Into::into) {
            Ok(Ok(event)) => {
                if event.paths.iter().any(|path| paths.binary_search(&path).is_ok()) {
                    handle(event)?;
                }
            }
            Ok(Err(err)) => return Err(err.into()),
            Err(err) => return Err(Error::RecvTimeoutError { source: err }),
        };
    }
}
