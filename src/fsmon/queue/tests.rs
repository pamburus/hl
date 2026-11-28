use super::*;

use notify::event::{AccessKind, CreateKind, DataChange, ModifyKind};
use std::{path::PathBuf, sync::Arc, thread, time::Duration};

use crate::fsmon::{
    EventKind,
    tests::{MODIFY, REMOVE, event, filter},
};

fn drain(queue: &Queue) -> Vec<notify::Result<Event>> {
    queue.events.lock().unwrap().drain(..).collect()
}

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

#[test]
fn test_queue_keeps_event_repeating_one_taken() {
    let queue = Queue::new(filter());
    queue.push(Ok(event(MODIFY, "/logs/a.log")));
    assert_eq!(queue.pop().unwrap().kind, MODIFY);
    queue.push(Ok(event(MODIFY, "/logs/a.log")));
    assert_eq!(drain(&queue).len(), 1);
}

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

#[test]
fn test_queue_pop_waits_for_event() {
    let queue = Arc::new(Queue::new(filter()));
    let popper = thread::spawn({
        let queue = queue.clone();
        move || queue.pop().unwrap().kind
    });
    thread::sleep(Duration::from_millis(50));
    queue.push(Ok(event(REMOVE, "/logs/a.log")));
    assert_eq!(popper.join().unwrap(), REMOVE);
}
