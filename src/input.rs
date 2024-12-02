// std imports
use std::cmp::min;
use std::convert::TryInto;
use std::fs::File;
use std::io::{self, stdin, BufReader, Cursor, Read, Seek, SeekFrom};
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
use crate::iox::ReadFill;
use crate::pool::SQPool;
use crate::replay::{ReplayBufCreator, ReplayBufReader};
use crate::tee::TeeReader;

// ---

pub type InputStream = Box<dyn Read + Send + Sync>;
pub type InputStreamFactory = Box<dyn FnOnce() -> Box<dyn Read> + Send + Sync>;

pub type InputSeekStream = Box<Mutex<dyn ReadSeek + Send + Sync>>;

pub type BufPool = SQPool<Vec<u8>>;

// ---

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

    pub fn open(&self) -> io::Result<Input> {
        self.hold()?.open()
    }

    pub fn open_tail(&self, n: u64) -> io::Result<Input> {
        match self {
            Self::Stdin => self.open(),
            Self::File(path) => {
                let mut file = File::open(path)
                    .map_err(|e| io::Error::new(e.kind(), format!("failed to open {}: {}", self.description(), e)))?;
                let mut buf = vec![0; 8];
                let bl = file.read(&mut buf)?;
                buf.truncate(bl);
                let stream: InputStream = if AnyDecoder::new(Cursor::new(&buf)).kind()? == Format::Verbatim {
                    Self::seek_tail(&mut file, n).ok();
                    Box::new(file)
                } else {
                    Box::new(AnyDecoder::new(BufReader::new(Cursor::new(buf).chain(file))))
                };
                Ok(Input::new(self.clone(), stream))
            }
        }
    }

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

pub struct InputHolder {
    pub reference: InputReference,
    pub stream: Option<Box<dyn ReadSeek + Send + Sync>>,
}

impl InputHolder {
    pub fn new(reference: InputReference, stream: Option<Box<dyn ReadSeek + Send + Sync>>) -> Self {
        Self { reference, stream }
    }

    pub fn open(self) -> io::Result<Input> {
        match self.reference {
            InputReference::Stdin => Ok(Input::new(self.reference.clone(), self.stdin())),
            InputReference::File(path) => match self.stream {
                Some(stream) => Input::open_stream(&path, stream),
                None => Input::open(&path),
            },
        }
    }

    pub fn index(self, indexer: &Indexer) -> Result<IndexedInput> {
        match self.reference {
            InputReference::Stdin => IndexedInput::open_sequential(self.reference.clone(), self.stdin(), indexer),
            InputReference::File(path) => match self.stream {
                Some(stream) => IndexedInput::open_stream(&path, stream, indexer),
                None => IndexedInput::open(&path, indexer),
            },
        }
    }

    fn stdin(self) -> InputStream {
        self.stream
            .map(|s| Box::new(ReadSeekToRead(s)) as InputStream)
            .unwrap_or_else(|| Box::new(decode(stdin())))
    }
}

pub struct Input {
    pub reference: InputReference,
    pub stream: InputStream,
}

impl Input {
    pub fn new(reference: InputReference, stream: InputStream) -> Self {
        Self {
            reference: reference.clone(),
            stream: Box::new(WrappedInputStream { reference, stream }),
        }
    }

    pub fn open(path: &PathBuf) -> io::Result<Self> {
        InputReference::File(path.clone()).hold()?.open()
    }

    pub fn open_stream(path: &PathBuf, stream: Box<dyn ReadSeek + Send + Sync>) -> io::Result<Self> {
        let stream: InputStream = Box::new(AnyDecoder::new(BufReader::new(stream)));
        Ok(Self::new(InputReference::File(path.clone()), stream))
    }
}

// ---

pub struct WrappedInputStream {
    reference: InputReference,
    stream: InputStream,
}

impl Read for WrappedInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to read {}: {}", self.reference.description(), e),
            )
        })
    }
}

// ---

pub struct IndexedInput {
    pub reference: InputReference,
    pub stream: InputSeekStream,
    pub index: Index,
}

impl IndexedInput {
    pub fn new(reference: InputReference, stream: InputSeekStream, index: Index) -> Self {
        Self {
            reference,
            stream,
            index,
        }
    }

    pub fn open(path: &PathBuf, indexer: &Indexer) -> Result<Self> {
        InputReference::File(path.clone()).hold()?.index(indexer)
    }

    pub fn open_stream(path: &PathBuf, mut stream: Box<dyn ReadSeek + Send + Sync>, indexer: &Indexer) -> Result<Self> {
        if !Self::is_seekable(&mut stream) {
            return Self::open_sequential(
                InputReference::File(path.clone()),
                Box::new(decode(stream).as_input_stream()),
                indexer,
            );
        }

        let index = indexer.index(&path)?;
        Ok(Self::new(
            InputReference::File(path.clone()),
            Box::new(Mutex::new(stream)),
            index,
        ))
    }

    pub fn open_sequential(reference: InputReference, stream: InputStream, indexer: &Indexer) -> Result<Self> {
        let mut tee = TeeReader::new(stream, ReplayBufCreator::new());
        let index = indexer.index_from_stream(&mut tee)?;
        let buf = tee.into_writer().result()?;
        Ok(IndexedInput::new(
            reference,
            Box::new(Mutex::new(ReplayBufReader::new(buf))),
            index,
        ))
    }

    pub fn into_blocks(self) -> Blocks<IndexedInput, impl Iterator<Item = usize>> {
        let n = self.index.source().blocks.len();
        Blocks::new(Arc::new(self), (0..n).into_iter())
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
    fn as_input_stream(self) -> InputStream;
}

impl<T: Read + Send + Sync + 'static> AsInputStream for T {
    fn as_input_stream(self) -> InputStream {
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
