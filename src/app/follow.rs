//! Stages of the follow mode merger: the window that sorts the entries of all inputs by time, and the writer that shows them.

// std imports
use std::{
    cmp::max,
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
    source_last_ts: HashMap<usize, Timestamp>,
    mem_usage: usize,
    mem_limit: usize,
    sync_interval: Duration,
    delim: &'a [u8],
}

impl<'a> MergeWindow<'a> {
    pub fn new(delim: &'a [u8], mem_limit: usize, sync_interval: Duration) -> Self {
        Self {
            entries: BTreeMap::new(),
            source_last_ts: HashMap::new(),
            mem_usage: 0,
            mem_limit,
            sync_interval,
            delim,
        }
    }

    /// Adds the entries of a block of input `i` processed into `buf`.
    ///
    /// Returns the trailing part of the block that has no timestamp to be sorted by,
    /// as no record of the input came before it, to be written right away.
    pub fn insert(
        &mut self,
        i: usize,
        buf: Vec<u8>,
        index: TimestampIndex,
        now: Instant,
    ) -> Option<(Rc<Vec<u8>>, Range<usize>)> {
        let delim = self.delim;
        let delim_len = delim.len();
        let buf = Rc::new(buf);
        let mut unsorted = None;

        // Iterate over gaps between indexed records and insert them
        // into the window with the timestamp of the next indexed
        // record (or the last known timestamp for the source as
        // fallback for the trailing gap) so they get sorted
        // together with parsed records.
        let gap_starts = std::iter::once(0usize).chain(index.lines.iter().map(|l| l.location.end + delim_len));
        let gap_ends = index
            .lines
            .iter()
            .map(|l| l.location.start)
            .chain(std::iter::once(buf.len()));
        let gap_timestamps = index
            .lines
            .iter()
            .map(|l| Some(l.ts))
            .chain(std::iter::once(self.source_last_ts.get(&i).copied()));

        for ((start, end), ts) in gap_starts.zip(gap_ends).zip(gap_timestamps) {
            if start >= end {
                continue;
            }
            let trimmed = if end >= start + delim_len && buf[end - delim_len..end] == *delim {
                end - delim_len
            } else {
                end
            };
            if start < trimmed {
                if let Some(ts) = ts {
                    self.push(
                        Key::new(ts, i, index.block, start),
                        Entry::new(&buf, start..trimmed, now),
                    );
                } else {
                    unsorted = Some((buf.clone(), start..end));
                }
            }
        }

        // Insert indexed records into the window.
        for record in index.lines {
            let ts = record.ts;
            self.source_last_ts
                .entry(i)
                .and_modify(|prev| *prev = max(*prev, ts))
                .or_insert(ts);
            let key = Key::new(ts, i, index.block, record.location.start);
            self.push(key, Entry::new(&buf, record.location, now));
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
}

#[cfg(test)]
mod tests;
