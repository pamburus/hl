// std imports
use std::{
    cmp::min,
    collections::{btree_map::Entry as BTreeEntry, hash_map::Entry, BTreeMap, HashMap},
    convert::{TryFrom, TryInto},
    hash::Hash,
    io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write},
    mem::replace,
    num::{NonZeroU64, NonZeroUsize},
    time::Instant,
};

// third-party imports
use snap::{read::FrameDecoder, write::FrameEncoder};

// local imports
use crate::iox::ReadFill;

// ---

const DEFAULT_SEGMENT_SIZE: Option<NonZeroUsize> = NonZeroUsize::new(256 * 1024);

// ---

type Buf = Vec<u8>;

// ---

pub trait Cache {
    type Key: Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash;

    fn cache<F: FnOnce() -> Result<Buf>>(&mut self, key: Self::Key, f: F) -> Result<&[u8]>;
}

// ---

pub struct ReplayBuf {
    segment_size: NonZeroUsize,
    segments: Vec<CompressedBuf>,
    size: usize,
}

impl ReplayBuf {
    fn new(segment_size: NonZeroUsize) -> Self {
        Self {
            segment_size,
            segments: Vec::new(),
            size: 0,
        }
    }
}

impl TryFrom<ReplayBufCreator> for ReplayBuf {
    type Error = Error;

    fn try_from(builder: ReplayBufCreator) -> Result<Self> {
        builder.result()
    }
}

// ---

pub struct ReplayBufCreator {
    buf: ReplayBuf,
    scratch: ReusableBuf,
}

impl ReplayBufCreator {
    pub fn new() -> Self {
        Self::build().result()
    }

    pub fn build() -> ReplayBufCreatorBuilder {
        ReplayBufCreatorBuilder {
            segment_size: DEFAULT_SEGMENT_SIZE.unwrap(),
        }
    }

    pub fn result(mut self) -> Result<ReplayBuf> {
        self.flush()?;
        Ok(self.buf)
    }

    fn prepare(&mut self) -> Result<()> {
        if self.buf.size % self.buf.segment_size != 0 {
            assert_eq!(self.scratch.len(), 0);
            self.buf.segments.pop().unwrap().decode(self.scratch.backstage())?;
        }
        Ok(())
    }
}

impl Write for ReplayBufCreator {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut k: usize = 0;
        if buf.len() != 0 {
            self.prepare()?;
        }
        while k < buf.len() {
            let buf = &buf[k..];
            let target = self.scratch.backstage();
            let n = min(buf.len(), target.len());
            target[..n].copy_from_slice(&buf[..n]);
            self.scratch.extend(n);
            k += n;
            if self.scratch.full() {
                self.flush()?;
            }
        }
        Ok(k)
    }

    fn flush(&mut self) -> Result<()> {
        if self.scratch.len() != 0 {
            let buf = self.scratch.clear();
            self.buf.segments.push(CompressedBuf::try_from(buf)?);
            self.buf.size += buf.len();
        }
        Ok(())
    }
}

impl From<ReplayBufCreatorBuilder> for ReplayBufCreator {
    fn from(builder: ReplayBufCreatorBuilder) -> Self {
        builder.result()
    }
}

// ---

pub struct ReplayBufCreatorBuilder {
    segment_size: NonZeroUsize,
}

impl ReplayBufCreatorBuilder {
    #[allow(dead_code)]
    pub fn segment_size(mut self, segment_size: NonZeroUsize) -> Self {
        self.segment_size = segment_size;
        self
    }

    pub fn result(self) -> ReplayBufCreator {
        ReplayBufCreator {
            buf: ReplayBuf::new(self.segment_size),
            scratch: ReusableBuf::new(self.segment_size.get()),
        }
    }
}

// ---

pub struct ReplayBufReader<C> {
    buf: ReplayBuf,
    cache: C,
    position: usize,
}

impl ReplayBufReader<MinimalCache<usize>> {
    pub fn new(buf: ReplayBuf) -> Self {
        Self::build(buf).result()
    }

    pub fn build(buf: ReplayBuf) -> ReplayBufReaderBuilder<MinimalCache<usize>> {
        ReplayBufReaderBuilder {
            buf,
            cache: MinimalCache::new(),
            position: 0,
        }
    }
}

impl<C: Cache<Key = usize>> ReplayBufReader<C> {
    #[inline(always)]
    fn segment_size(&self) -> NonZeroUsize {
        self.buf.segment_size
    }

    fn segment(&mut self, index: usize) -> Result<&[u8]> {
        if index >= self.buf.segments.len() {
            panic!("logic error")
        }
        let ss = self.segment_size().get();
        let data = &mut self.buf.segments;
        self.cache.cache(index, || {
            let mut buf = vec![0; ss];
            data[index].decode(&mut buf)?;
            Ok(buf)
        })
    }

    fn from_start(&self, offset: u64) -> Option<usize> {
        usize::try_from(offset).ok().filter(|&v| v <= self.buf.size)
    }

    fn from_current(&self, offset: i64) -> Option<usize> {
        usize::try_from(i64::try_from(self.position).ok()?.checked_add(offset)?)
            .ok()
            .filter(|&v| v <= self.buf.size)
    }

    fn from_end(&mut self, offset: i64) -> Option<usize> {
        usize::try_from(i64::try_from(self.buf.size).ok()?.checked_add(offset)?)
            .ok()
            .filter(|&v| v <= self.buf.size)
    }
}

impl<C: Cache<Key = usize>> Read for ReplayBufReader<C> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut i = 0;
        let ss = self.segment_size().get();
        loop {
            let segment = self.position / self.segment_size();
            let offset = self.position % self.segment_size();
            let data = self.segment(segment)?;
            let k = data.len();
            let n = min(buf.len() - i, data.len() - offset);
            buf[i..i + n].copy_from_slice(&data[offset..offset + n]);
            i += n;
            self.position += n;
            if k != ss || i == buf.len() {
                return Ok(i);
            }
        }
    }
}

impl<C: Cache<Key = usize>> Seek for ReplayBufReader<C> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let pos = match pos {
            SeekFrom::Start(pos) => self.from_start(pos),
            SeekFrom::Current(pos) => self.from_current(pos),
            SeekFrom::End(pos) => self.from_end(pos),
        };
        let pos = pos.ok_or_else(|| Error::new(ErrorKind::InvalidInput, "position out of range"))?;
        let pos = min(pos, self.buf.size);
        self.position = pos;
        u64::try_from(pos).map_err(|e| Error::new(ErrorKind::InvalidInput, e))
    }
}

impl<C: Cache> From<ReplayBufReaderBuilder<C>> for ReplayBufReader<C> {
    fn from(builder: ReplayBufReaderBuilder<C>) -> Self {
        builder.result()
    }
}

// ---

pub struct ReplayBufReaderBuilder<C> {
    buf: ReplayBuf,
    cache: C,
    position: usize,
}

impl<C: Cache> ReplayBufReaderBuilder<C> {
    #[allow(dead_code)]
    pub fn cache<C2: Cache>(self, cache: C2) -> ReplayBufReaderBuilder<C2> {
        ReplayBufReaderBuilder {
            buf: self.buf,
            cache,
            position: self.position,
        }
    }

    #[allow(dead_code)]
    pub fn position(mut self, position: usize) -> Self {
        self.position = position;
        self
    }

    pub fn result(self) -> ReplayBufReader<C> {
        ReplayBufReader {
            buf: self.buf,
            cache: self.cache,
            position: self.position,
        }
    }
}

// ---

#[derive(Default)]
pub struct CompressedBuf(Vec<u8>);

impl CompressedBuf {
    pub fn new(data: &[u8]) -> Result<Self> {
        let mut encoded = Vec::new();
        FrameEncoder::new(&mut encoded).write_all(data)?;
        Ok(Self(encoded))
    }

    pub fn decode(&self, buf: &mut [u8]) -> Result<()> {
        FrameDecoder::new(&self.0[..]).read_exact(buf)
    }
}

impl TryFrom<&[u8]> for CompressedBuf {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self> {
        Self::new(data)
    }
}

impl TryInto<Buf> for &CompressedBuf {
    type Error = Error;

    fn try_into(self) -> Result<Buf> {
        let mut decoded = Buf::new();
        self.decode(&mut decoded)?;
        Ok(decoded)
    }
}

// ---

#[derive(Default)]
struct ReusableBuf {
    buf: Buf,
    len: usize,
}

impl ReusableBuf {
    fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0; capacity],
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    #[allow(dead_code)]
    fn bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    fn backstage(&mut self) -> &mut [u8] {
        &mut self.buf[self.len..]
    }

    fn extend(&mut self, n: usize) {
        self.len += n
    }

    fn full(&self) -> bool {
        self.len == self.buf.len()
    }

    fn clear(&mut self) -> &[u8] {
        self.len = 0;
        self.backstage()
    }

    #[allow(dead_code)]
    fn replace(&mut self, buf: Buf) -> Buf {
        self.len = 0;
        replace(&mut self.buf, buf)
    }
}

// ---

pub struct MinimalCache<Key> {
    data: Option<(Key, Buf)>,
}

impl<Key> MinimalCache<Key> {
    pub fn new() -> Self {
        Self { data: None }
    }
}

impl<Key: Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash> Cache for MinimalCache<Key> {
    type Key = Key;

    fn cache<F: FnOnce() -> Result<Buf>>(&mut self, key: Key, f: F) -> Result<&[u8]> {
        if self.data.as_ref().map(|v| v.0) != Some(key) {
            self.data = Some((key, f()?));
        }
        Ok(&self.data.as_ref().unwrap().1)
    }
}

// ---

pub struct LruCache<Key> {
    limit: usize,
    data: BTreeMap<(Instant, Key), Buf>,
    timestamps: HashMap<Key, Instant>,
}

#[allow(dead_code)]
impl<Key: Ord + PartialOrd> LruCache<Key> {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            data: BTreeMap::new(),
            timestamps: HashMap::new(),
        }
    }
}

impl<Key: Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash> Cache for LruCache<Key> {
    type Key = Key;

    fn cache<F: FnOnce() -> Result<Buf>>(&mut self, key: Key, f: F) -> Result<&[u8]> {
        let now = Instant::now();
        if self.timestamps.len() == self.limit && !self.timestamps.contains_key(&key) {
            if let Some((&(timestamp, i), &_)) = self.data.iter().next() {
                self.timestamps.remove(&i);
                self.data.remove(&(timestamp, i));
            }
        }

        Ok(match self.timestamps.entry(key) {
            Entry::Vacant(e) => {
                e.insert(now);
                match self.data.entry((now, key)) {
                    BTreeEntry::Vacant(e) => e.insert(f()?),
                    BTreeEntry::Occupied(_) => unreachable!(),
                }
            }
            Entry::Occupied(mut e) => {
                let buf = self.data.remove(&(*e.get(), key)).unwrap();
                e.insert(now);
                match self.data.entry((now, key)) {
                    BTreeEntry::Vacant(e) => e.insert(buf),
                    BTreeEntry::Occupied(_) => unreachable!(),
                }
            }
        })
    }
}

// ---

pub trait ReaderFactory {
    type Reader: Read;

    fn new_reader(&self) -> Result<Self::Reader>;
}

impl<R: Read, F: Fn() -> Result<R>> ReaderFactory for F {
    type Reader = R;

    #[inline(always)]
    fn new_reader(&self) -> Result<R> {
        (*self)()
    }
}

// ---

pub struct RewindingReader<F: ReaderFactory, C> {
    factory: F,
    block_size: NonZeroU64,
    cache: C,
    position: u64,
    inner: F::Reader,
    inner_pos: u64,
    size: Option<u64>,
}

impl<F: ReaderFactory> RewindingReader<F, MinimalCache<u64>> {
    #[allow(dead_code)]
    pub fn new(factory: F) -> Result<Self> {
        Self::build(factory).result()
    }

    pub fn build(factory: F) -> RewindingReaderBuilder<F, MinimalCache<u64>> {
        RewindingReaderBuilder {
            factory,
            block_size: DEFAULT_SEGMENT_SIZE.unwrap(),
            cache: MinimalCache::new(),
            position: 0,
        }
    }
}

impl<F: ReaderFactory, C: Cache> RewindingReader<F, C> {
    fn from_start(&self, offset: u64) -> Option<u64> {
        Some(offset)
    }

    fn from_current(&self, offset: i64) -> Option<u64> {
        u64::try_from(i64::try_from(self.position).ok()?.checked_add(offset)?).ok()
    }

    fn from_end(&mut self, end: u64, offset: i64) -> Option<u64> {
        u64::try_from(i64::try_from(end).ok()?.checked_add(offset)?).ok()
    }
}

impl<F: ReaderFactory, C: Cache<Key = u64>> Read for RewindingReader<F, C> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut i = 0;
        let inner = &mut self.inner;
        let mut inner_pos = self.inner_pos;
        let bs = self.block_size.get();
        let factory = &self.factory;
        let mut found_end = false;
        while i < buf.len() && self.position < self.size.unwrap_or(u64::MAX) {
            let block = self.position / self.block_size;
            let block = self.cache.cache(block, || {
                if block * bs < inner_pos {
                    *inner = factory.new_reader()?;
                    inner_pos = 0;
                }
                if block * bs > inner_pos {
                    let n = block * bs - inner_pos;
                    let n = std::io::copy(&mut inner.take(n), &mut std::io::sink())?;
                    inner_pos += n;
                }
                let mut data = vec![0; bs as usize];
                let k = inner.read_fill(&mut data)?;
                if k != data.len() {
                    found_end = true;
                }
                data.resize(k, 0);
                inner_pos += u64::try_from(k).unwrap();
                Ok(data)
            })?;
            let offset = (self.position % self.block_size) as usize;
            let src = &block[offset..];
            let dst = &mut buf[i..];
            let n = min(dst.len(), src.len());
            dst[..n].copy_from_slice(&src[..n]);
            i += n;
            self.position += u64::try_from(n).unwrap();
            if found_end {
                self.size = Some(inner_pos);
            }
        }
        self.inner_pos = inner_pos;
        Ok(i)
    }
}

impl<F: ReaderFactory, C: Cache<Key = u64>> Seek for RewindingReader<F, C> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let pos = match pos {
            SeekFrom::Start(pos) => self.from_start(pos),
            SeekFrom::Current(pos) => self.from_current(pos),
            SeekFrom::End(pos) => {
                let end = if let Some(end) = self.size {
                    end
                } else {
                    let end = self.inner_pos + std::io::copy(&mut self.inner, &mut std::io::sink())?;
                    self.size = Some(end);
                    self.inner_pos = end;
                    end
                };
                self.from_end(end, pos)
            }
        };
        let pos = pos.ok_or_else(|| Error::new(ErrorKind::InvalidInput, "position out of range"))?;
        self.position = pos;
        Ok(pos)
    }
}

// ---

pub struct RewindingReaderBuilder<F, C> {
    factory: F,
    block_size: NonZeroUsize,
    cache: C,
    position: u64,
}

impl<F: ReaderFactory, C: Cache> RewindingReaderBuilder<F, C> {
    #[allow(dead_code)]
    pub fn block_size(mut self, block_size: NonZeroUsize) -> Self {
        self.block_size = block_size;
        self
    }

    #[allow(dead_code)]
    pub fn cache<C2: Cache>(self, cache: C2) -> RewindingReaderBuilder<F, C2> {
        RewindingReaderBuilder {
            factory: self.factory,
            block_size: self.block_size,
            cache,
            position: self.position,
        }
    }

    #[allow(dead_code)]
    pub fn position(mut self, position: u64) -> Self {
        self.position = position;
        self
    }

    pub fn result(self) -> Result<RewindingReader<F, C>> {
        Ok(RewindingReader {
            inner: self.factory.new_reader()?,
            factory: self.factory,
            block_size: self.block_size.try_into().unwrap(),
            cache: self.cache,
            position: self.position,
            inner_pos: 0,
            size: None,
        })
    }
}

// ---

#[allow(dead_code)]
trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, str};

    fn dual<'a>(b: &[u8]) -> (&str, &[u8]) {
        (str::from_utf8(b).unwrap(), b)
    }

    fn test_rewinding_reader<F: FnOnce(usize, &str) -> Box<dyn ReadSeek>>(f: F) {
        let mut r = f(4, "Lorem ipsum dolor sit amet.");

        let mut buf3 = vec![0; 3];
        assert_eq!(r.read(&mut buf3).unwrap(), 3);
        assert_eq!(dual(&buf3), dual("Lor".as_bytes()));

        let mut buf4 = vec![0; 4];
        assert_eq!(r.read(&mut buf4).unwrap(), 4);
        assert_eq!(dual(&buf4), dual("em i".as_bytes()));

        let mut buf6 = vec![0; 6];
        assert_eq!(r.read(&mut buf6).unwrap(), 6);
        assert_eq!(dual(&buf6), dual("psum d".as_bytes()));

        assert_eq!(r.seek(SeekFrom::Start(1)).unwrap(), 1);

        assert_eq!(r.read(&mut buf4).unwrap(), 4);
        assert_eq!(dual(&buf4), dual("orem".as_bytes()));

        assert_eq!(r.seek(SeekFrom::Current(7)).unwrap(), 12);

        let mut buf5 = vec![0; 5];
        assert_eq!(r.read(&mut buf5).unwrap(), 5);
        assert_eq!(dual(&buf5), dual("dolor".as_bytes()));

        assert_eq!(r.seek(SeekFrom::End(-5)).unwrap(), 22);

        assert_eq!(r.read(&mut buf4).unwrap(), 4);
        assert_eq!(dual(&buf4), dual("amet".as_bytes()));

        assert_eq!(r.read(&mut buf3).unwrap(), 1);
        assert_eq!(dual(&buf3[..1]), dual(".".as_bytes()));

        assert_eq!(r.read(&mut buf3).unwrap(), 0);
    }

    #[test]
    fn test_rewinding_reader_default() {
        test_rewinding_reader(|block_size, data| {
            let data = data.as_bytes().to_vec();
            Box::new(
                RewindingReader::build(move || Ok(Cursor::new(data.clone())))
                    .block_size(block_size.try_into().unwrap())
                    .result()
                    .unwrap(),
            )
        });
    }

    #[test]
    fn test_rewinding_reader_lru() {
        test_rewinding_reader(|block_size, data| {
            let data = data.as_bytes().to_vec();
            Box::new(
                RewindingReader::build(move || Ok(Cursor::new(data.clone())))
                    .block_size(block_size.try_into().unwrap())
                    .cache(LruCache::new(3))
                    .result()
                    .unwrap(),
            )
        });
    }
}
