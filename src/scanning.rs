use std::io::Read;
use std::sync::Arc;

use crossbeam_queue::{PopError, SegQueue};

use crate::error::*;

pub struct Segment {
    data: Vec<u8>,
    size: usize,
}

pub enum ScannedSegment {
    Complete(Segment),
    Incomplete(Segment),
}

pub struct Scanner {
    delimiter: String,
    sf: Arc<SegmentFactory>,
}

pub struct SegmentFactory {
    buf_size: usize,
    recycled: SegQueue<Segment>,
}

impl SegmentFactory {
    pub fn new(buf_size: usize) -> Self {
        return Self {
            buf_size,
            recycled: SegQueue::new(),
        };
    }

    pub fn new_segment(&self) -> Segment {
        match self.recycled.pop() {
            Ok(segment) => segment.resetted(),
            Err(PopError) => Segment::new(self.buf_size),
        }
    }

    pub fn recycle(&self, segment: Segment) {
        self.recycled.push(segment);
    }
}

pub struct BufFactory {
    buf_size: usize,
    recycled: SegQueue<Vec<u8>>,
}

impl BufFactory {
    pub fn new(buf_size: usize) -> Self {
        return Self {
            buf_size,
            recycled: SegQueue::new(),
        };
    }

    pub fn new_buf(&self) -> Vec<u8> {
        match self.recycled.pop() {
            Ok(mut buf) => {
                buf.resize(0, 0);
                buf
            }
            Err(PopError) => Vec::with_capacity(self.buf_size),
        }
    }

    pub fn recycle(&self, buf: Vec<u8>) {
        self.recycled.push(buf);
    }
}

impl ScannedSegment {
    fn new(segment: Segment, partial: bool) -> Self {
        if partial {
            Self::Incomplete(segment)
        } else {
            Self::Complete(segment)
        }
    }
}

impl Segment {
    fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        data.resize(capacity, 0);
        Self { data, size: 0 }
    }

    fn zero() -> Self {
        Self {
            data: Vec::new(),
            size: 0,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.size]
    }

    pub fn to_vec(mut self) -> Vec<u8> {
        self.data.resize(self.size, 0);
        self.data
    }

    fn reset(&mut self) {
        self.data.resize(self.data.capacity(), 0);
        self.size = 0;
    }

    fn resetted(mut self) -> Self {
        self.reset();
        self
    }

    fn replace(&mut self, mut other: Self) -> Self {
        std::mem::swap(self, &mut other);
        other
    }
}

impl Scanner {
    pub fn new(sf: Arc<SegmentFactory>, delimiter: String) -> Self {
        Self {
            delimiter: delimiter.clone(),
            sf,
        }
    }

    pub fn items<'a, 'b>(&'a self, input: &'b mut dyn Read) -> ScannerIter<'a, 'b> {
        return ScannerIter::new(self, input);
    }
}

pub struct ScannerIter<'a, 'b> {
    scanner: &'a Scanner,
    input: &'b mut dyn Read,
    next: Segment,
    partial: bool,
    done: bool,
}

impl<'a, 'b> ScannerIter<'a, 'b> {
    fn new(scanner: &'a Scanner, input: &'b mut dyn Read) -> Self {
        return Self {
            scanner,
            input,
            next: scanner.sf.new_segment(),
            partial: false,
            done: false,
        };
    }

    fn split(&mut self) -> Option<Segment> {
        let k = self.scanner.delimiter.len();
        if self.next.size < k || k == 0 {
            return None;
        }

        for i in (0..self.next.size - k + 1).rev() {
            if self.next.data[i..].starts_with(self.scanner.delimiter.as_bytes()) {
                let n = self.next.size - i - k;
                let mut result = self.scanner.sf.new_segment();
                if result.data.len() < n {
                    result.data.resize(n, 0);
                }
                if n > 0 {
                    result.data[..n].copy_from_slice(&self.next.data[i + k..i + k + n]);
                    result.size = n;
                    self.next.size -= n;
                }
                return Some(result);
            }
        }
        None
    }
}

impl<'a, 'b> Iterator for ScannerIter<'a, 'b> {
    type Item = Result<ScannedSegment>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            let n = match self.input.read(&mut self.next.data[self.next.size..]) {
                Ok(value) => value,
                Err(err) => {
                    self.done = true;
                    return Some(Err(err.into()));
                }
            };
            self.next.size += n;
            let full = self.next.size == self.next.data.capacity();

            let (next, partial) = if n == 0 {
                self.done = true;
                (Segment::zero(), self.partial)
            } else {
                match self.split() {
                    Some(next) => {
                        let result = (next, self.partial);
                        self.partial = false;
                        result
                    }
                    None => {
                        if !full {
                            continue;
                        }
                        self.partial = true;
                        (self.scanner.sf.new_segment(), true)
                    }
                }
            };

            return Some(Ok(ScannedSegment::new(self.next.replace(next), partial)));
        }
    }
}
