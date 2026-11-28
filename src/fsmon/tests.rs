use super::*;

use notify::event::{AccessKind, CreateKind, DataChange, ModifyKind, RemoveKind};
use rstest::rstest;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::mpsc,
    thread,
    time::Duration,
};

pub(super) fn filter() -> Filter {
    Filter {
        paths: vec![PathBuf::from("/logs/a.log"), PathBuf::from("/logs/b.log")],
    }
}

pub(super) fn event(kind: EventKind, path: &str) -> Event {
    Event::new(kind).add_path(PathBuf::from(path))
}

pub(super) const MODIFY: EventKind = EventKind::Modify(ModifyKind::Data(DataChange::Any));
pub(super) const REMOVE: EventKind = EventKind::Remove(RemoveKind::File);

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hl-fsmon-{}-{}", std::process::id(), name));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Appends a line to `path` `times` times, each with a write of its own.
fn append(path: &Path, times: usize) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    for _ in 0..times {
        file.write_all(b"x\n").unwrap();
    }
}

/// Appends to `path` until something comes from `rx`, so the monitor is known to watch it, and returns whether it did.
fn wait_watched<T>(rx: &mpsc::Receiver<T>, path: &Path) -> bool {
    (0..100).any(|_| {
        append(path, 1);
        rx.recv_timeout(Duration::from_millis(100)).is_ok()
    })
}

/// Appends to `path` until the monitor, whose handler fails once the test is done, sends its result to `done`.
fn wait_stopped(done: &mpsc::Receiver<Result<()>>, path: &Path) -> Result<()> {
    for _ in 0..50 {
        append(path, 1);
        if let Ok(result) = done.recv_timeout(Duration::from_millis(100)) {
            return result;
        }
    }
    panic!("the monitor did not stop");
}

#[rstest]
#[case::modify(event(MODIFY, "/logs/a.log"), true)]
#[case::create(event(EventKind::Create(CreateKind::File), "/logs/b.log"), true)]
#[case::remove(event(REMOVE, "/logs/a.log"), true)]
#[case::any(event(EventKind::Any, "/logs/a.log"), true)]
#[case::access(event(EventKind::Access(AccessKind::Any), "/logs/a.log"), false)]
#[case::other_file(event(MODIFY, "/logs/c.log"), false)]
#[case::directory(event(MODIFY, "/logs"), false)]
#[case::no_paths(Event::new(EventKind::Other), false)]
fn test_filter(#[case] event: Event, #[case] accepted: bool) {
    assert_eq!(filter().accepts(&event), accepted);
}

#[test]
fn test_events_coming_while_busy_are_coalesced() {
    let dir = temp_dir("busy");
    let file = dir.join("followed.log");
    let other = dir.join("other.log");
    fs::write(&file, b"").unwrap();
    fs::write(&other, b"").unwrap();

    // The first event keeps the handler busy until resumed, the next ones are counted.
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel::<()>();
    let (handled_tx, handled_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let watched = file.clone();
    thread::spawn(move || {
        let mut first = true;
        let result = run(vec![watched], |event| {
            if first {
                first = false;
                started_tx.send(()).unwrap();
                resume_rx.recv().unwrap();
                return Ok(());
            }
            // Once the test is done, the next event stops the monitor.
            handled_tx
                .send(event.kind)
                .map_err(|_| std::io::Error::other("done").into())
        });
        done_tx.send(result).ok();
    });

    assert!(wait_watched(&started_rx, &file), "no event came for the followed file");

    // Many writes to the followed file and to another file in its directory, and a new file in the directory,
    // while the handler is busy.
    append(&file, 1000);
    append(&other, 1000);
    fs::write(dir.join("new.log"), b"").unwrap();
    // Events are delivered by a thread of the watcher, so give it time to deliver them while the handler is still busy.
    thread::sleep(Duration::from_millis(300));
    resume_tx.send(()).unwrap();

    let mut handled = Vec::new();
    while let Ok(kind) = handled_rx.recv_timeout(Duration::from_millis(500)) {
        handled.push(kind);
    }

    drop(handled_rx);
    assert!(wait_stopped(&done_rx, &file).is_err());
    fs::remove_dir_all(&dir).unwrap();

    // Handling one event reads all that was written before it, so the writes need no more than a few.
    assert!(
        (1..=3).contains(&handled.len()),
        "{} events handled: {:?}",
        handled.len(),
        &handled[..handled.len().min(10)]
    );
}

#[test]
fn test_recreated_file_brings_only_its_own_events() {
    let dir = temp_dir("recreated");
    let file = dir.join("followed.log");
    fs::write(&file, b"").unwrap();
    let canonical = file.canonicalize().unwrap();

    let (handled_tx, handled_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let watched = file.clone();
    thread::spawn(move || {
        let result = run(vec![watched], |event| {
            // Once the test is done, the next event stops the monitor.
            handled_tx.send(event).map_err(|_| std::io::Error::other("done").into())
        });
        done_tx.send(result).ok();
    });

    // Once the file is watched, replace it.
    assert!(wait_watched(&handled_rx, &file), "no event came for the followed file");
    fs::remove_file(&file).unwrap();
    thread::sleep(Duration::from_millis(200));
    fs::write(&file, b"").unwrap();

    let mut handled = Vec::new();
    while let Ok(event) = handled_rx.recv_timeout(Duration::from_millis(500)) {
        handled.push(event);
    }

    drop(handled_rx);
    assert!(wait_stopped(&done_rx, &file).is_err());
    fs::remove_dir_all(&dir).unwrap();

    assert!(handled.iter().any(|event| event.kind.is_create()), "{:?}", handled);
    assert!(
        handled.iter().all(|event| event.paths == [canonical.clone()]),
        "{:?}",
        handled
    );
}
