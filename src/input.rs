// std imports
use std::{
    cmp::min,
    convert::TryInto,
    fs::{self, File, Metadata},
    io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write, stdin},
    mem::size_of_val,
    ops::{Deref, Range},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

// third-party imports
use deko::{Format, bufread::AnyDecoder};

// local imports
use crate::{
    Delimit,
    error::Result,
    index::{Index, Indexer, SourceBlock, SourceMetadata},
    iox::ReadFill,
    replay::{ReplayBufCreator, ReplayBufReader, ReplaySeekReader},
    scanning::{Delimiter, Search},
    tee::TeeReader,
    vfs::{FileSystem, LocalFileSystem},
    xerr::HighlightQuoted,
};

// ---

pub type SequentialStream = Box<dyn ReadMeta + Send + Sync>;
pub type RandomAccessStream = Box<dyn ReadSeekMeta + Send + Sync>;

// ---

/// The path to an input file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InputPath {
    pub original: PathBuf,
    pub canonical: PathBuf,
}

impl InputPath {
    /// Creates a new input path.
    pub fn new(original: PathBuf, canonical: PathBuf) -> Self {
        Self { original, canonical }
    }

    /// Resolves the canonical path for the given path.
    pub fn resolve(original: PathBuf) -> io::Result<Self> {
        Self::resolve_with_fs(original, LocalFileSystem)
    }

    /// Resolves the canonical path for the given path using the specified file system.
    pub fn resolve_with_fs<FS: FileSystem>(original: PathBuf, fs: FS) -> io::Result<Self> {
        let canonical = fs.canonicalize(&original).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to resolve path for {}: {}", original.hlq(), e),
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
        Self::resolve(original)
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
            Self::File(path) => format!("file {}", path.original.hlq()),
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

type InputStream = Box<dyn ReadSeekMeta + Send + Sync>;

/// A holder of an input file.
/// It can be used to ensure the input file is not suddenly deleting while it is needed.
pub struct InputHolder {
    pub reference: InputReference,
    pub stream: Option<InputStream>,
}

impl InputHolder {
    /// Creates a new input holder.
    pub fn new(reference: InputReference, stream: Option<InputStream>) -> Self {
        Self { reference, stream }
    }

    /// Opens the input file for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(self) -> io::Result<Input> {
        let stream = Self::stream(&self.reference, self.stream)?;
        Ok(Input::new(self.reference, stream))
    }

    /// Indexes the input file and returns IndexedInput that can be used to access the data in random order.
    pub fn index<FS>(self, indexer: &Indexer<FS>, delimiter: Delimiter) -> Result<IndexedInput>
    where
        FS: FileSystem + Sync,
        FS::Metadata: SourceMetadata,
    {
        self.open()?.indexed(indexer, delimiter)
    }

    fn stream(reference: &InputReference, stream: Option<InputStream>) -> io::Result<Stream> {
        Ok(match &reference {
            InputReference::Stdin => match stream {
                Some(stream) => Stream::Sequential(Stream::RandomAccess(stream).into_sequential()),
                None => Stream::Sequential(Self::stdin(stream)),
            },
            InputReference::File(_) => match stream {
                Some(stream) => Stream::RandomAccess(stream),
                None => Stream::RandomAccess(reference.hold()?.stream.unwrap()),
            },
        })
    }

    fn stdin(stream: Option<InputStream>) -> SequentialStream {
        stream
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
    pub fn indexed<FS>(self, indexer: &Indexer<FS>, delimiter: Delimiter) -> Result<IndexedInput>
    where
        FS: FileSystem + Sync,
        FS::Metadata: SourceMetadata,
    {
        IndexedInput::from_stream(self.reference, self.stream, delimiter, indexer)
    }

    /// Opens the file for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(path: &Path) -> io::Result<Self> {
        InputReference::File(path.to_path_buf().try_into()?).open()
    }

    /// Opens the stdin for reading.
    pub fn stdin() -> io::Result<Self> {
        InputReference::Stdin.open()
    }

    /// Seeks to the last `entries` entries of the input.
    pub fn tail(mut self, entries: u64, delimiter: Delimiter) -> io::Result<Self> {
        match &mut self.stream {
            Stream::Sequential(_) => (),
            Stream::RandomAccess(stream) => Self::seek_tail(stream, entries, delimiter)?,
        }
        Ok(self)
    }

    fn seek_tail(stream: &mut RandomAccessStream, entries: u64, delimiter: Delimiter) -> io::Result<()> {
        const BUF_SIZE: usize = 64 * 1024;
        let searcher = delimiter.into_searcher();
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

            let mut r = bn;
            while let Some(i) = searcher.search_r(&buf[..r], pos == 0) {
                if count == entries {
                    stream.seek(SeekFrom::Start(pos + i.end as u64))?;
                    return Ok(());
                }
                count += 1;
                r = i.start;
            }

            if r != 0 {
                if let Some(i) = searcher.partial_match_r(&buf[..r]) {
                    pos += i as u64;
                }
            }
        }
        stream.seek(SeekFrom::Start(pos))?;
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
                if stream.stream_position().is_err() {
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
                if let Ok(size) = stream.stream_position() {
                    log::debug!("detecting format of random access stream");
                    let meta = stream.metadata().ok().flatten();
                    let kind = AnyDecoder::new(BufReader::new(&mut stream)).kind().ok();
                    log::debug!("format detected: {:?}", &kind);
                    stream.seek(SeekFrom::Start(size)).ok();
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
    pub fn as_sequential(&mut self) -> StreamOver<&mut (dyn ReadMeta + Send + Sync)> {
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
    pub delimiter: Delimiter,
    pub index: Index,
}

impl IndexedInput {
    #[inline]
    fn new(reference: InputReference, stream: RandomAccessStream, delimiter: Delimiter, index: Index) -> Self {
        Self {
            reference,
            stream: Mutex::new(stream),
            delimiter,
            index,
        }
    }

    /// Opens the input file and indexes it.
    pub fn open<FS>(path: &Path, indexer: &Indexer<FS>, delimiter: Delimiter) -> Result<Self>
    where
        FS: FileSystem + Sync,
        FS::Metadata: SourceMetadata,
    {
        InputReference::File(PathBuf::from(path).try_into()?)
            .hold()?
            .index(indexer, delimiter)
    }

    /// Converts the input to blocks.
    pub fn into_blocks(self) -> Blocks<IndexedInput, impl Iterator<Item = usize>> {
        let n = self.index.source().blocks.len();
        Blocks::new(Arc::new(self), 0..n)
    }

    fn from_stream<FS>(
        reference: InputReference,
        stream: Stream,
        delimiter: Delimiter,
        indexer: &Indexer<FS>,
    ) -> Result<Self>
    where
        FS: FileSystem + Sync,
        FS::Metadata: SourceMetadata,
    {
        let (stream, index) = Self::index_stream(&reference, stream, indexer)?;
        Ok(Self::new(reference, stream, delimiter, index))
    }

    fn index_stream<FS>(
        reference: &InputReference,
        stream: Stream,
        indexer: &Indexer<FS>,
    ) -> Result<(RandomAccessStream, Index)>
    where
        FS: FileSystem + Sync,
        FS::Metadata: SourceMetadata,
    {
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

    fn index_random_access_stream<FS>(
        path: &Path,
        meta: &Metadata,
        mut stream: RandomAccessStream,
        indexer: &Indexer<FS>,
    ) -> Result<(RandomAccessStream, Index)>
    where
        FS: FileSystem + Sync,
        FS::Metadata: SourceMetadata,
    {
        let position = stream.stream_position()?;
        let index = indexer.index_stream(&mut stream, path, meta)?;

        stream.seek(SeekFrom::Start(position))?;

        Ok((stream, index))
    }

    fn index_sequential_stream<FS>(
        path: &Path,
        meta: &Metadata,
        stream: SequentialStream,
        indexer: &Indexer<FS>,
    ) -> Result<(RandomAccessStream, Index)>
    where
        FS: FileSystem + Sync,
        FS::Metadata: SourceMetadata,
    {
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
}

impl Block<IndexedInput> {
    #[inline]
    pub fn new(input: Arc<IndexedInput>, index: usize) -> Self {
        Self { input, index }
    }

    #[inline]
    pub fn into_entries(self) -> Result<BlockEntries<IndexedInput>> {
        BlockEntries::new(self)
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
    pub fn entries_valid(&self) -> u64 {
        self.source_block().stat.entries_valid
    }
}

// ---

pub struct BlockEntries<I> {
    searcher: Arc<dyn Search>,
    block: Block<I>,
    buf: Arc<Vec<u8>>,
    total: usize,
    current: usize,
    byte: usize,
    jump: usize,
}

impl BlockEntries<IndexedInput> {
    pub fn new(mut block: Block<IndexedInput>) -> Result<Self> {
        let (buf, total) = {
            let block = &mut block;
            let mut buf = Vec::new();
            let source_block = block.source_block();
            buf.resize(source_block.size.try_into()?, 0);
            let mut stream = block.input.stream.lock().unwrap();
            stream.seek(SeekFrom::Start(source_block.offset))?;
            stream.read_fill(&mut buf)?;
            let total = (source_block.stat.entries_valid + source_block.stat.entries_invalid).try_into()?;
            (buf, total)
        };
        Ok(Self {
            searcher: block.input.delimiter.clone().into_searcher(),
            block,
            buf: Arc::new(buf),
            total,
            current: 0,
            byte: 0,
            jump: 0,
        })
    }
}

impl Iterator for BlockEntries<IndexedInput> {
    type Item = BlockEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }
        let block = self.block.source_block();
        let bitmap = &block.chronology.bitmap;

        if !bitmap.is_empty() {
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
        let l = self.searcher.search_l(s, true).map_or(s.len(), |i| i.end);
        let offset = self.byte;
        self.byte += l;
        self.current += 1;

        Some(BlockEntry::new(self.buf.clone(), offset..offset + l))
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

pub struct BlockEntry {
    buf: Arc<Vec<u8>>,
    range: Range<usize>,
}

impl BlockEntry {
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

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range.start == self.range.end
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
    #[inline]
    fn new(inner: T, meta: Option<Metadata>) -> Self {
        Self { inner, meta }
    }
}

impl<T: Read> Read for WithMetadata<T> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<T: Write> Write for WithMetadata<T> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Seek> Seek for WithMetadata<T> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<T> Meta for WithMetadata<T> {
    #[inline]
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        Ok(self.meta.clone())
    }
}

// ---

#[cfg(test)]
mod tests;
