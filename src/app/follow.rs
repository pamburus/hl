//! Stages of the follow mode merger: the window that sorts the entries of all inputs by time, and the writer that shows them.

// std imports
use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
    rc::Rc,
    time::{Duration, Instant},
};

// local imports
use super::{FollowBadges, Output, TimestampIndex, flush_wait};
use crate::{error::*, index::Timestamp, scanning::PartialPlacement};

// ---

/// Holds the entries received from workers until they are due, and gives them out ordered by time.
pub(super) struct MergeWindow<'a> {
    entries: BTreeMap<Key, Entry>,
    order: BlockOrder,
    /// The parts of split entries, log entries too large to be read at once, that continue them.
    /// They are not sorted, as each of them is written right after the part before it,
    /// but are kept by input and block, as the split entries of an input can be written out of the order they came in.
    parts: HashMap<(usize, usize), Entry>,
    /// The timestamp of the last record of each input.
    last_ts: HashMap<usize, Timestamp>,
    /// A split entry being written: the input and block of its next part, and when its last part was given out.
    /// Until the next part is given out, nothing else is, so that no other entries come in between,
    /// unless the window holds twice its memory limit. The margin over the limit leaves time for
    /// the next part, which may still be processed, to come while the other inputs keep the window full.
    open: Option<(usize, usize, Instant)>,
    /// Whether the split entry of each input was ended before its rest came.
    /// If the next block of such an input is that rest, it is sorted first, so that it is given out as it comes.
    ended: Vec<bool>,
    mem_usage: usize,
    mem_limit: usize,
    sync_interval: Duration,
    delim: &'a [u8],
}

impl<'a> MergeWindow<'a> {
    pub fn new(delim: &'a [u8], mem_limit: usize, sync_interval: Duration) -> Self {
        Self {
            entries: BTreeMap::new(),
            order: BlockOrder::default(),
            parts: HashMap::new(),
            last_ts: HashMap::new(),
            open: None,
            ended: Vec::new(),
            mem_usage: 0,
            mem_limit,
            sync_interval,
            delim,
        }
    }

    /// Adds a block of input `i` processed into `buf`.
    ///
    /// Workers finish blocks in any order, so the blocks of each input are taken in order.
    pub fn insert(&mut self, i: usize, buf: Vec<u8>, index: TimestampIndex, now: Instant) {
        self.order.push(i, buf, index);
        while let Some((buf, index)) = self.order.pop(i) {
            self.insert_block(i, buf, index, now);
        }
    }

    /// Removes and returns what is next to be written if it is due as of `now`, or if `all` entries are due.
    ///
    /// Entries without a timestamp to be sorted by are due right away.
    pub fn pop_due(&mut self, now: Instant, all: bool) -> Option<Due> {
        if let Some((i, block, since)) = self.open {
            if let Some(entry) = self.take_part(i, block, now) {
                return Some(Due::Entry(None, entry));
            }
            let passed = self.order.passed(i, block);
            // A late rest is waited for until something else is due, as nothing can be written before it.
            let late = flush_wait(since, now, self.sync_interval).is_zero() && self.next_entry_wait(now).is_zero();
            let full = self.mem_usage >= self.mem_limit.saturating_mul(2);
            if all || passed || late || full {
                // The rest of the entry will not come, comes too late, or cannot be awaited
                // without holding more than the memory limit, so the entry is ended as it is.
                self.open = None;
                if !all && !passed {
                    if self.ended.len() <= i {
                        self.ended.resize(i + 1, false);
                    }
                    self.ended[i] = true;
                }
                return Some(Due::SplitEnd);
            }
            return None;
        }

        // Only the first entry is ever removed from the sorted entries,
        // which keeps the removal cheap in this common path.
        if !all && !self.next_entry_wait(now).is_zero() {
            return None;
        }
        let (key, entry) = self.entries.pop_first()?;
        self.mem_usage -= entry.location.len();
        if entry.continued() {
            self.open = Some((key.input, key.block + 1, now));
        }
        // Of the parts that continue a split entry, only the rest of one ended before it came is sorted.
        if entry.continues() {
            return Some(Due::Resumed(key.input, entry));
        }
        Some(Due::Entry(key.ts, entry))
    }

    /// Returns how long, as of `now`, until something is due.
    pub fn wait(&self, now: Instant) -> Duration {
        if let Some((_, _, since)) = self.open {
            return flush_wait(since, now, self.sync_interval).max(self.next_entry_wait(now));
        }
        self.next_entry_wait(now)
    }

    /// Returns how long, as of `now`, until the first of the sorted entries is due, or [`Duration::MAX`] if there are none.
    fn next_entry_wait(&self, now: Instant) -> Duration {
        let Some((key, first)) = self.entries.first_key_value() else {
            return Duration::MAX;
        };
        if key.ts.is_none() || self.mem_usage >= self.mem_limit {
            return Duration::ZERO;
        }
        self.wait_for(first, now)
    }

    /// Adds the entries of the next block of input `i`.
    ///
    /// Text that is not a record is sorted together with the record before it in its input,
    /// so that, for example, a stack trace stays right after the record it belongs to.
    /// So are the parts of a log entry too large to be read at once.
    fn insert_block(&mut self, i: usize, buf: Vec<u8>, index: TimestampIndex, now: Instant) {
        let buf = Rc::new(buf);
        let mut ts = self.last_ts.get(&i).copied();

        // Whatever the next block of the input is, it settles whether the rest of its ended split entry came.
        let ended = self.ended.get_mut(i).is_some_and(std::mem::take);

        if let Some(part) = index.part {
            let entry = Entry::new(&buf, 0..buf.len(), now, Some(part));
            if entry.continues() && ended {
                // Without a timestamp, the rest is sorted first and is due right away.
                // An empty rest adds nothing to the entry already ended, so it is dropped.
                if !entry.data().is_empty() {
                    self.push(Key::new(None, i, index.block, 0), entry);
                }
            } else if entry.continues() {
                self.push_part(i, index.block, entry);
            } else {
                self.push(Key::new(ts, i, index.block, 0), entry);
            }
            return;
        }

        let mut start = 0;
        for record in index.lines {
            self.push_text(ts, i, index.block, &buf, start..record.location.start, now);
            start = record.location.end + self.delim.len();
            ts = Some(record.ts);
            let key = Key::new(ts, i, index.block, record.location.start);
            self.push(key, Entry::new(&buf, record.location, now, None));
        }
        self.push_text(ts, i, index.block, &buf, start..buf.len(), now);

        if let Some(ts) = ts {
            self.last_ts.insert(i, ts);
        }
    }

    /// Gives out the next part of a split entry, which is in block `block` of input `i`, if it came,
    /// and keeps the split entry open if the part is not its last.
    fn take_part(&mut self, i: usize, block: usize, now: Instant) -> Option<Entry> {
        let entry = self.parts.remove(&(i, block))?;
        self.mem_usage -= entry.location.len();
        self.open = entry.continued().then_some((i, block + 1, now));
        Some(entry)
    }

    fn push_part(&mut self, i: usize, block: usize, entry: Entry) {
        self.mem_usage += entry.location.len();
        self.parts.insert((i, block), entry);
    }

    fn wait_for(&self, entry: &Entry, now: Instant) -> Duration {
        flush_wait(entry.received, now, self.sync_interval)
    }

    /// Adds text at `location` in `buf`, sorted by `ts` of the record before it, if any.
    fn push_text(
        &mut self,
        ts: Option<Timestamp>,
        i: usize,
        block: usize,
        buf: &Rc<Vec<u8>>,
        location: Range<usize>,
        now: Instant,
    ) {
        let Range { start, end } = location;
        if start >= end {
            return;
        }
        let trimmed = if buf[start..end].ends_with(self.delim) {
            end - self.delim.len()
        } else {
            end
        };
        if start >= trimmed {
            return;
        }
        self.push(
            Key::new(ts, i, block, start),
            Entry::new(buf, start..trimmed, now, None),
        );
    }

    fn push(&mut self, key: Key, entry: Entry) {
        self.mem_usage += entry.location.len();
        self.entries.insert(key, entry);
    }
}

// ---

/// What the window gives out next.
pub(super) enum Due {
    /// An entry to be written, with the timestamp it was sorted by, if any.
    Entry(Option<Timestamp>, Entry),
    /// The next part of a split entry of input `i` ended before it came, which starts a new line.
    Resumed(usize, Entry),
    /// The end of a split entry, whose rest will not come.
    SplitEnd,
}

// ---

/// Takes the blocks of each input in order.
#[derive(Default)]
struct BlockOrder {
    inputs: Vec<InputBlocks>,
}

impl BlockOrder {
    fn push(&mut self, i: usize, buf: Vec<u8>, index: TimestampIndex) {
        if self.inputs.len() <= i {
            self.inputs.resize_with(i + 1, Default::default);
        }
        self.inputs[i].early.insert(index.block, (buf, index));
    }

    /// Removes and returns the next block of input `i` if it came.
    fn pop(&mut self, i: usize) -> Option<(Vec<u8>, TimestampIndex)> {
        let input = self.inputs.get_mut(i)?;
        let block = input.early.remove(&input.next)?;
        input.next += 1;
        Some(block)
    }

    /// Returns whether block `block` of input `i` was already given out.
    fn passed(&self, i: usize, block: usize) -> bool {
        self.inputs.get(i).is_some_and(|input| input.next > block)
    }
}

/// The blocks of an input: the number of the next block expected, and the blocks that came before it.
#[derive(Default)]
struct InputBlocks {
    next: usize,
    early: HashMap<usize, (Vec<u8>, TimestampIndex)>,
}

// ---

/// Sorts entries in the window: by time, then by their position in the inputs.
///
/// Entries without a timestamp to be sorted by, as no record of their input came before them, come first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    ts: Option<Timestamp>,
    input: usize,
    block: usize,
    offset: usize,
}

impl Key {
    fn new(ts: Option<Timestamp>, input: usize, block: usize, offset: usize) -> Self {
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
/// It can also be a part of a split entry, a log entry too large to be read at once.
///
/// It is stored as where it is in its block's buffer, without the final delimiter, and when it was received.
pub(super) struct Entry {
    buf: Rc<Vec<u8>>,
    location: Range<usize>,
    received: Instant,
    part: Option<PartialPlacement>,
}

impl Entry {
    fn new(buf: &Rc<Vec<u8>>, location: Range<usize>, received: Instant, part: Option<PartialPlacement>) -> Self {
        Self {
            buf: buf.clone(),
            location,
            received,
            part,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.buf[self.location.clone()]
    }

    /// Returns whether the entry continues a split entry.
    fn continues(&self) -> bool {
        matches!(self.part, Some(PartialPlacement::Next | PartialPlacement::Last))
    }

    /// Returns whether the split entry the entry is a part of is continued by the next part.
    fn continued(&self) -> bool {
        matches!(self.part, Some(PartialPlacement::First | PartialPlacement::Next))
    }
}

// ---

/// Writes entries given out by the window, each marked by whether it is in time order with the entry before.
pub(super) struct MergeWriter<'a> {
    badges: &'a FollowBadges,
    delim: &'a [u8],
    prev_ts: Option<Timestamp>,
}

impl<'a> MergeWriter<'a> {
    pub fn new(badges: &'a FollowBadges, delim: &'a [u8]) -> Self {
        Self {
            badges,
            delim,
            prev_ts: None,
        }
    }

    /// Writes `entry` with timestamp `ts`.
    ///
    /// An entry starts with a placeholder for the sync indicator, unless it continues a split entry.
    /// The placeholder is kept for an entry without a timestamp. A split entry is ended
    /// after its last part, or by [`MergeWriter::end_split`].
    pub fn write(&mut self, output: &mut Output, ts: Option<Timestamp>, entry: &Entry) -> Result<()> {
        let data = entry.data();
        let si = &self.badges.si;
        match ts {
            Some(ts) if !entry.continues() => {
                let sync_indicator = if self.prev_ts.map(|prev| prev <= ts).unwrap_or(true) {
                    &si.synced
                } else {
                    &si.failed
                };
                self.prev_ts = Some(ts);
                output.write_all(sync_indicator.as_bytes())?;
                output.write_all(&data[si.width..])?;
            }
            _ => output.write_all(data)?,
        }
        self.end(output, entry)
    }

    /// Writes `entry`, the next part of a split entry of input `i` ended before it came,
    /// on a new line that starts like other entries of the input.
    pub fn resume(&mut self, output: &mut Output, i: usize, entry: &Entry) -> Result<()> {
        output.write_all(self.badges.prefix(i).as_bytes())?;
        output.write_all(entry.data())?;
        self.end(output, entry)
    }

    /// Ends a split entry whose rest will not come.
    pub fn end_split(&mut self, output: &mut Output) -> Result<()> {
        output.write_all(self.delim)?;
        Ok(())
    }

    /// Ends the line of `entry`, unless the split entry it is a part of is continued.
    fn end(&self, output: &mut Output, entry: &Entry) -> Result<()> {
        if !entry.continued() {
            output.write_all(self.delim)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
