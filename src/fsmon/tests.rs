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

fn filter() -> Filter {
    Filter {
        paths: vec![PathBuf::from("/logs/a.log"), PathBuf::from("/logs/b.log")],
    }
}

fn event(kind: EventKind, path: &str) -> Event {
    Event::new(kind).add_path(PathBuf::from(path))
}

const MODIFY: EventKind = EventKind::Modify(ModifyKind::Data(DataChange::Any));
const REMOVE: EventKind = EventKind::Remove(RemoveKind::File);

#[cfg(not(target_os = "macos"))]
fn drain(queue: &Queue) -> Vec<notify::Result<Event>> {
    queue.events.lock().unwrap().drain(..).collect()
}

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

#[cfg(not(target_os = "macos"))]
#[test]
fn test_queue_drops_events_not_accepted() {
    let queue = Queue::new(filter());
    queue.push(Ok(event(MODIFY, "/logs/c.log")));
    queue.push(Ok(event(EventKind::Access(AccessKind::Any), "/logs/a.log")));
    queue.push(Ok(event(MODIFY, "/logs/a.log")));

    let events = drain(&queue);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].as_ref().unwrap().paths, [PathBuf::from("/logs/a.log")]);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_queue_drops_repeats_of_last_event_waiting() {
    let queue = Queue::new(filter());
    for _ in 0..3 {
        queue.push(Ok(event(MODIFY, "/logs/a.log")));
    }
    queue.push(Ok(event(MODIFY, "/logs/b.log")));
    queue.push(Ok(event(MODIFY, "/logs/b.log")));
    queue.push(Ok(event(REMOVE, "/logs/b.log")));
    queue.push(Ok(event(MODIFY, "/logs/b.log")));

    let events = drain(&queue)
        .into_iter()
        .map(|event| event.map(|event| (event.kind, event.paths)).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        events,
        [
            (MODIFY, vec![PathBuf::from("/logs/a.log")]),
            (MODIFY, vec![PathBuf::from("/logs/b.log")]),
            (REMOVE, vec![PathBuf::from("/logs/b.log")]),
            (MODIFY, vec![PathBuf::from("/logs/b.log")]),
        ]
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_queue_drops_repeats_with_other_detail() {
    let queue = Queue::new(filter());
    queue.push(Ok(event(MODIFY, "/logs/a.log")));
    queue.push(Ok(event(
        EventKind::Modify(ModifyKind::Data(DataChange::Size)),
        "/logs/a.log",
    )));
    queue.push(Ok(event(EventKind::Modify(ModifyKind::Any), "/logs/a.log")));
    queue.push(Ok(event(EventKind::Create(CreateKind::File), "/logs/a.log")));

    let kinds = drain(&queue)
        .into_iter()
        .map(|event| event.unwrap().kind)
        .collect::<Vec<_>>();
    assert_eq!(kinds, [MODIFY, EventKind::Create(CreateKind::File)]);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_queue_keeps_event_repeating_one_taken() {
    let queue = Queue::new(filter());
    queue.push(Ok(event(MODIFY, "/logs/a.log")));
    assert_eq!(queue.pop().unwrap().kind, MODIFY);
    queue.push(Ok(event(MODIFY, "/logs/a.log")));
    assert_eq!(drain(&queue).len(), 1);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_queue_keeps_errors() {
    let queue = Queue::new(filter());
    queue.push(Ok(event(MODIFY, "/logs/a.log")));
    queue.push(Err(notify::Error::generic("first")));
    queue.push(Err(notify::Error::generic("second")));
    queue.push(Ok(event(MODIFY, "/logs/a.log")));

    let events = drain(&queue);
    assert_eq!(events.len(), 4);
    assert!(events[1].is_err() && events[2].is_err());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_queue_pop_waits_for_event() {
    let queue = std::sync::Arc::new(Queue::new(filter()));
    let popper = thread::spawn({
        let queue = queue.clone();
        move || queue.pop().unwrap().kind
    });
    thread::sleep(Duration::from_millis(50));
    queue.push(Ok(event(REMOVE, "/logs/a.log")));
    assert_eq!(popper.join().unwrap(), REMOVE);
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
    let watched = file.clone();
    let monitor = thread::spawn(move || {
        let mut first = true;
        run(vec![watched], |event| {
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
        })
    });

    let mut started = false;
    for _ in 0..100 {
        append(&file, 1);
        if started_rx.recv_timeout(Duration::from_millis(100)).is_ok() {
            started = true;
            break;
        }
    }
    assert!(started, "no event came for the followed file");

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
    append(&file, 1);
    assert!(monitor.join().unwrap().is_err());
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
    let watched = file.clone();
    let monitor = thread::spawn(move || {
        run(vec![watched], |event| {
            // Once the test is done, the next event stops the monitor.
            handled_tx.send(event).map_err(|_| std::io::Error::other("done").into())
        })
    });

    // Wait for the watch to start, then replace the file.
    thread::sleep(Duration::from_millis(200));
    fs::remove_file(&file).unwrap();
    thread::sleep(Duration::from_millis(200));
    fs::write(&file, b"").unwrap();

    let mut handled = Vec::new();
    while let Ok(event) = handled_rx.recv_timeout(Duration::from_millis(500)) {
        handled.push(event);
    }

    drop(handled_rx);
    append(&file, 1);
    assert!(monitor.join().unwrap().is_err());
    fs::remove_dir_all(&dir).unwrap();

    assert!(handled.iter().any(|event| event.kind.is_create()), "{:?}", handled);
    assert!(
        handled.iter().all(|event| event.paths == [canonical.clone()]),
        "{:?}",
        handled
    );
}
