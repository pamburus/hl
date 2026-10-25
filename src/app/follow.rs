//! Stages of the follow mode merger: the window that sorts the entries of all inputs by time, and the writer that shows them.

// std imports
use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
    rc::Rc,
    time::{Duration, Instant},
};

// local imports
use super::{Output, SyncIndicator, TimestampIndex, flush_wait};
use crate::{error::*, index::Timestamp};

// ---

/// Holds the entries received from workers until they are due, and gives them out ordered by time.
pub(super) struct MergeWindow<'a> {
    entries: BTreeMap<Key, Entry>,
    /// The timestamp of the last record of each input.
    last_ts: HashMap<usize, Timestamp>,
    mem_usage: usize,
    mem_limit: usize,
    sync_interval: Duration,
    delim: &'a [u8],
}

impl<'a> MergeWindow<'a> {
    pub fn new(delim: &'a [u8], mem_limit: usize, sync_interval: Duration) -> Self {
        Self {
            entries: BTreeMap::new(),
            last_ts: HashMap::new(),
            mem_usage: 0,
            mem_limit,
            sync_interval,
            delim,
        }
    }

    /// Adds the entries of a block of input `i` processed into `buf`.
    ///
    /// Text that is not a record is sorted together with the record before it in its input,
    /// so that, for example, a stack trace stays right after the record it belongs to.
    ///
    /// Returns the leading part of the block that has no record before it to be sorted with,
    /// as no record of the input came yet, to be written right away.
    pub fn insert(
        &mut self,
        i: usize,
        buf: Vec<u8>,
        index: TimestampIndex,
        now: Instant,
    ) -> Option<(Rc<Vec<u8>>, Range<usize>)> {
        let buf = Rc::new(buf);
        let mut ts = self.last_ts.get(&i).copied();
        let mut start = 0;
        let mut unsorted = None;

        for record in index.lines {
            let text = self.push_text(ts, i, index.block, &buf, start..record.location.start, now);
            unsorted = unsorted.or(text);
            start = record.location.end + self.delim.len();
            ts = Some(record.ts);
            let key = Key::new(record.ts, i, index.block, record.location.start);
            self.push(key, Entry::new(&buf, record.location, now));
        }
        let text = self.push_text(ts, i, index.block, &buf, start..buf.len(), now);
        unsorted = unsorted.or(text);

        if let Some(ts) = ts {
            self.last_ts.insert(i, ts);
        }

        unsorted
    }

    /// Removes and returns the earliest entry if it is due as of `now`, or if `all` entries are due.
    pub fn pop_due(&mut self, now: Instant, all: bool) -> Option<(Key, Entry)> {
        let (_, first) = self.entries.first_key_value()?;
        if !all && self.mem_usage < self.mem_limit && !self.wait_for(first, now).is_zero() {
            return None;
        }
        let (key, entry) = self.entries.pop_first()?;
        self.mem_usage -= entry.location.len();
        Some((key, entry))
    }

    /// Returns how long, as of `now`, until the earliest entry is due.
    pub fn wait(&self, now: Instant) -> Duration {
        self.entries
            .first_key_value()
            .map(|(_, first)| self.wait_for(first, now))
            .unwrap_or(Duration::MAX)
    }

    fn wait_for(&self, entry: &Entry, now: Instant) -> Duration {
        flush_wait(entry.received, now, self.sync_interval)
    }

    /// Adds text at `location` in `buf`, sorted by `ts` of the record before it, if any.
    ///
    /// Returns the text if there is no such record.
    fn push_text(
        &mut self,
        ts: Option<Timestamp>,
        i: usize,
        block: usize,
        buf: &Rc<Vec<u8>>,
        location: Range<usize>,
        now: Instant,
    ) -> Option<(Rc<Vec<u8>>, Range<usize>)> {
        let Range { start, end } = location;
        if start >= end {
            return None;
        }
        let trimmed = if buf[start..end].ends_with(self.delim) {
            end - self.delim.len()
        } else {
            end
        };
        if start >= trimmed {
            return None;
        }
        let Some(ts) = ts else {
            return Some((buf.clone(), start..trimmed));
        };
        self.push(Key::new(ts, i, block, start), Entry::new(buf, start..trimmed, now));
        None
    }

    fn push(&mut self, key: Key, entry: Entry) {
        self.mem_usage += entry.location.len();
        self.entries.insert(key, entry);
    }
}

// ---

/// Sorts entries in the window: by time, then by their position in the inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Key {
    pub ts: Timestamp,
    input: usize,
    block: usize,
    offset: usize,
}

impl Key {
    fn new(ts: Timestamp, input: usize, block: usize, offset: usize) -> Self {
        Self {
            ts,
            input,
            block,
            offset,
        }
    }
}

// ---

/// A unit of output sorted as a whole in the window: a formatted record, or a run of text next to
/// records that is not a record itself. Either can span several lines.
///
/// It is stored as where it is in its block's buffer, without the final delimiter, and when it was received.
pub(super) struct Entry {
    buf: Rc<Vec<u8>>,
    location: Range<usize>,
    received: Instant,
}

impl Entry {
    fn new(buf: &Rc<Vec<u8>>, location: Range<usize>, received: Instant) -> Self {
        Self {
            buf: buf.clone(),
            location,
            received,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.buf[self.location.clone()]
    }
}

// ---

/// Writes entries given out by the window, each marked by whether it is in time order with the entry before.
pub(super) struct MergeWriter<'a> {
    si: &'a SyncIndicator,
    delim: &'a [u8],
    prev_ts: Option<Timestamp>,
}

impl<'a> MergeWriter<'a> {
    pub fn new(si: &'a SyncIndicator, delim: &'a [u8]) -> Self {
        Self {
            si,
            delim,
            prev_ts: None,
        }
    }

    /// Writes `entry` with timestamp `ts`, which starts with a placeholder for the sync indicator.
    pub fn write(&mut self, output: &mut Output, ts: Timestamp, entry: &[u8]) -> Result<()> {
        let sync_indicator = if self.prev_ts.map(|prev| prev <= ts).unwrap_or(true) {
            &self.si.synced
        } else {
            &self.si.failed
        };
        self.prev_ts = Some(ts);
        output.write_all(sync_indicator.as_bytes())?;
        output.write_all(&entry[self.si.width..])?;
        output.write_all(self.delim)?;
        Ok(())
    }

    /// Writes `text` that has no timestamp to be sorted by, as is.
    pub fn write_unsorted(&mut self, output: &mut Output, text: &[u8]) -> Result<()> {
        output.write_all(text)?;
        output.write_all(self.delim)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
