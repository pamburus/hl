// std imports
use std::{
    cmp::min,
    convert::TryInto,
    fs::{self, File, Metadata},
    io::{self, stdin, BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    mem::size_of_val,
    ops::{Deref, Range},
    path::PathBuf,
    sync::{Arc, Mutex},
};

// third-party imports
use deko::{bufread::AnyDecoder, Format};
use nu_ansi_term::Color;

// local imports
use crate::{
    error::{Result, HILITE},
    index::{FileSystem, Index, Indexer, SourceBlock},
    iox::ReadFill,
    pool::SQPool,
    replay::{ReplayBufCreator, ReplayBufReader, ReplaySeekReader},
    tee::TeeReader,
};

// ---

pub type SequentialStream = Box<dyn ReadMeta + Send + Sync>;
pub type RandomAccessStream = Box<dyn ReadSeekMeta + Send + Sync>;
pub type BufPool = SQPool<Vec<u8>>;

// ---

/// The path to an input file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InputPath {
    pub original: PathBuf,
    pub canonical: PathBuf,
}

impl InputPath {
    /// Resolves the canonical path for the given path.
    pub fn new(original: PathBuf) -> io::Result<Self> {
        let canonical = fs::canonicalize(&original).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "failed to resolve path for '{}': {}",
                    HILITE.paint(original.to_string_lossy()),
                    e
                ),
            )
        })?;

        Ok(Self { original, canonical })
    }

    /// Creates an ephemeral path.
    pub fn ephemeral(original: PathBuf) -> Self {
        Self {
            original: original.clone(),
            canonical: original,
        }
    }
}

impl TryFrom<PathBuf> for InputPath {
    type Error = io::Error;

    fn try_from(original: PathBuf) -> io::Result<Self> {
        Self::new(original)
    }
}

// ---

/// A reference to an input file or stdin.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InputReference {
    Stdin,
    File(InputPath),
}

impl InputReference {
    /// Preliminarily opens the input file to ensure it exists and is readable
    /// and protect it from being suddenly deleted while we need it.
    pub fn hold(&self) -> io::Result<InputHolder> {
        let (reference, stream): (_, Option<Box<dyn ReadSeekMeta + Send + Sync>>) = match self {
            Self::Stdin => (self.clone(), None),
            Self::File(path) => {
                let meta = fs::metadata(&path.canonical).map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!("failed to get information on {}: {}", self.description(), e),
                    )
                })?;
                if meta.is_dir() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("{} is a directory", self.description()),
                    ));
                }
                let stream =
                    Box::new(File::open(&path.canonical).map_err(|e| {
                        io::Error::new(e.kind(), format!("failed to open {}: {}", self.description(), e))
                    })?);

                (InputReference::File(path.clone()), Some(stream))
            }
        };

        Ok(InputHolder::new(reference, stream))
    }

    /// Completely opens the input for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(&self) -> io::Result<Input> {
        self.hold()?.open()
    }

    /// Returns a description of the input reference.
    pub fn description(&self) -> String {
        match self {
            Self::Stdin => "<stdin>".into(),
            Self::File(path) => format!("file '{}'", Color::Yellow.paint(path.original.to_string_lossy())),
        }
    }

    #[inline]
    fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::Stdin => None,
            Self::File(path) => Some(&path.canonical),
        }
    }
}

// ---

/// Meta information about the input.
pub trait Meta {
    fn metadata(&self) -> io::Result<Option<Metadata>>;
}

impl<T: Meta> Meta for &T {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        (*self).metadata()
    }
}

impl<T: Meta> Meta for &mut T {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        (**self).metadata()
    }
}

impl Meta for fs::File {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        self.metadata().map(Some)
    }
}

impl Meta for io::Stdin {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        Ok(None)
    }
}

impl<T> Meta for Cursor<T> {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        Ok(None)
    }
}

impl<T: Meta> Meta for Mutex<T> {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        self.lock().unwrap().metadata()
    }
}

// ---

/// A holder of an input file.
/// It can be used to ensure the input file is not suddenly deleting while it is needed.
pub struct InputHolder {
    pub reference: InputReference,
    pub stream: Option<Box<dyn ReadSeekMeta + Send + Sync>>,
}

impl InputHolder {
    /// Creates a new input holder.
    pub fn new(reference: InputReference, stream: Option<Box<dyn ReadSeekMeta + Send + Sync>>) -> Self {
        Self { reference, stream }
    }

    /// Opens the input file for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(self) -> io::Result<Input> {
        Ok(Input::new(self.reference.clone(), self.stream()?))
    }

    /// Indexes the input file and returns IndexedInput that can be used to access the data in random order.
    pub fn index<FS: FileSystem + Sync>(self, indexer: &Indexer<FS>) -> Result<IndexedInput> {
        self.open()?.indexed(indexer)
    }

    fn stream(self) -> io::Result<Stream> {
        Ok(match &self.reference {
            InputReference::Stdin => match self.stream {
                Some(stream) => Stream::Sequential(Stream::RandomAccess(stream).into_sequential()),
                None => Stream::Sequential(self.stdin()),
            },
            InputReference::File(_) => match self.stream {
                Some(stream) => Stream::RandomAccess(stream),
                None => Stream::RandomAccess(self.reference.hold()?.stream.unwrap()),
            },
        })
    }

    fn stdin(self) -> SequentialStream {
        self.stream
            .map(|s| Box::new(StreamOver(s)) as SequentialStream)
            .unwrap_or_else(|| Box::new(stdin()))
    }
}

/// Represents already opened and decoded input file or stdin.
pub struct Input {
    pub reference: InputReference,
    pub stream: Stream,
}

impl Input {
    fn new(reference: InputReference, stream: Stream) -> Self {
        Self {
            reference: reference.clone(),
            stream: stream.verified().decoded().tagged(reference),
        }
    }

    /// Indexes the input file and returns IndexedInput that can be used to access the data in random order.
    pub fn indexed<FS: FileSystem + Sync>(self, indexer: &Indexer<FS>) -> Result<IndexedInput> {
        IndexedInput::from_stream(self.reference, self.stream, indexer)
    }

    /// Opens the file for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(path: &PathBuf) -> io::Result<Self> {
        InputReference::File(path.clone().try_into()?).open()
    }

    /// Opens the stdin for reading.
    pub fn stdin() -> io::Result<Self> {
        InputReference::Stdin.open()
    }

    pub fn tail(mut self, lines: u64) -> io::Result<Self> {
        match &mut self.stream {
            Stream::Sequential(_) => (),
            Stream::RandomAccess(stream) => Self::seek_tail(stream, lines)?,
        }
        Ok(self)
    }

    fn seek_tail(stream: &mut RandomAccessStream, lines: u64) -> io::Result<()> {
        const BUF_SIZE: usize = 64 * 1024;
        let mut scratch = [0; BUF_SIZE];
        let mut count: u64 = 0;
        let mut pos = stream.seek(SeekFrom::End(0))?;
        while pos != 0 {
            let n = min(BUF_SIZE as u64, pos);
            pos -= n;
            pos = stream.seek(SeekFrom::Start(pos))?;
            let bn = n as usize;
            let buf = scratch[..bn].as_mut();

            stream.read_exact(buf)?;

            for i in (0..bn).rev() {
                if buf[i] == b'\n' {
                    if count == lines {
                        stream.seek(SeekFrom::Start(pos + i as u64 + 1))?;
                        return Ok(());
                    }
                    count += 1;
                }
            }
        }
        stream.seek(SeekFrom::Start(pos as u64))?;
        Ok(())
    }
}

// ---

/// Stream of input data.
/// It can be either sequential or supporing random access.
pub enum Stream {
    Sequential(SequentialStream),
    RandomAccess(RandomAccessStream),
}

impl Stream {
    /// Verifies if the stream supports random access.
    /// If not, converts it to a sequential stream.
    pub fn verified(self) -> Self {
        match self {
            Self::Sequential(stream) => Self::Sequential(stream),
            Self::RandomAccess(stream) => {
                let mut stream = stream;
                if stream.seek(SeekFrom::Current(0)).is_err() {
                    Self::Sequential(Box::new(stream))
                } else {
                    Self::RandomAccess(stream)
                }
            }
        }
    }

    /// Decodes the stream if needed.
    pub fn decoded(self) -> Self {
        match self {
            Self::Sequential(stream) => {
                let meta = stream.metadata().ok().flatten();
                Self::Sequential(Box::new(AnyDecoder::new(BufReader::new(stream)).with_metadata(meta)))
            }
            Self::RandomAccess(mut stream) => {
                if let Some(pos) = stream.seek(SeekFrom::Current(0)).ok() {
                    log::debug!("detecting format of random access stream");
                    let meta = stream.metadata().ok().flatten();
                    let kind = AnyDecoder::new(BufReader::new(&mut stream)).kind().ok();
                    log::debug!("format detected: {:?}", &kind);
                    stream.seek(SeekFrom::Start(pos)).ok();
                    match kind {
                        Some(Format::Verbatim) => {
                            return Self::RandomAccess(stream);
                        }
                        Some(_) => {
                            log::debug!("creating decoder");
                            let dec = AnyDecoder::new(BufReader::new(stream));
                            log::debug!("decoder created");
                            return Self::Sequential(Box::new(dec.with_metadata(meta)));
                        }
                        None => (),
                    }
                }
                Self::Sequential(Box::new(stream))
            }
        }
    }

    /// Converts the stream to a sequential stream.
    pub fn as_sequential<'a>(&'a mut self) -> StreamOver<&'a mut (dyn ReadMeta + Send + Sync)> {
        match self {
            Self::Sequential(stream) => StreamOver(stream),
            Self::RandomAccess(stream) => StreamOver(stream),
        }
    }

    /// Converts the stream to a sequential stream.
    pub fn into_sequential(self) -> SequentialStream {
        match self {
            Self::Sequential(stream) => stream,
            Self::RandomAccess(stream) => Box::new(StreamOver(stream)),
        }
    }

    /// Adds context to the returned errors.
    pub fn tagged(self, reference: InputReference) -> Self {
        match self {
            Self::Sequential(stream) => Self::Sequential(Box::new(TaggedStream { reference, stream })),
            Self::RandomAccess(stream) => Self::RandomAccess(Box::new(TaggedStream { reference, stream })),
        }
    }
}

impl Read for Stream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Sequential(stream) => stream.read(buf),
            Self::RandomAccess(stream) => stream.read(buf),
        }
    }
}

impl Meta for Stream {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        match self {
            Self::Sequential(stream) => stream.metadata(),
            Self::RandomAccess(stream) => stream.metadata(),
        }
    }
}

// ---

/// A wrapper around a stream that adds context to the returned errors.
pub struct TaggedStream<R> {
    reference: InputReference,
    stream: R,
}

impl<R> Deref for TaggedStream<R> {
    type Target = R;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<R: Read> Read for TaggedStream<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to read {}: {}", self.reference.description(), e),
            )
        })
    }
}

impl<R: Seek> Seek for TaggedStream<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.stream.seek(pos).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to seek {}: {}", self.reference.description(), e),
            )
        })
    }
}

impl<R: Meta> Meta for TaggedStream<R> {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        self.stream.metadata()
    }
}

// ---

pub struct IndexedInput {
    pub reference: InputReference,
    pub stream: Mutex<RandomAccessStream>,
    pub index: Index,
}

impl IndexedInput {
    #[inline]
    fn new(reference: InputReference, stream: RandomAccessStream, index: Index) -> Self {
        Self {
            reference,
            stream: Mutex::new(stream),
            index,
        }
    }

    /// Opens the input file and indexes it.
    pub fn open<FS: FileSystem + Sync>(path: &PathBuf, indexer: &Indexer<FS>) -> Result<Self> {
        InputReference::File(path.clone().try_into()?).hold()?.index(indexer)
    }

    /// Converts the input to blocks.
    pub fn into_blocks(self) -> Blocks<IndexedInput, impl Iterator<Item = usize>> {
        let n = self.index.source().blocks.len();
        Blocks::new(Arc::new(self), (0..n).into_iter())
    }

    fn from_stream<FS: FileSystem + Sync>(
        reference: InputReference,
        stream: Stream,
        indexer: &Indexer<FS>,
    ) -> Result<Self> {
        let (stream, index) = Self::index_stream(&reference, stream, indexer)?;
        Ok(Self::new(reference, stream, index))
    }

    fn index_stream<FS: FileSystem + Sync>(
        reference: &InputReference,
        stream: Stream,
        indexer: &Indexer<FS>,
    ) -> Result<(RandomAccessStream, Index)> {
        log::info!("indexing {}", reference.description());

        if let (Some(path), Some(meta)) = (reference.path(), stream.metadata()?) {
            match stream {
                Stream::Sequential(stream) => Self::index_sequential_stream(path, &meta, stream, indexer),
                Stream::RandomAccess(stream) => Self::index_random_access_stream(path, &meta, stream, indexer),
            }
        } else {
            let mut tee = TeeReader::new(stream, ReplayBufCreator::new());
            let index = indexer.index_in_memory(&mut tee)?;
            let buf = tee.into_writer().result()?;
            let stream = Box::new(ReplayBufReader::new(buf).with_metadata(None));

            Ok((stream, index))
        }
    }

    fn index_random_access_stream<FS: FileSystem + Sync>(
        path: &PathBuf,
        meta: &Metadata,
        mut stream: RandomAccessStream,
        indexer: &Indexer<FS>,
    ) -> Result<(RandomAccessStream, Index)> {
        let pos = stream.seek(SeekFrom::Current(0))?;
        let index = indexer.index_stream(&mut stream, path, meta)?;

        stream.seek(SeekFrom::Start(pos))?;

        Ok((stream, index))
    }

    fn index_sequential_stream<FS: FileSystem + Sync>(
        path: &PathBuf,
        meta: &Metadata,
        stream: SequentialStream,
        indexer: &Indexer<FS>,
    ) -> Result<(RandomAccessStream, Index)> {
        let mut tee = TeeReader::new(stream, ReplayBufCreator::new());
        let index = indexer.index_stream(&mut tee, path, meta)?;
        let meta = meta.clone();

        let stream: RandomAccessStream = if tee.processed() == 0 {
            Box::new(ReplaySeekReader::new(tee.into_reader()).with_metadata(Some(meta)))
        } else {
            let buf = tee.into_writer().result()?;
            Box::new(ReplayBufReader::new(buf).with_metadata(Some(meta)))
        };

        Ok((stream, index))
    }
}

// ---

pub struct Blocks<I, II> {
    input: Arc<I>,
    indexes: II,
}

impl<II: Iterator<Item = usize>> Blocks<IndexedInput, II> {
    #[inline]
    pub fn new(input: Arc<IndexedInput>, indexes: II) -> Self {
        Self { input, indexes }
    }

    pub fn sorted(self) -> Blocks<IndexedInput, impl Iterator<Item = usize>> {
        let (input, indexes) = (self.input, self.indexes);
        let mut indexes: Vec<_> = indexes.collect();
        indexes.sort_by_key(|&i| input.index.source().blocks[i].stat.ts_min_max);
        Blocks::new(input, indexes.into_iter())
    }
}

impl<II: Iterator<Item = usize>> Iterator for Blocks<IndexedInput, II> {
    type Item = Block<IndexedInput>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.indexes.next().map(|i| Block::new(self.input.clone(), i))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indexes.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.indexes.count()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.indexes.nth(n).map(|i| Block::new(self.input.clone(), i))
    }
}

// ---

pub struct Block<I> {
    input: Arc<I>,
    index: usize,
    buf_pool: Option<Arc<BufPool>>,
}

impl Block<IndexedInput> {
    #[inline]
    pub fn new(input: Arc<IndexedInput>, index: usize) -> Self {
        Self {
            input,
            index,
            buf_pool: None,
        }
    }

    #[inline]
    pub fn with_buf_pool(self, buf_pool: Arc<BufPool>) -> Self {
        Self {
            input: self.input,
            index: self.index,
            buf_pool: Some(buf_pool),
        }
    }

    #[inline]
    pub fn into_lines(self) -> Result<BlockLines<IndexedInput>> {
        BlockLines::new(self)
    }

    #[inline]
    pub fn offset(&self) -> u64 {
        self.source_block().offset
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.source_block().size
    }

    #[inline]
    pub fn source_block(&self) -> &SourceBlock {
        &self.input.index.source().blocks[self.index]
    }

    #[inline]
    pub fn lines_valid(&self) -> u64 {
        self.source_block().stat.lines_valid
    }
}

// ---

pub struct BlockLines<I> {
    block: Block<I>,
    buf: Arc<Vec<u8>>,
    total: usize,
    current: usize,
    byte: usize,
    jump: usize,
}

impl BlockLines<IndexedInput> {
    pub fn new(mut block: Block<IndexedInput>) -> Result<Self> {
        let (buf, total) = {
            let block = &mut block;
            let mut buf = if let Some(pool) = &block.buf_pool {
                pool.checkout() // TODO: implement checkin
            } else {
                Vec::new()
            };
            let source_block = block.source_block();
            buf.resize(source_block.size.try_into()?, 0);
            let mut stream = block.input.stream.lock().unwrap();
            stream.seek(SeekFrom::Start(source_block.offset))?;
            stream.read_fill(&mut buf)?;
            let total = (source_block.stat.lines_valid + source_block.stat.lines_invalid).try_into()?;
            (buf, total)
        };
        Ok(Self {
            block,
            buf: Arc::new(buf), // TODO: optimize allocations
            total,
            current: 0,
            byte: 0,
            jump: 0,
        })
    }
}

impl Iterator for BlockLines<IndexedInput> {
    type Item = BlockLine;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }
        let block = self.block.source_block();
        let bitmap = &block.chronology.bitmap;

        if bitmap.len() != 0 {
            let k = 8 * size_of_val(&bitmap[0]);
            let n = self.current / k;
            let m = self.current % k;
            if m == 0 {
                let offsets = block.chronology.offsets[n];
                self.byte = offsets.bytes as usize;
                self.jump = offsets.jumps as usize;
            }
            if bitmap[n] & (1 << m) != 0 {
                self.byte = block.chronology.jumps[self.jump] as usize;
                self.jump += 1;
            }
        }
        let s = &self.buf[self.byte..];
        let l = s.iter().position(|&x| x == b'\n').map_or(s.len(), |i| i + 1);
        let offset = self.byte;
        self.byte += l;
        self.current += 1;

        Some(BlockLine::new(self.buf.clone(), offset..offset + l))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.total - self.current;
        (count, Some(count))
    }

    #[inline]
    fn count(self) -> usize {
        self.size_hint().0
    }
}

// ---

pub struct BlockLine {
    buf: Arc<Vec<u8>>,
    range: Range<usize>,
}

impl BlockLine {
    #[inline]
    pub fn new(buf: Arc<Vec<u8>>, range: Range<usize>) -> Self {
        Self { buf, range }
    }

    #[inline]
    pub fn bytes(&self) -> &[u8] {
        &self.buf[self.range.clone()]
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.range.start
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }
}

// ---

pub struct ConcatReader<I> {
    iter: I,
    item: Option<Input>,
}

impl<I> ConcatReader<I> {
    #[inline]
    pub fn new(iter: I) -> Self {
        Self { iter, item: None }
    }
}

impl<I> Read for ConcatReader<I>
where
    I: Iterator<Item = io::Result<Input>>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.item.is_none() {
                match self.iter.next() {
                    None => {
                        return Ok(0);
                    }
                    Some(result) => {
                        self.item = Some(result?);
                    }
                };
            }

            let input = self.item.as_mut().unwrap();
            let n = input.stream.read(buf).map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!("failed to read {}: {}", input.reference.description(), e),
                )
            })?;
            if n != 0 {
                return Ok(n);
            }
            self.item = None;
        }
    }
}

// ---

pub trait ReadSeek: Read + Seek {}
pub trait ReadSeekMeta: ReadSeek + Meta {}
pub trait ReadMeta: Read + Meta {}
pub trait BufReadSeek: BufRead + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

impl<T: Read + Seek + Meta> ReadSeekMeta for T {}

impl<T: Read + Meta> ReadMeta for T {}

impl<T: Meta + ?Sized> Meta for Box<T> {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        self.as_ref().metadata()
    }
}

pub struct StreamOver<T>(T);

impl<T> Deref for StreamOver<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Read> Read for StreamOver<T> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<T: Seek> Seek for StreamOver<T> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}

impl<T: Meta> Meta for StreamOver<T> {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        self.0.metadata()
    }
}

// ---

trait WithMeta {
    fn with_metadata(self, meta: Option<Metadata>) -> WithMetadata<Self>
    where
        Self: Sized;
}

impl<T> WithMeta for T {
    #[inline]
    fn with_metadata(self, meta: Option<Metadata>) -> WithMetadata<Self> {
        WithMetadata::new(self, meta)
    }
}

// ---

struct WithMetadata<T> {
    inner: T,
    meta: Option<Metadata>,
}

impl<T> WithMetadata<T> {
    fn new(inner: T, meta: Option<Metadata>) -> Self {
        Self { inner, meta }
    }
}

impl<T: Read> Read for WithMetadata<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<T: Write> Write for WithMetadata<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Seek> Seek for WithMetadata<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<T> Meta for WithMetadata<T> {
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        Ok(self.meta.clone())
    }
}

// ---

#[cfg(test)]
mod tests {
    use std::{
        str::FromStr,
        time::{Duration, SystemTime},
    };

    use itertools::Itertools;

    use crate::index::{IndexerSettings, MockFileSystem, MockSourceMetadata, RealFileSystem};

    use super::*;
    use io::Read;

    #[test]
    fn test_input_reference() {
        let reference = InputReference::Stdin;
        assert_eq!(reference.description(), "<stdin>");
        assert_eq!(reference.path(), None);
        let input = reference.open().unwrap();
        assert_eq!(input.reference, reference);
        let reference = InputReference::File(InputPath::ephemeral(PathBuf::from("test.log")));
        assert_eq!(reference.description(), "file '\u{1b}[33mtest.log\u{1b}[0m'");
        assert_eq!(reference.path(), Some(&PathBuf::from("test.log")));
    }

    #[test]
    fn test_input_holder() {
        let reference = InputReference::File(InputPath::ephemeral(PathBuf::from("sample/test.log")));
        let holder = InputHolder::new(reference, None);
        let mut stream = holder.stream().unwrap();
        let mut buf = Vec::new();
        let n = stream.read_to_end(&mut buf).unwrap();
        assert!(matches!(stream, Stream::RandomAccess(_)));
        let stream = stream.as_sequential();
        let meta = stream.metadata().unwrap();
        assert_eq!(meta.is_some(), true);
        assert_eq!(n, 70);
        assert_eq!(
            buf,
            br#"{"ts":"2024-10-01T01:02:03Z","level":"info","msg":"some test message"}"#
        );
    }

    #[test]
    fn test_input() {
        let input = Input::stdin().unwrap();
        assert!(matches!(input.stream, Stream::Sequential(_)));
        assert_eq!(input.reference.description(), "<stdin>");
        let input = Input::open(&PathBuf::from("sample/prometheus.log")).unwrap();
        assert!(matches!(input.stream, Stream::RandomAccess(_)));
        assert_eq!(
            input.reference.description(),
            "file '\u{1b}[33msample/prometheus.log\u{1b}[0m'"
        );
    }

    #[test]
    fn test_input_tail() {
        let input = Input::stdin().unwrap().tail(1).unwrap();
        assert!(matches!(input.stream, Stream::Sequential(_)));

        for &(filename, requested, expected) in &[
            ("sample/test.log", 1, 1),
            ("sample/test.log", 2, 1),
            ("sample/prometheus.log", 2, 2),
        ] {
            let input = Input::open(&PathBuf::from(filename)).unwrap().tail(requested).unwrap();
            let mut buf = Vec::new();
            let n = input.stream.into_sequential().read_to_end(&mut buf).unwrap();
            assert!(n > 0);
            assert_eq!(buf.lines().count(), expected);
        }
    }

    #[test]
    fn test_stream() {
        let stream = Stream::Sequential(Box::new(Cursor::new(b"test")));
        let stream = stream.verified().decoded().tagged(InputReference::Stdin);
        assert!(matches!(stream, Stream::Sequential(_)));
        let mut buf = Vec::new();
        let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(buf, b"test");

        let stream = Stream::RandomAccess(Box::new(UnseekableReader(Cursor::new(b"test"))));
        let stream = stream.tagged(InputReference::Stdin).verified();
        assert!(matches!(stream, Stream::Sequential(_)));
        let mut buf = Vec::new();
        let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(buf, b"test");

        let stream = Stream::RandomAccess(Box::new(UnseekableReader(Cursor::new(b"test"))));
        assert!(matches!(stream.metadata(), Ok(None)));
        let stream = stream.tagged(InputReference::Stdin).decoded();
        assert!(matches!(stream, Stream::Sequential(_)));
        assert!(matches!(stream.metadata(), Ok(None)));
        let mut buf = Vec::new();
        let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(buf, b"test");

        // echo 't' | gzip -cf | xxd -p | sed 's/\(..\)/\\x\1/g'
        let data = b"\x1f\x8b\x08\x00\x03\x87\x55\x67\x00\x03\x2b\xe1\x02\x00\x13\x47\x5f\xea\x02\x00\x00\x00";
        let stream = Stream::RandomAccess(Box::new(Cursor::new(data).with_metadata(None)));
        let stream = stream.tagged(InputReference::Stdin).decoded();
        assert!(matches!(stream, Stream::Sequential(_)));
        let mut buf = Vec::new();
        let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(buf, b"t\n");

        let stream = Stream::RandomAccess(Box::new(FailingReader));
        let stream = stream.tagged(InputReference::Stdin).decoded();
        assert!(matches!(stream, Stream::Sequential(_)));
        let mut buf = [0; 128];
        let result = stream.into_sequential().read(&mut buf);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn test_input_read_error() {
        let reference = InputReference::File(InputPath::ephemeral(PathBuf::from("test.log")));
        let input = Input::new(reference, Stream::Sequential(Box::new(FailingReader)));
        let mut buf = [0; 128];
        let result = input.stream.into_sequential().read(&mut buf);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
        assert_eq!(err.to_string().contains("test.log"), true);
    }

    #[test]
    fn test_input_hold_error_is_dir() {
        let reference = InputReference::File(InputPath::ephemeral(PathBuf::from(".")));
        let result = reference.hold();
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(err.to_string().contains("is a directory"), true);
    }

    #[test]
    fn test_input_hold_error_not_found() {
        let filename = "AKBNIJGHERHBNMCKJABHSDJ";
        let reference = InputReference::File(InputPath::ephemeral(PathBuf::from(filename)));
        let result = reference.hold();
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert_eq!(err.to_string().contains(filename), true);
    }

    #[test]
    fn test_input_gzip() {
        use std::io::Cursor;
        let data = Cursor::new(
            // echo 'test' | gzip -cf | xxd -p | sed 's/\(..\)/\\x\1/g'
            b"\x1f\x8b\x08\x00\x9e\xdd\x48\x67\x00\x03\x2b\x49\x2d\x2e\xe1\x02\x00\xc6\x35\xb9\x3b\x05\x00\x00\x00",
        );
        let mut stream = Stream::Sequential(Box::new(data)).verified().decoded();
        let mut buf = Vec::new();
        let result = stream.read_to_end(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
        assert_eq!(buf, b"test\n");
    }

    #[test]
    fn test_indexed_input_stdin() {
        let data = br#"{"ts":"2024-10-01T01:02:03Z","level":"info","msg":"some test message"}\n"#;
        let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
        let indexer = Indexer::<RealFileSystem>::new(1, PathBuf::new(), IndexerSettings::default());
        let input = IndexedInput::from_stream(InputReference::Stdin, stream, &indexer).unwrap();
        let mut blocks = input.into_blocks().collect_vec();
        assert_eq!(blocks.len(), 1);
        let block = blocks.drain(..).next().unwrap();
        assert_eq!(block.lines_valid(), 1);
        let mut lines = block.into_lines().unwrap().collect_vec();
        let line = lines.drain(..).next().unwrap();
        assert_eq!(line.bytes(), data);
    }

    #[test]
    fn test_indexed_input_file_random_access() {
        let meta = || {
            let ts = SystemTime::from(SystemTime::UNIX_EPOCH + Duration::from_secs(1704067200));
            let mut meta = MockSourceMetadata::new();
            meta.expect_len().return_const(70u64);
            meta.expect_modified().returning(move || Ok(ts));
            meta
        };

        let mut fs = MockFileSystem::new();
        let mut index_file = Vec::new();
        fs.expect_canonicalize()
            .returning(|path| Ok(std::path::Path::new("/tmp").join(path)));
        fs.expect_metadata().returning(move |_| Ok(meta()));
        fs.expect_exists().once().returning(|_| Ok(false));
        fs.expect_create()
            .returning(move |_| Ok(Box::new(Cursor::new(&mut index_file).with_metadata(None))));
        let index_file_path =
            PathBuf::from_str("a4c307cfc85cdccafeded6cb95e594cf32e24bf3aca066fd0be834ebc66bd0fc").unwrap();
        fs.expect_exists().returning(move |x| Ok(x == &index_file_path));

        let path = PathBuf::from("sample/test.log");
        let indexer = Indexer::new(
            1,
            PathBuf::new(),
            IndexerSettings {
                fs,
                ..Default::default()
            },
        );
        let input = IndexedInput::open(&path, &indexer).unwrap();
        let mut blocks = input.into_blocks().collect_vec();
        assert_eq!(blocks.len(), 1);
        let block = blocks.drain(..).next().unwrap();
        assert_eq!(block.lines_valid(), 1);
        let mut lines = block.into_lines().unwrap().collect_vec();
        let line = lines.drain(..).next().unwrap();
        assert_eq!(line.len(), 70);
    }

    // ---

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::Other, "read error"))
        }
    }

    impl Seek for FailingReader {
        fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
            match from {
                SeekFrom::Start(0) => Ok(0),
                SeekFrom::Current(0) => Ok(0),
                SeekFrom::End(0) => Ok(0),
                _ => Err(io::Error::new(io::ErrorKind::Other, "seek error")),
            }
        }
    }

    impl Meta for FailingReader {
        fn metadata(&self) -> std::io::Result<Option<Metadata>> {
            Ok(None)
        }
    }

    // ---

    struct UnseekableReader<R>(R);

    impl<R: Read> Read for UnseekableReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.0.read(buf)
        }
    }

    impl<R> Seek for UnseekableReader<R> {
        fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
            Err(io::Error::new(io::ErrorKind::Other, "seek error"))
        }
    }

    impl<R> Meta for UnseekableReader<R> {
        fn metadata(&self) -> io::Result<Option<Metadata>> {
            Ok(None)
        }
    }
}
