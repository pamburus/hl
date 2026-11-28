// std imports
use std::{
    collections::VecDeque,
    mem,
    sync::{Condvar, Mutex},
};

// local imports
use super::{Event, Filter};

// ---

/// Holds the events that come from the watcher until they are handled.
///
/// While the handler is busy, events can come much faster than it takes them, such as one for every write to a file,
/// so only the events accepted by the [`Filter`] are kept, and an event that repeats the last one waiting is dropped,
/// as handling one event reads all that was written before it.
///
/// Events repeat each other when they have the same paths and the same kind, whatever its detail, as different sources
/// can report the same change in more detail or less, such as the watcher and the poller on Windows.
pub(super) struct Queue {
    filter: Filter,
    events: Mutex<VecDeque<notify::Result<Event>>>,
    ready: Condvar,
}

impl Queue {
    pub(super) fn new(filter: Filter) -> Self {
        Self {
            filter,
            events: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
        }
    }

    /// Adds `event`, unless the filter does not accept it or it repeats the last event waiting.
    pub(super) fn push(&self, event: notify::Result<Event>) {
        if let Ok(new) = &event
            && !self.filter.accepts(new)
        {
            return;
        }
        let mut events = self.events.lock().unwrap();
        if let (Ok(new), Some(Ok(last))) = (&event, events.back())
            && mem::discriminant(&new.kind) == mem::discriminant(&last.kind)
            && new.paths == last.paths
        {
            return;
        }
        events.push_back(event);
        self.ready.notify_one();
    }

    /// Removes and returns the first event waiting, waiting for one to come if there is none.
    pub(super) fn pop(&self) -> notify::Result<Event> {
        let mut events = self.events.lock().unwrap();
        loop {
            if let Some(event) = events.pop_front() {
                return event;
            }
            events = self.ready.wait(events).unwrap();
        }
    }
}

#[cfg(test)]
mod tests;
