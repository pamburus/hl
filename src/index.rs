//
// indexing module
//
// index build phase pipeline scheme:
// -----------------------------------------------------------------------
//                            % | N                   ->   |
// | dir-scan -> | file-scan -> | N * segment-process -> % | save-index ->
//                            % | N                   ->   |
// -----------------------------------------------------------------------
//

// std imports
#[cfg(test)]
use mockall::{automock, predicate::*};
use std::{
    cmp::{max, min},
    convert::{Into, TryFrom, TryInto},
    fmt::{self, Display},
    fs::{self, File},
    io::{self, Read, Seek, Write},
    iter::empty,
    num::{NonZero, NonZeroU32},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

// third-party imports
use capnp::{message, serialize::read_message};
use closure::closure;
use crossbeam_channel as channel;
use crossbeam_channel::RecvError;
use crossbeam_utils::thread;
use derive_more::{Deref, From};
use itertools::izip;
use nonzero_ext::nonzero;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// local imports
use crate::{
    app::{InputFormat, UnixTimestampUnit},
    error::{Error, Result},
    index_capnp as schema,
    level::Level,
    model::{Parser, ParserSettings, RawRecord},
    scanning::{Delimiter, Scanner, Segment, SegmentBuf, SegmentBufFactory},
    settings::PredefinedFields,
};

// types
pub type Writer = dyn Write + Send + Sync;
pub type Reader = dyn Read + Send + Sync;

// ---

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Hash {
    Sha256([u8; 32]),
    GxHash64(u64),
    WyHash(u64),
}

// ---

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Timestamp {
    pub sec: i64,
    pub nsec: u32,
}

impl Timestamp {
    pub fn add(mut self, interval: std::time::Duration) -> Self {
        self.sec += interval.as_secs() as i64;
        self.nsec += interval.subsec_nanos();
        self
    }

    pub fn sub(mut self, interval: std::time::Duration) -> Self {
        self.sec -= interval.as_secs() as i64;
        if self.nsec >= interval.subsec_nanos() {
            self.nsec -= interval.subsec_nanos();
        } else {
            self.sec -= 1;
            self.nsec += 1_000_000_000 - interval.subsec_nanos();
        }
        self
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.sec, self.nsec)
    }
}

impl From<(i64, u32)> for Timestamp {
    fn from(value: (i64, u32)) -> Self {
        Self {
            sec: value.0,
            nsec: value.1,
        }
    }
}

impl From<chrono::DateTime<chrono::Utc>> for Timestamp {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            sec: value.timestamp(),
            nsec: value.timestamp_subsec_nanos(),
        }
    }
}

impl std::ops::Sub for Timestamp {
    type Output = std::time::Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut secs = (self.sec - rhs.sec) as u64;
        let nanos = if self.nsec >= rhs.nsec {
            self.nsec - rhs.nsec
        } else {
            secs -= 1;
            self.nsec + 1_000_000_000 - rhs.nsec
        };
        std::time::Duration::new(secs, nanos)
    }
}

// ---

#[derive(Default)]
pub struct IndexerSettings<'a, FS: FileSystem> {
    fs: FS,
    buffer_size: BufferSize,
    max_message_size: MessageSize,
    fields: &'a PredefinedFields,
    delimiter: Delimiter,
    allow_prefix: bool,
    unix_ts_unit: Option<UnixTimestampUnit>,
    format: Option<InputFormat>,
}

impl<'a, FS: FileSystem + Default> IndexerSettings<'a, FS> {
    pub fn new(
        buffer_size: BufferSize,
        max_message_size: MessageSize,
        fields: &'a PredefinedFields,
        delimiter: Delimiter,
        allow_prefix: bool,
        unix_ts_unit: Option<UnixTimestampUnit>,
        format: Option<InputFormat>,
    ) -> Self {
        Self {
            fs: FS::default(),
            buffer_size,
            max_message_size,
            fields,
            delimiter,
            allow_prefix,
            unix_ts_unit,
            format,
        }
    }

    pub fn hash(&self) -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();
        bincode::serialize_into(
            &mut hasher,
            &(
                &self.buffer_size,
                &self.max_message_size,
                &self.fields,
                &self.delimiter,
                &self.allow_prefix,
                &self.unix_ts_unit,
                &self.format,
            ),
        )?;
        Ok(hasher.finalize().into())
    }
}

#[derive(Deref, From, Serialize, Deserialize)]
pub struct BufferSize(NonZeroU32);

impl Default for BufferSize {
    fn default() -> Self {
        Self(nonzero!(4 * 1024u32))
    }
}

impl TryFrom<NonZero<usize>> for BufferSize {
    type Error = std::num::TryFromIntError;

    fn try_from(value: NonZero<usize>) -> std::result::Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl From<BufferSize> for NonZeroU32 {
    fn from(value: BufferSize) -> NonZeroU32 {
        value.0.into()
    }
}

impl From<BufferSize> for u32 {
    fn from(value: BufferSize) -> u32 {
        value.0.into()
    }
}

#[derive(Deref, From, Serialize, Deserialize)]
pub struct MessageSize(NonZeroU32);

impl Default for MessageSize {
    fn default() -> Self {
        Self(nonzero!(64 * 1024u32))
    }
}

impl TryFrom<NonZero<usize>> for MessageSize {
    type Error = std::num::TryFromIntError;

    fn try_from(value: NonZero<usize>) -> std::result::Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl From<MessageSize> for NonZeroU32 {
    fn from(value: MessageSize) -> NonZeroU32 {
        value.0.into()
    }
}

impl From<MessageSize> for u32 {
    fn from(value: MessageSize) -> u32 {
        value.0.into()
    }
}

// ---

/// Allows log files indexing to enable message sorting.
pub struct Indexer<FS = RealFileSystem> {
    fs: FS,
    concurrency: usize,
    buffer_size: u32,
    max_message_size: u32,
    dir: PathBuf,
    parser: Parser,
    delimiter: Delimiter,
    allow_prefix: bool,
    format: Option<InputFormat>,
}

impl<FS: FileSystem + Sync> Indexer<FS> {
    /// Returns a new Indexer with the given parameters.
    pub fn new(concurrency: usize, dir: PathBuf, settings: IndexerSettings<'_, FS>) -> Self {
        Self {
            fs: settings.fs,
            concurrency,
            buffer_size: settings.buffer_size.into(),
            max_message_size: settings.max_message_size.into(),
            dir,
            parser: Parser::new(ParserSettings::new(
                &settings.fields,
                empty(),
                false,
                settings.unix_ts_unit,
            )),
            delimiter: settings.delimiter,
            allow_prefix: settings.allow_prefix,
            format: settings.format,
        }
    }

    /// Builds index for the given file.
    ///
    /// Builds the index, saves it to disk and returns it.
    pub fn index(&self, source_path: &PathBuf) -> Result<Index> {
        let (source_path, _, index_path, index, actual) = self.prepare(source_path, None)?;
        if actual {
            return Ok(index.unwrap());
        }

        self.build_index(&source_path, &index_path, index)
    }

    /// Builds index for the given file represended by a stream.
    ///
    /// The stream may be an uncompressed representation of the file.
    pub fn index_stream(
        &self,
        stream: &mut Reader,
        source_path: Option<&PathBuf>,
        meta: Option<fs::Metadata>,
    ) -> Result<Index> {
        let Some(source_path) = source_path else {
            return self.index_in_memory(stream);
        };

        let (source_path, meta, index_path, index, actual) = self.prepare(source_path, meta)?;
        if actual {
            return Ok(index.unwrap());
        }

        self.build_index_from_stream(stream, &source_path, &index_path, meta, index)
    }

    /// Builds an in-memory index for the given stream.
    pub fn index_in_memory(&self, input: &mut Reader) -> Result<Index> {
        self.process_file(
            &PathBuf::from("<none>"),
            Metadata {
                len: 0,
                modified: (0, 0),
            },
            input,
            &mut io::sink(),
            None,
        )
    }

    fn prepare(
        &self,
        source_path: &PathBuf,
        meta: Option<fs::Metadata>,
    ) -> Result<(PathBuf, fs::Metadata, PathBuf, Option<Index>, bool)> {
        let source_path = self.fs.canonicalize(source_path)?;
        let meta = match meta {
            Some(meta) => meta,
            None => self.fs.metadata(&source_path)?,
        };
        let hash = hex::encode(sha256(source_path.to_string_lossy().as_bytes()));
        let index_path = self.dir.join(PathBuf::from(hash));
        let mut existing_index = None;
        let mut actual = false;

        log::debug!("canonical source path: {}", source_path.display());
        log::debug!("index file path:       {}", index_path.display());
        log::debug!("source meta: size={} modified={:?}", meta.len(), ts(meta.modified()?));

        if self.fs.exists(&index_path)? {
            let mut file = match self.fs.open(&index_path) {
                Ok(file) => file,
                Err(err) => {
                    return Err(Error::FailedToOpenFileForReading {
                        path: index_path.clone(),
                        source: err,
                    });
                }
            };
            if let Ok(index) = Index::load(&mut file) {
                log::debug!(
                    "index stuff: size={} modified={:?}",
                    index.source().size,
                    index.source().modified
                );
                if meta.len() == index.source().size && ts(meta.modified()?) == index.source().modified {
                    actual = true;
                }
                existing_index = Some(index);
            }
        }
        Ok((source_path, meta, index_path, existing_index, actual))
    }

    fn build_index(&self, source_path: &PathBuf, index_path: &PathBuf, existing_index: Option<Index>) -> Result<Index> {
        let mut input = match self.fs.open(&source_path) {
            Ok(input) => input,
            Err(err) => {
                return Err(Error::FailedToOpenFileForReading {
                    path: source_path.clone(),
                    source: err,
                });
            }
        };

        let meta = match self.fs.metadata(&source_path) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(Error::FailedToGetFileMetadata {
                    path: source_path.clone(),
                    source: err,
                });
            }
        };

        self.build_index_from_stream(&mut input, source_path, index_path, meta, existing_index)
    }

    fn build_index_from_stream(
        &self,
        stream: &mut Reader,
        source_path: &PathBuf,
        index_path: &PathBuf,
        metadata: fs::Metadata,
        existing_index: Option<Index>,
    ) -> Result<Index> {
        let mut output = match self.fs.create(&index_path) {
            Ok(output) => output,
            Err(err) => {
                return Err(Error::FailedToOpenFileForWriting {
                    path: index_path.clone(),
                    source: err,
                });
            }
        };

        self.process_file(
            &source_path,
            (&metadata).try_into()?,
            stream,
            &mut output,
            existing_index,
        )
    }

    fn process_file(
        &self,
        path: &PathBuf,
        metadata: Metadata,
        input: &mut Reader,
        output: &mut Writer,
        existing_index: Option<Index>,
    ) -> Result<Index> {
        let n = self.concurrency;
        let sfi = Arc::new(SegmentBufFactory::new(self.buffer_size.try_into()?));
        thread::scope(|scope| -> Result<Index> {
            // prepare receive/transmit channels for input data
            let (txi, rxi): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded(1)).unzip();
            // prepare receive/transmit channels for output data
            let (txo, rxo): (Vec<_>, Vec<_>) = (0..n)
                .into_iter()
                .map(|_| channel::bounded::<(usize, Stat, Chronology, Option<Hash>)>(1))
                .unzip();
            // spawn reader thread
            let reader = scope.spawn(closure!(clone sfi, |_| -> Result<()> {
                let mut sn: usize = 0;
                let scanner = Scanner::new(sfi, &self.delimiter);
                for item in scanner.items(input).with_max_segment_size(self.max_message_size.try_into()?) {
                    if let Err(_) = txi[sn % n].send((sn, item?)) {
                        break;
                    }
                    sn += 1;
                }
                Ok(())
            }));
            // spawn processing threads
            for (rxi, txo) in izip!(rxi, txo) {
                scope.spawn(closure!(ref sfi, ref existing_index, |_| {
                    for (sn, segment) in rxi.iter() {
                        let (stat, chronology, segment, hash) = match segment {
                            Segment::Complete(segment) => {
                                let hash = Hash::WyHash(wyhash::wyhash(segment.data(), 0));
                                let (stat, chronology) = existing_index
                                    .as_ref()
                                    .and_then(|index| Self::match_segment(&index, sn, &hash))
                                    .map(|(stat, chronology)| (stat, chronology))
                                    .unwrap_or_else(|| self.process_segment(&segment));
                                (stat, chronology, segment, Some(hash))
                            }
                            Segment::Incomplete(segment, _) => {
                                let mut stat = Stat::new();
                                stat.add_invalid();
                                (stat, Chronology::default(), segment, None)
                            }
                        };
                        let size = segment.data().len();
                        sfi.recycle(segment);
                        if let Err(_) = txo.send((size, stat, chronology, hash)) {
                            break;
                        };
                    }
                }));
            }
            // spawn builder thread
            let builder = scope.spawn(move |_| -> Result<Index> {
                let bs = usize::try_from(self.buffer_size)?;
                let mut index = Index {
                    source: SourceFile {
                        size: metadata.len,
                        path: path.to_string_lossy().into(),
                        modified: metadata.modified,
                        stat: Stat::new(),
                        blocks: Vec::with_capacity((usize::try_from(metadata.len)? + bs - 1) / bs),
                    },
                };

                let mut sn = 0;
                let mut offset: u64 = 0;
                loop {
                    match rxo[sn % n].recv() {
                        Ok((size, stat, chronology, hash)) => {
                            index.source.stat.merge(&stat);
                            index.source.blocks.push(SourceBlock::new(
                                offset,
                                size.try_into()?,
                                stat,
                                chronology,
                                hash,
                            ));
                            offset += u64::try_from(size)?;
                        }
                        Err(RecvError) => {
                            break;
                        }
                    }
                    sn += 1;
                }
                Ok(index)
            });
            // collect errors from reader and builder threads
            reader.join().unwrap()?;
            let index = builder.join().unwrap()?;
            index.save(output)?;
            Ok(index)
        })
        .unwrap()
    }

    fn process_segment(&self, segment: &SegmentBuf) -> (Stat, Chronology) {
        let mut stat = Stat::new();
        let mut sorted = true;
        let mut prev_ts = None;
        let mut lines = Vec::<(Option<Timestamp>, u32, u32)>::with_capacity(segment.data().len() / 512);
        let mut offset = 0;
        let mut i = 0;
        for data in rtrim(segment.data(), b'\n').split(|c| *c == b'\n') {
            let data_len = data.len();
            let data = strip(data, b'\r');
            let mut ts = None;
            let mut rel = 0;
            if data.len() != 0 {
                let mut stream = RawRecord::parser()
                    .allow_prefix(self.allow_prefix)
                    .format(self.format)
                    .parse(data);
                while let Some(item) = stream.next() {
                    match item {
                        Ok(ar) => {
                            let rec = self.parser.parse(ar.record);
                            let mut flags = 0;
                            match rec.level {
                                Some(Level::Debug) => {
                                    flags |= schema::FLAG_LEVEL_DEBUG;
                                }
                                Some(Level::Info) => {
                                    flags |= schema::FLAG_LEVEL_INFO;
                                }
                                Some(Level::Warning) => {
                                    flags |= schema::FLAG_LEVEL_WARNING;
                                }
                                Some(Level::Error) => {
                                    flags |= schema::FLAG_LEVEL_ERROR;
                                }
                                None => (),
                            }
                            ts = rec.ts.and_then(|ts| ts.unix_utc()).map(|ts| ts.into());
                            if ts < prev_ts {
                                sorted = false;
                            }
                            stat.add_valid(ts, flags);
                            lines.push((ts.or(prev_ts), i as u32, offset + ar.offsets.start as u32));
                            rel = ar.offsets.end;
                            i += 1;
                            prev_ts = ts;
                        }
                        _ => {
                            stat.add_invalid();
                            lines.push((ts.or(prev_ts), i as u32, offset + rel as u32));
                            i += 1;
                            break;
                        }
                    }
                }
            } else {
                stat.add_invalid();
                lines.push((ts.or(prev_ts), i as u32, offset));
                i += 1;
            }
            offset += data_len as u32 + 1;
        }
        let chronology = if sorted {
            Chronology::default()
        } else {
            stat.flags |= schema::FLAG_UNSORTED;
            lines.sort();

            let n = (lines.len() + 63) / 64;
            let mut bitmap = Vec::with_capacity(n);
            let mut offsets = Vec::with_capacity(n);
            let mut jumps = Vec::new();
            let mut prev = None;
            for chunk in lines.chunks(64) {
                let mut mask: u64 = 0;
                for (i, line) in chunk.iter().enumerate() {
                    if i == 0 {
                        offsets.push(OffsetPair {
                            bytes: line.2,
                            jumps: jumps.len().try_into().unwrap(),
                        });
                    }
                    if let Some(prev) = prev {
                        if line.1 != prev + 1 {
                            mask |= 1 << i;
                            jumps.push(line.2);
                        }
                    }
                    prev = Some(line.1);
                }
                bitmap.push(mask);
            }
            Chronology { bitmap, offsets, jumps }
        };
        (stat, chronology)
    }

    fn match_segment(index: &Index, sn: usize, hash: &Hash) -> Option<(Stat, Chronology)> {
        index.source().blocks.get(sn).and_then(|block| {
            block.hash.as_ref().and_then(|h| {
                if h == hash {
                    Some((block.stat.clone(), block.chronology.clone()))
                } else {
                    None
                }
            })
        })
    }
}

// ---

pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

// ---

#[cfg_attr(test, automock)]
pub trait FileSystem {
    fn canonicalize(&self, path: &PathBuf) -> io::Result<PathBuf>;
    fn metadata(&self, path: &PathBuf) -> io::Result<fs::Metadata>;
    fn exists(&self, path: &PathBuf) -> io::Result<bool>;
    fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadSeek + Send + Sync>>;
    fn create(&self, path: &PathBuf) -> io::Result<Box<dyn Write + Send + Sync>>;
}

// ---

#[derive(Default)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn canonicalize(&self, path: &PathBuf) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn metadata(&self, path: &PathBuf) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }

    fn exists(&self, path: &PathBuf) -> io::Result<bool> {
        fs::exists(path)
    }

    fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadSeek + Send + Sync>> {
        Ok(Box::new(File::open(path)?))
    }

    fn create(&self, path: &PathBuf) -> io::Result<Box<dyn Write + Send + Sync>> {
        Ok(Box::new(File::create(path)?))
    }
}

// ---

// Contains index information for a single source file.
#[derive(Debug)]
pub struct Index {
    source: SourceFile,
}

impl Index {
    /// Returns index information for the source file.
    pub fn source(&self) -> &SourceFile {
        &self.source
    }

    /// Loads the index.
    pub fn load(input: &mut Reader) -> Result<Index> {
        Header::load(input)?.validate()?;
        let message = read_message(input, message::ReaderOptions::new())?;
        let root: schema::root::Reader = message.get_root()?;
        let source = root.get_source()?;
        let modified = source.get_modified();
        Ok(Index {
            source: SourceFile {
                size: source.get_size(),
                path: source.get_path()?.to_string()?,
                modified: (modified.get_sec(), modified.get_nsec()),
                stat: Self::load_stat(source.get_index()?),
                blocks: Self::load_blocks(source)?,
            },
        })
    }

    /// Saves the index.
    pub fn save(&self, output: &mut Writer) -> Result<()> {
        let header = Header::new();
        header.save(output)?;
        let mut message = capnp::message::Builder::new_default();
        let root: schema::root::Builder = message.init_root();
        let mut source = root.init_source();
        source.set_size(self.source.size);
        source.set_path(&self.source.path);
        let mut modified = source.reborrow().init_modified();
        modified.set_sec(self.source.modified.0);
        modified.set_nsec(self.source.modified.1);
        let mut index = source.reborrow().init_index();
        Self::save_stat(index.reborrow(), &self.source.stat);
        self.save_blocks(source)?;
        capnp::serialize::write_message(output, &message)?;
        Ok(())
    }

    fn load_stat(index: schema::index::Reader) -> Stat {
        let lines = index.get_lines();
        let ts = index.get_timestamps();
        let flags = index.get_flags();
        Stat {
            flags,
            lines_valid: lines.get_valid(),
            lines_invalid: lines.get_invalid(),
            ts_min_max: if flags & schema::FLAG_HAS_TIMESTAMPS != 0 {
                Some((
                    Timestamp {
                        sec: ts.get_min().get_sec(),
                        nsec: ts.get_min().get_nsec(),
                    },
                    Timestamp {
                        sec: ts.get_max().get_sec(),
                        nsec: ts.get_max().get_nsec(),
                    },
                ))
            } else {
                None
            },
        }
    }

    fn save_stat(mut index: schema::index::Builder, stat: &Stat) {
        index.set_flags(stat.flags);
        let mut lines = index.reborrow().init_lines();
        lines.set_valid(stat.lines_valid);
        lines.set_invalid(stat.lines_invalid);
        if let Some((min, max)) = stat.ts_min_max {
            let mut timestamps = index.init_timestamps();
            let mut ts_min = timestamps.reborrow().init_min();
            ts_min.set_sec(min.sec);
            ts_min.set_nsec(min.nsec);
            let mut ts_max = timestamps.init_max();
            ts_max.set_sec(max.sec);
            ts_max.set_nsec(max.nsec);
        }
    }

    fn load_blocks(source: schema::source_file::Reader) -> Result<Vec<SourceBlock>> {
        let blocks = source.get_blocks()?;
        let mut result = Vec::with_capacity(blocks.len().try_into()?);
        for block in blocks.iter() {
            result.push(SourceBlock {
                offset: block.get_offset(),
                size: block.get_size(),
                stat: Self::load_stat(block.get_index()?),
                chronology: Self::load_chronology(block.get_chronology()?)?,
                hash: Self::load_hash(block.get_hash()?)?,
            })
        }
        Ok(result)
    }

    fn save_blocks(&self, source: schema::source_file::Builder) -> Result<()> {
        let mut blocks = source.init_blocks(self.source.blocks.len().try_into()?);
        for (i, source_block) in self.source.blocks.iter().enumerate() {
            let mut block = blocks.reborrow().get(i.try_into()?);
            block.set_offset(source_block.offset);
            block.set_size(source_block.size);
            Self::save_stat(block.reborrow().init_index(), &source_block.stat);
            Self::save_chronology(block.reborrow().init_chronology(), &source_block.chronology)?;
            Self::save_hash(block.init_hash(), &source_block.hash)?;
        }
        Ok(())
    }

    fn load_chronology(chronology: schema::chronology::Reader) -> Result<Chronology> {
        let bitmap = chronology.get_bitmap()?;
        let bitmap = {
            let mut result = Vec::with_capacity(bitmap.len().try_into()?);
            for i in 0..bitmap.len() {
                result.push(bitmap.get(i));
            }
            result
        };
        let offsets = chronology.get_offsets();
        let offsets = {
            let bytes = offsets.get_bytes()?;
            let jumps = offsets.get_jumps()?;
            if bytes.len() != jumps.len() {
                return Err(Error::InconsistentIndex {
                    details: "chronology offsets length mismatch".into(),
                });
            }
            let mut result = Vec::with_capacity(bytes.len().try_into()?);
            for i in 0..bytes.len() {
                result.push(OffsetPair {
                    bytes: bytes.get(i),
                    jumps: jumps.get(i),
                });
            }
            result
        };
        let jumps = chronology.get_jumps()?;
        let jumps = {
            let mut result = Vec::with_capacity(jumps.len().try_into()?);
            for i in 0..jumps.len() {
                result.push(jumps.get(i));
            }
            result
        };
        Ok(Chronology { bitmap, offsets, jumps })
    }

    fn save_chronology(mut to: schema::chronology::Builder, from: &Chronology) -> Result<()> {
        let mut bitmap = to.reborrow().init_bitmap(from.bitmap.len().try_into()?);
        for (i, value) in from.bitmap.iter().enumerate() {
            bitmap.set(i as u32, value.clone());
        }
        let n = from.offsets.len().try_into()?;
        let mut offsets = to.reborrow().init_offsets();
        {
            let mut bytes = offsets.reborrow().init_bytes(n);
            for (i, pair) in from.offsets.iter().enumerate() {
                bytes.set(i as u32, pair.bytes.clone());
            }
        }
        let mut jumps = offsets.reborrow().init_jumps(n);
        for (i, pair) in from.offsets.iter().enumerate() {
            jumps.set(i as u32, pair.jumps.clone());
        }
        let mut jumps = to.init_jumps(from.jumps.len().try_into()?);
        for (i, value) in from.jumps.iter().enumerate() {
            jumps.set(i as u32, value.clone());
        }
        Ok(())
    }

    fn load_hash(hash: schema::hash::Reader) -> Result<Option<Hash>> {
        match hash.get_algorithm().ok() {
            Some(schema::HashAlgorithm::Sha256) => {
                let value = hash.get_value()?;
                if value.len() == 32 {
                    Ok(Some(Hash::Sha256(value.try_into().unwrap())))
                } else {
                    Ok(None)
                }
            }
            Some(schema::HashAlgorithm::WyHash) => {
                let value = hash.get_value()?;
                if value.len() == 8 {
                    Ok(Some(Hash::WyHash(u64::from_be_bytes(value.try_into().unwrap()))))
                } else {
                    Ok(None)
                }
            }
            Some(schema::HashAlgorithm::GxHash64) => Ok(None),
            None => Ok(None),
        }
    }

    fn save_hash(mut to: schema::hash::Builder, from: &Option<Hash>) -> Result<()> {
        match from {
            Some(Hash::Sha256(value)) => {
                to.set_algorithm(schema::HashAlgorithm::Sha256);
                to.set_value(value.as_slice());
            }
            Some(Hash::WyHash(value)) => {
                to.set_algorithm(schema::HashAlgorithm::WyHash);
                to.set_value(&value.to_be_bytes());
            }
            Some(Hash::GxHash64(_)) => (),
            None => (),
        }
        Ok(())
    }
}

// ---

/// SourceFile contains index data of scanned source log file.
#[derive(Debug)]
pub struct SourceFile {
    pub size: u64,
    pub path: String,
    pub modified: (i64, u32),
    pub stat: Stat,
    pub blocks: Vec<SourceBlock>,
}

// ---

/// SourceBlock contains index data of a block in a scanned source log file.
#[derive(Debug, Clone)]
pub struct SourceBlock {
    pub offset: u64,
    pub size: u32,
    pub stat: Stat,
    pub chronology: Chronology,
    pub hash: Option<Hash>,
}

impl SourceBlock {
    /// Returns a new SourceBlock.
    pub fn new(offset: u64, size: u32, stat: Stat, chronology: Chronology, hash: Option<Hash>) -> Self {
        Self {
            offset,
            size,
            stat,
            chronology,
            hash,
        }
    }

    /// Returns true if SourceBlock contains at least one line matching the given level or higher level.
    pub fn match_level(&self, level: Level) -> bool {
        let mut flags = 0;
        for &l in &[Level::Error, Level::Warning, Level::Info, Level::Debug] {
            flags |= level_to_flag(l);
            if l == level {
                break;
            }
        }
        self.stat.flags & flags != 0
    }

    /// Returns true if this SourceBlock overlaps by time with other SourceBlock.
    pub fn overlaps_by_time(&self, other: &SourceBlock) -> bool {
        if let (Some(ts1), Some(ts2)) = (self.stat.ts_min_max, other.stat.ts_min_max) {
            (ts2.0 >= ts1.0 && ts2.0 <= ts1.1) || (ts2.1 >= ts1.0 && ts2.1 <= ts1.1)
        } else {
            false
        }
    }
}

// ---

/// Stat contains statistical information over a file or over a block.
#[derive(Debug, Clone)]
pub struct Stat {
    pub flags: u64,
    pub lines_valid: u64,
    pub lines_invalid: u64,
    pub ts_min_max: Option<(Timestamp, Timestamp)>,
}

impl Stat {
    /// New returns a new Stat.
    pub fn new() -> Self {
        Self {
            flags: 0,
            lines_valid: 0,
            lines_invalid: 0,
            ts_min_max: None,
        }
    }

    /// Adds information about a single valid line.
    pub fn add_valid(&mut self, ts: Option<Timestamp>, flags: u64) {
        self.ts_min_max = min_max_opt(self.ts_min_max, ts.and_then(|ts| Some((ts, ts))));
        self.flags |= flags;
        self.lines_valid += 1;
        if self.ts_min_max.is_some() {
            self.flags |= schema::FLAG_HAS_TIMESTAMPS;
        }
    }

    /// Counts a single invalid line.
    pub fn add_invalid(&mut self) {
        self.lines_invalid += 1;
    }

    /// Merges with other Stat.
    pub fn merge(&mut self, other: &Self) {
        self.lines_valid += other.lines_valid;
        self.lines_invalid += other.lines_invalid;
        self.flags |= other.flags;
        self.ts_min_max = min_max_opt(self.ts_min_max, other.ts_min_max);
    }
}

// ---

/// Chronology contains information about ordering of log messages by timestamp in a SourceBlock.
#[derive(Clone)]
pub struct Chronology {
    pub bitmap: Vec<u64>,
    pub offsets: Vec<OffsetPair>,
    pub jumps: Vec<u32>,
}

impl Default for Chronology {
    fn default() -> Self {
        Self {
            bitmap: Vec::new(),
            offsets: Vec::new(),
            jumps: Vec::new(),
        }
    }
}

impl fmt::Debug for Chronology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chronology")
            .field("bitmap", &AsHex(&self.bitmap))
            .field("offsets", &self.offsets)
            .field("jumps", &self.jumps)
            .finish()
    }
}

// ---

/// OffsetPair contains information offsets for a line in bytes in a SourceBlock and in a jump table.
#[derive(Debug, Clone, Copy)]
pub struct OffsetPair {
    pub bytes: u32,
    pub jumps: u32,
}

// ---

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Header {
    magic: u64,
    version: u64,
    size: u64,
    checksum: u64,
}

impl Header {
    fn new() -> Self {
        Self {
            magic: VALID_MAGIC,
            version: CURRENT_VERSION,
            size: 0,
            checksum: 0,
        }
    }

    fn load(reader: &mut Reader) -> Result<Self> {
        Ok(bincode::deserialize_from(reader)?)
    }

    fn is_valid(&self) -> bool {
        self.magic == VALID_MAGIC && self.version == CURRENT_VERSION
    }

    fn validate(&self) -> Result<()> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(Error::InvalidIndexHeader)
        }
    }

    fn save(&self, writer: &mut Writer) -> Result<()> {
        Ok(bincode::serialize_into(writer, &self)?)
    }
}

// ---

struct Metadata {
    len: u64,
    modified: (i64, u32),
}

impl TryFrom<&fs::Metadata> for Metadata {
    type Error = io::Error;

    fn try_from(value: &fs::Metadata) -> io::Result<Self> {
        Ok(Self {
            len: value.len(),
            modified: ts(value.modified()?),
        })
    }
}

// ---

struct AsHex<T>(T);

impl<T: fmt::Debug> fmt::Debug for AsHex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x?}", &self.0)
    }
}

// ---

fn min_max_opt<T: Ord>(v1: Option<(T, T)>, v2: Option<(T, T)>) -> Option<(T, T)> {
    match (v1, v2) {
        (Some(v1), Some(v2)) => Some((min(v1.0, v2.0), max(v1.1, v2.1))),
        (Some(v1), None) => Some(v1),
        (None, Some(v2)) => Some(v2),
        (None, None) => None,
    }
}

fn ts(ts: SystemTime) -> (i64, u32) {
    match ts.duration_since(UNIX_EPOCH) {
        Ok(ts) => (ts.as_secs() as i64, ts.subsec_nanos()),
        Err(_) => match UNIX_EPOCH.duration_since(ts) {
            Ok(ts) => (-(ts.as_secs() as i64), ts.subsec_nanos()),
            Err(_) => (0, 0),
        },
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn strip<'a>(slice: &'a [u8], ch: u8) -> &'a [u8] {
    let n = slice.len();
    if n == 0 {
        slice
    } else if slice[n - 1] == ch {
        &slice[..n - 1]
    } else {
        slice
    }
}

fn level_to_flag(level: Level) -> u64 {
    match level {
        Level::Debug => schema::FLAG_LEVEL_DEBUG,
        Level::Info => schema::FLAG_LEVEL_INFO,
        Level::Warning => schema::FLAG_LEVEL_WARNING,
        Level::Error => schema::FLAG_LEVEL_ERROR,
    }
}

fn rtrim<'a>(s: &'a [u8], c: u8) -> &'a [u8] {
    if s.len() > 0 && s[s.len() - 1] == c {
        &s[..s.len() - 1]
    } else {
        s
    }
}

const VALID_MAGIC: u64 = 0x5845444e492d4c48;
const CURRENT_VERSION: u64 = 1;

/*
---
 TS encoding proposal
---
 00 - seconds
 01 - milliseconds
 10 - microseconds
 11 - nanoseconds
 xx0 - [*]seconds in next 5 bits
 xx10 - [*]seconds in next 12 bits
 xx110 - [*]seconds in next 19 bits
 xx1110 - [*]seconds in next 26 bits
 xx111100 - [*]seconds in next 32 bits
 xx111101 - [*]seconds in next 40/40/48/48 bits
 xx111110 - [*]seconds in next 48/56/56/64 bits
 xx111111 - [*]seconds in next 64/80/88/96 bits
---
 */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_file_success() {
        use io::Cursor;
        let indexer = Indexer::new(
            1,
            PathBuf::from("/tmp/cache"),
            IndexerSettings {
                fs: MockFileSystem::new(),
                buffer_size: nonzero!(1024u32).into(),
                max_message_size: nonzero!(1024u32).into(),
                ..Default::default()
            },
        );
        let data = concat!(
            "ts=2023-12-04T10:01:07.091243+01:00 msg=msg1\n",
            "ts=2023-12-04T10:01:07.091252+01:00 msg=msg2\n",
            "ts=2023-12-04T10:01:07.091633+01:00 msg=msg3\n",
        );
        let mut input = Cursor::new(data);
        let mut output = Cursor::new(Vec::new());
        let index = indexer
            .process_file(
                &PathBuf::from("/tmp/test.log"),
                Metadata {
                    len: data.len() as u64,
                    modified: (1714739340, 0),
                },
                &mut input,
                &mut output,
                None,
            )
            .unwrap();
        assert_ne!(output.into_inner().len(), 0);
        assert_eq!(index.source.size, data.len() as u64);
        assert_eq!(index.source.path, "/tmp/test.log");
        assert_eq!(index.source.modified, (1714739340, 0));
        assert_eq!(index.source.stat.lines_valid, 3);
        assert_eq!(index.source.stat.lines_invalid, 0);
        assert_eq!(index.source.stat.flags, schema::FLAG_HAS_TIMESTAMPS);
        assert_eq!(
            index.source.stat.ts_min_max,
            Some((
                Timestamp::from((1701680467, 91243000)),
                Timestamp::from((1701680467, 91633000))
            ))
        );
        assert_eq!(index.source.blocks.len(), 1);
        assert_eq!(index.source.blocks[0].stat.lines_valid, 3);
        assert_eq!(index.source.blocks[0].stat.lines_invalid, 0);
        assert_eq!(index.source.blocks[0].stat.flags, schema::FLAG_HAS_TIMESTAMPS);
        assert_eq!(
            index.source.blocks[0].stat.ts_min_max,
            Some((
                Timestamp::from((1701680467, 91243000)),
                Timestamp::from((1701680467, 91633000))
            ))
        );
    }

    #[test]
    fn test_process_file_error() {
        use io::Cursor;
        let fs = MockFileSystem::new();
        let indexer = Indexer::new(
            1,
            PathBuf::from("/tmp/cache"),
            IndexerSettings {
                fs,
                buffer_size: nonzero!(1024u32).into(),
                max_message_size: nonzero!(1024u32).into(),
                ..Default::default()
            },
        );
        let mut input = FailingReader;
        let mut output = Cursor::new(Vec::new());
        let result = indexer.process_file(
            &PathBuf::from("/tmp/test.log"),
            Metadata {
                len: 135,
                modified: (1714739340, 0),
            },
            &mut input,
            &mut output,
            None,
        );
        assert_eq!(result.is_err(), true);
        assert_eq!(output.into_inner().len(), 0);
    }

    // ---

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::Other, "read error"))
        }
    }
}
