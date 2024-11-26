/*

    Variants of input sources and their holders:
    1. Pipe (stdin, bash substitution, etc.) - Sequential access
    2. File - Random access (unless not seekable or compressed)

    Variants of compressions:
    1. raw (uncompressed) - Random access
    2. gzip (zlib) - Sequential access
    3. bzip2 - Sequential access
    4. xz - Sequential access
    5. zstd - Sequential access (unless multiple frames of small size)

    Variants of access:
    1. Sequential
    2. Random

    Variants of initial positioning:
    1. Head
    2. Tail (last n lines)
    2.1. Tail for sequential access - Skip HEAD
    2.2. Tail for random access - Read blocks from END until n lines are found

    Algorithm:
    1. Check if pipe or file
        1.1. If pipe, go to opening with sequential access (2)
        1.2. If file, go to opening with random access (3)
    2. Open with sequential access
        2.1. Use AnyDecoder to decode compressed files
        2.2. Go to initial positioning (4)
    3. Open with random access - check if compressed or seekable
        3.1. Try to seek to get current position
            3.1.1. If not seekable, go to opening with sequential access (2)
        3.2. Open with AnyDecoder to decode compressed files
            3.2.1. If compressed (except zstd-framed), go to opening with sequential access (2.2)
            3.2.2. If uncompressed (or zstd-framed) and seekable, open with random access
    4. Initial positioning
        4.1. Head - do nothing
        4.2. Tail
            4.2.1. Tail for sequential access - read from start to the end and keep last n lines
            4.2.2. Tail for random access - read blocks from the end until n lines are found



*/

// std imports
use std::cmp::min;
use std::convert::TryInto;
use std::fs::{File, Metadata};
use std::io::{self, stdin, BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
use std::mem::size_of_val;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// third-party imports
use deko::{bufread::AnyDecoder, Format};
use nu_ansi_term::Color;

// local imports
use crate::error::Result;
use crate::index::{Index, Indexer, SourceBlock};
use crate::pool::SQPool;
use crate::replay::{ReplayBufCreator, ReplayBufReader};
use crate::tee::TeeReader;

// ---

pub type SequentialStream = Box<dyn Read + Send + Sync>;
pub type RandomAccessStream = Box<dyn ReadSeekMetadata + Send + Sync>;
pub type BufPool = SQPool<Vec<u8>>;

// ---

/// A reference to an input file or stdin.
#[derive(Clone)]
pub enum InputReference {
    Stdin,
    File(PathBuf),
}

impl Into<io::Result<InputHolder>> for InputReference {
    fn into(self) -> io::Result<InputHolder> {
        self.hold()
    }
}

impl InputReference {
    /// Preliminarily opens the input file to ensure it exists and is readable
    /// and protect it from being suddenly deleted while we need it.
    pub fn hold(&self) -> io::Result<InputHolder> {
        Ok(InputHolder::new(
            self.clone(),
            match self {
                InputReference::Stdin => None,
                InputReference::File(path) => {
                    let meta = std::fs::metadata(path).map_err(|e| {
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
                    Some(Box::new(File::open(path).map_err(|e| {
                        io::Error::new(e.kind(), format!("failed to open {}: {}", self.description(), e))
                    })?))
                }
            },
        ))
    }

    /// Completely opens the input for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(&self) -> io::Result<Input> {
        self.hold()?.open()
    }

    // pub fn open_tail(&self, n: u64) -> io::Result<Input> {
    //     match self {
    //         Self::Stdin => self.open(),
    //         Self::File(path) => {
    //             let mut file = File::open(path)
    //                 .map_err(|e| io::Error::new(e.kind(), format!("failed to open {}: {}", self.description(), e)))?;
    //             let mut buf = vec![0; 8];
    //             let bl = file.read(&mut buf)?;
    //             buf.truncate(bl);
    //             let stream: SequentialStream = if AnyDecoder::new(Cursor::new(&buf)).kind()? == Format::Verbatim {
    //                 Self::seek_tail(&mut file, n).ok();
    //                 Box::new(file)
    //             } else {
    //                 Box::new(AnyDecoder::new(BufReader::new(Cursor::new(buf).chain(file))))
    //             };
    //             Ok(Input::new(self.clone(), stream))
    //         }
    //     }
    // }

    /// Returns a description of the input reference.
    pub fn description(&self) -> String {
        match self {
            Self::Stdin => "<stdin>".into(),
            Self::File(filename) => format!("file '{}'", Color::Yellow.paint(filename.to_string_lossy())),
        }
    }

    fn seek_tail(file: &mut File, lines: u64) -> io::Result<()> {
        const BUF_SIZE: usize = 64 * 1024;
        let mut scratch = [0; BUF_SIZE];
        let mut count: u64 = 0;
        let mut prev_pos = file.seek(SeekFrom::End(0))?;
        let mut pos = prev_pos;
        while pos > 0 {
            pos -= min(BUF_SIZE as u64, pos);
            pos = file.seek(SeekFrom::Start(pos))?;
            if pos == prev_pos {
                break;
            }
            let bn = min(BUF_SIZE, (prev_pos - pos) as usize);
            let buf = scratch[..bn].as_mut();

            file.read_exact(buf)?;

            for i in (0..bn).rev() {
                if buf[i] == b'\n' {
                    if count == lines {
                        file.seek(SeekFrom::Start(pos + i as u64 + 1))?;
                        return Ok(());
                    }
                    count += 1;
                }
            }

            prev_pos = pos;
        }
        file.seek(SeekFrom::Start(pos as u64))?;
        Ok(())
    }
}

// ---

pub trait MetadataHolder {
    fn metadata(&self) -> io::Result<Option<Metadata>>;
}

// ---

/// A holder of an input file.
/// It can be used to ensure the input file is not suddenly deleting while it is needed.
pub struct InputHolder {
    pub reference: InputReference,
    pub stream: Option<Box<dyn ReadSeekMetadata + Send + Sync>>,
}

impl InputHolder {
    /// Creates a new input holder.
    pub fn new(reference: InputReference, stream: Option<Box<dyn ReadSeekMetadata + Send + Sync>>) -> Self {
        Self { reference, stream }
    }

    /// Opens the input file for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(self) -> io::Result<Input> {
        Ok(Input::new(self.reference.clone(), self.stream()?))
    }

    /// Indexes the input file and returns IndexedInput that can be used to access the data in random order.
    pub fn index(self, indexer: &Indexer) -> Result<IndexedInput> {
        self.open()?.index(indexer)
        // match self.reference {
        //     InputReference::Stdin => IndexedInput::open_sequential(self.reference.clone(), self.stdin(), indexer),
        //     InputReference::File(path) => match self.stream {
        //         Some(stream) => IndexedInput::open_stream(&path, stream, indexer),
        //         None => IndexedInput::open(&path, indexer),
        //     },
        // }
    }

    fn stream(self) -> io::Result<Stream> {
        Ok(match &self.reference {
            InputReference::Stdin => match self.stream {
                Some(stream) => Stream::Sequential(Stream::RandomAccess(stream).sequential()),
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
            .map(|s| Box::new(ReadSeekToRead(s)) as SequentialStream)
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
            stream: stream.verified().decoded().with_context_errors(reference),
        }
    }

    pub fn indexed(self, indexer: &Indexer) -> Result<IndexedInput> {
        IndexedInput::from_stream(self.reference, self.stream, indexer)
    }

    /// Opens the file for reading.
    /// This includes decoding compressed files if needed.
    pub fn open(path: &PathBuf) -> io::Result<Self> {
        InputReference::File(path.clone()).open()
    }

    /// Opens the stdin for reading.
    pub fn stdin() -> io::Result<Self> {
        InputReference::Stdin.open()
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
            Self::Sequential(stream) => Self::Sequential(Box::new(AnyDecoder::new(BufReader::new(stream)))),
            Self::RandomAccess(mut stream) => {
                if let Some(pos) = stream.seek(SeekFrom::Current(0)).ok() {
                    let mut dec = AnyDecoder::new(BufReader::new(&mut stream));
                    if dec.kind().ok() == Some(Format::Verbatim) {
                        stream.seek(SeekFrom::Start(pos)).ok();
                        return Self::RandomAccess(stream);
                    }
                }
                Self::Sequential(Box::new(ReadSeekToRead(stream)))
            }
        }
    }

    /// Converts the stream to a sequential stream.
    pub fn sequential(self) -> SequentialStream {
        match self {
            Self::Sequential(stream) => stream,
            Self::RandomAccess(stream) => Box::new(ReadSeekToRead(stream)),
        }
    }

    pub fn random(self) -> RandomAccessStream {
        match self {
            Self::Sequential(stream) => {
                let mut tee = TeeReader::new(stream, ReplayBufCreator::new());
                let index = indexer.index_from_stream(&mut tee)?;
                let buf = tee.into_writer().result()?;
                Ok(Box::new(Mutex::new(ReplayBufReader::new(buf))))
            }
            Self::RandomAccess(stream) => stream,
        }
    }

    /// Adds context to the returned errors.
    pub fn with_context_errors(self, reference: InputReference) -> Self {
        match self {
            Self::Sequential(stream) => Self::Sequential(Box::new(TaggedStream { reference, stream })),
            Self::RandomAccess(stream) => Self::RandomAccess(Box::new(TaggedStream { reference, stream })),
        }
    }
}

// ---

/// A wrapper around a stream that adds context to the returned errors.
pub struct TaggedStream<R> {
    reference: InputReference,
    stream: R,
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

// ---

pub struct IndexedInput {
    pub reference: InputReference,
    pub stream: Mutex<RandomAccessStream>,
    pub index: Index,
}

impl IndexedInput {
    fn new(reference: InputReference, stream: RandomAccessStream, index: Index) -> Self {
        Self {
            reference,
            stream: Mutex::new(stream),
            index,
        }
    }

    /// Opens the input file and indexes it.
    pub fn open(path: &PathBuf, indexer: &Indexer) -> Result<Self> {
        InputReference::File(path.clone()).hold()?.index(indexer)
    }

    /// Converts the input to blocks.
    pub fn into_blocks(self) -> Blocks<IndexedInput, impl Iterator<Item = usize>> {
        let n = self.index.source().blocks.len();
        Blocks::new(Arc::new(self), (0..n).into_iter())
    }

    fn from_stream(reference: InputReference, stream: Stream, indexer: &Indexer) -> Result<Self> {
        match stream {
            Stream::Sequential(stream) => Self::from_sequential_stream(reference, stream, indexer),
            Stream::RandomAccess(stream) => Self::from_random_access_stream(path, stream, indexer),
        }
    }

    fn from_random_access_stream(
        reference: InputReference,
        mut stream: RandomAccessStream,
        indexer: &Indexer,
    ) -> Result<Self> {
        let index = indexer.index(&path)?;
        return Self::new(reference, stream, indexer.index_from_stream(&mut stream)?);
        if !Self::is_seekable(&mut stream) {
            return Self::open_sequential(
                InputReference::File(path.clone()),
                Box::new(decode(stream).as_input_stream()),
                indexer,
            );
        }

        let index = indexer.index(&path)?;
        Ok(Self::new(InputReference::File(path.clone()), stream, index))
    }

    fn from_sequential_stream(reference: InputReference, stream: SequentialStream, indexer: &Indexer) -> Result<Self> {
        let mut tee = TeeReader::new(stream, ReplayBufCreator::new());
        let index = indexer.index_from_stream(&mut tee)?;
        let buf = tee.into_writer().result()?;
        Ok(IndexedInput::new(
            reference,
            Box::new(Mutex::new(ReplayBufReader::new(buf))),
            index,
        ))
    }

    fn is_seekable<R: ReadSeek + Send + Sync>(mut stream: R) -> bool {
        let Ok(pos) = stream.seek(SeekFrom::Current(0)) else {
            return false;
        };

        let seekable = decode(ReadSeekToRead(&mut stream)).kind().ok() == Some(Format::Verbatim);
        stream.seek(SeekFrom::Start(pos)).ok();

        seekable
    }
}

// ---

pub struct Blocks<I, II> {
    input: Arc<I>,
    indexes: II,
}

impl<II: Iterator<Item = usize>> Blocks<IndexedInput, II> {
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

    fn next(&mut self) -> Option<Self::Item> {
        self.indexes.next().map(|i| Block::new(self.input.clone(), i))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indexes.size_hint()
    }

    fn count(self) -> usize {
        self.indexes.count()
    }

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
    pub fn new(input: Arc<IndexedInput>, index: usize) -> Self {
        Self {
            input,
            index,
            buf_pool: None,
        }
    }

    pub fn with_buf_pool(self, buf_pool: Arc<BufPool>) -> Self {
        Self {
            input: self.input,
            index: self.index,
            buf_pool: Some(buf_pool),
        }
    }

    pub fn into_lines(self) -> Result<BlockLines<IndexedInput>> {
        BlockLines::new(self)
    }

    pub fn offset(&self) -> u64 {
        self.source_block().offset
    }

    pub fn size(&self) -> u32 {
        self.source_block().size
    }

    pub fn source_block(&self) -> &SourceBlock {
        &self.input.index.source().blocks[self.index]
    }

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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.total - self.current;
        (count, Some(count))
    }

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
    pub fn new(buf: Arc<Vec<u8>>, range: Range<usize>) -> Self {
        Self { buf, range }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf[self.range.clone()]
    }

    pub fn offset(&self) -> usize {
        self.range.start
    }

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
pub trait ReadSeekMetadata: ReadSeek + MetadataHolder {}
pub trait BufReadSeek: BufRead + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

pub struct ReadSeekToRead<T>(T);

impl<T> Read for ReadSeekToRead<T>
where
    T: ReadSeek,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

trait AsInputStream {
    fn as_input_stream(self) -> SequentialStream;
}

impl<T: Read + Send + Sync + 'static> AsInputStream for T {
    fn as_input_stream(self) -> SequentialStream {
        Box::new(self)
    }
}

// ---

fn decode<R: Read + Send + Sync>(input: R) -> AnyDecoder<BufReader<R>> {
    AnyDecoder::new(BufReader::new(input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::io::Read;

    #[test]
    fn test_input_read_error() {
        let reference = InputReference::File(PathBuf::from("test.log"));
        let mut input = Input::new(reference, Box::new(FailingReader));
        let mut buf = [0; 128];
        let result = input.stream.read(&mut buf);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Other);
        assert_eq!(err.to_string().contains("test.log"), true);
    }

    #[test]
    fn test_input_hold_error_is_dir() {
        let reference = InputReference::File(PathBuf::from("."));
        let result = reference.hold();
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(err.to_string().contains("is a directory"), true);
    }

    #[test]
    fn test_input_hold_error_not_found() {
        let filename = "AKBNIJGHERHBNMCKJABHSDJ";
        let reference = InputReference::File(PathBuf::from(filename));
        let result = reference.hold();
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(err.to_string().contains(filename), true);
    }

    #[test]
    fn test_input_gzip() {
        use std::io::Cursor;
        let data = Cursor::new(
            // echo 'test' | gzip -cf | xxd -p | sed 's/\(..\)/\\x\1/g'
            b"\x1f\x8b\x08\x00\x9e\xdd\x48\x67\x00\x03\x2b\x49\x2d\x2e\xe1\x02\x00\xc6\x35\xb9\x3b\x05\x00\x00\x00",
        );
        let mut input = Input::open_stream(&PathBuf::from("test.log.gz"), Box::new(data)).unwrap();
        let mut buf = Vec::new();
        let result = input.stream.read_to_end(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
        assert_eq!(buf, b"test\n");
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "read error"))
        }
    }
}
