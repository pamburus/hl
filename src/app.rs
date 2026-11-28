// std imports
use std::{
    cmp::{Reverse, max},
    convert::{TryFrom, TryInto},
    fs,
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    num::NonZeroUsize,
    ops::Range,
    path::PathBuf,
    str,
    sync::Arc,
    time::{Duration, Instant},
};

// third-party imports
use closure::closure;
use crossbeam_channel::{self as channel, Receiver, RecvTimeoutError, Sender};
use crossbeam_utils::thread;
use deko::bufread::AnyDecoder;
use enumset::{EnumSet, enum_set};
use enumset_ext::EnumSetExt;
use itertools::{Itertools, izip};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// local imports
use crate::{
    ExactIncludeExcludeKeyFilter, IncludeExcludeKeyFilter,
    appdirs::AppDirs,
    datefmt::{DateTimeFormat, DateTimeFormatter},
    error::*,
    filtering::{MatchOptions, NoNormalizing},
    fmtx::{Adjustment, Alignment, Padding, aligned},
    formatting::{
        DynRecordWithSourceFormatter, Expansion, RawRecordFormatter, RecordFormatterBuilder, RecordWithSourceFormatter,
    },
    help,
    index::{Indexer, IndexerSettings, Timestamp},
    input::{BlockEntry, InputDescriptor, InputHolder, InputReference, Stream, TaggedStream},
    model::{Filter, Origin, Parser, ParserSettings, RawRecord, Record, RecordFilter, RecordWithSourceConstructor},
    output::{BufFactory, Chunk, OutputBuf},
    query::Query,
    scanning::{
        Delimit, Delimiter, Newline, PartialPlacement, Scanner, SearchExt, Segment, SegmentBuf, SegmentBufFactory,
    },
    settings::{AsciiMode, ExpansionMode, FieldShowOption, Fields, Formatting, InputInfo, ResolvedPunctuation},
    theme::{Element, StylingPush, SyncIndicatorPack, Theme},
    themecfg,
    timezone::Tz,
    vfs::LocalFileSystem,
};

// test imports
#[cfg(test)]
use crate::testing::Sample;

// ---

mod follow;

use follow::{Due, MergeWindow, MergeWriter};

// ---

pub type Output = dyn Write + Send + Sync;
pub type InputInfoSet = EnumSet<InputInfo>;

// TODO: merge Options to Settings and replace Options with Settings.

// ---

pub struct Options {
    pub theme: Arc<Theme>,
    pub time_format: DateTimeFormat,
    pub raw: bool,
    pub raw_fields: bool,
    pub allow_prefix: bool,
    pub buffer_size: NonZeroUsize,
    pub max_message_size: NonZeroUsize,
    pub concurrency: usize,
    pub filter: Arc<AdvancedFilter>,
    pub fields: FieldOptions,
    pub formatting: Formatting,
    pub time_zone: Tz,
    pub hide_empty_fields: bool,
    pub sort: bool,
    pub follow: bool,
    pub sync_interval: Duration,
    pub input_info: InputInfoSet,
    pub input_format: Option<InputFormat>,
    pub dump_index: bool,
    pub app_dirs: Option<AppDirs>,
    pub tail: u64,
    pub delimiter: Delimiter,
    pub unix_ts_unit: Option<UnixTimestampUnit>,
    pub flatten: bool,
    pub ascii: AsciiMode,
    pub expand: ExpansionMode,
    pub output_delimiter: String,
}

impl Options {
    #[cfg(test)]
    fn with_theme(self, theme: Arc<Theme>) -> Self {
        Self { theme, ..self }
    }

    #[cfg(test)]
    fn with_fields(self, fields: FieldOptions) -> Self {
        Self { fields, ..self }
    }

    #[cfg(test)]
    fn with_raw_fields(self, raw_fields: bool) -> Self {
        Self { raw_fields, ..self }
    }

    #[cfg(test)]
    fn with_raw(self, raw: bool) -> Self {
        Self { raw, ..self }
    }

    #[cfg(test)]
    fn with_sort(self, sort: bool) -> Self {
        Self { sort, ..self }
    }

    #[cfg(test)]
    fn with_filter(self, filter: Arc<AdvancedFilter>) -> Self {
        Self { filter, ..self }
    }

    #[cfg(test)]
    fn with_input_info(self, input_info: InputInfoSet) -> Self {
        Self { input_info, ..self }
    }

    #[cfg(test)]
    fn with_expansion(self, expand: ExpansionMode) -> Self {
        Self { expand, ..self }
    }
}

#[derive(Default)]
pub struct AdvancedFilter {
    pub basic: Filter,
    pub query: Option<Query>,
}

impl AdvancedFilter {
    pub fn new(basic: Filter, query: Option<Query>) -> Self {
        Self { basic, query }
    }

    pub fn is_empty(&self) -> bool {
        self.basic.is_empty() && self.query.is_none()
    }
}

impl RecordFilter for AdvancedFilter {
    #[inline]
    fn apply(&self, record: &Record) -> bool {
        self.basic.apply(record) && self.query.apply(record)
    }
}

impl From<&Arc<AdvancedFilter>> for Query {
    fn from(options: &Arc<AdvancedFilter>) -> Self {
        if options.is_empty() {
            Query::default()
        } else {
            Query::new(options.clone())
        }
    }
}

impl From<Filter> for AdvancedFilter {
    fn from(filter: Filter) -> Self {
        Self::new(filter, None)
    }
}

impl From<Filter> for Arc<AdvancedFilter> {
    fn from(filter: Filter) -> Self {
        Self::new(filter.into())
    }
}

#[derive(Default)]
pub struct FieldOptions {
    pub filter: Arc<IncludeExcludeKeyFilter>,
    pub settings: Fields,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum InputFormat {
    Json,
    Logfmt,
}

// ---

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum UnixTimestampUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl UnixTimestampUnit {
    pub fn guess(ts: i64) -> Self {
        match ts {
            Self::TS_UNIX_AUTO_S_MIN..=Self::TS_UNIX_AUTO_S_MAX => Self::Seconds,
            Self::TS_UNIX_AUTO_MS_MIN..=Self::TS_UNIX_AUTO_MS_MAX => Self::Milliseconds,
            Self::TS_UNIX_AUTO_US_MIN..=Self::TS_UNIX_AUTO_US_MAX => Self::Microseconds,
            _ => Self::Nanoseconds,
        }
    }

    const TS_UNIX_AUTO_S_MIN: i64 = -62135596800;
    const TS_UNIX_AUTO_S_MAX: i64 = 253402300799;
    const TS_UNIX_AUTO_MS_MIN: i64 = Self::TS_UNIX_AUTO_S_MIN * 1000;
    const TS_UNIX_AUTO_MS_MAX: i64 = Self::TS_UNIX_AUTO_S_MAX * 1000;
    const TS_UNIX_AUTO_US_MIN: i64 = Self::TS_UNIX_AUTO_MS_MIN * 1000;
    const TS_UNIX_AUTO_US_MAX: i64 = Self::TS_UNIX_AUTO_MS_MAX * 1000;
}

// ---

pub fn list_themes(
    dirs: &AppDirs,
    tags: Option<EnumSet<themecfg::Tag>>,
    mut formatter: impl help::Format,
) -> Result<()> {
    use themecfg::Tag;

    let items = Theme::list(dirs)?;

    let tags = tags.unwrap_or_default();
    let mut exclude = EnumSet::default();
    if !tags.contains(Tag::Base) {
        exclude.insert(Tag::Base);
    }
    if !tags.contains(Tag::Overlay) {
        exclude.insert(Tag::Overlay);
    }

    formatter.format_grouped_list(
        items
            .into_iter()
            .filter(|(name, _)| {
                themecfg::Theme::load(dirs, name)
                    .ok()
                    .map(|theme| theme.tags.includes(tags) && !theme.tags.intersects(exclude))
                    .unwrap_or(false)
            })
            .sorted_by_key(|x| (x.1.origin, x.0.clone()))
            .chunk_by(|x| x.1.origin)
            .into_iter()
            .map(|(origin, group)| (origin, group.map(|x| x.0))),
    )?;
    Ok(())
}

// ---

pub struct App {
    options: Options,
    punctuation: Arc<ResolvedPunctuation>,
    formatter: DynRecordWithSourceFormatter,
}

impl App {
    pub fn new(mut options: Options) -> Self {
        if options.raw && options.input_info.intersects(InputInfo::None | InputInfo::Auto) {
            options.input_info = InputInfo::None.into()
        }
        options.input_info = InputInfo::resolve(options.input_info);

        let punctuation = Arc::new(options.formatting.punctuation.resolve(options.ascii));

        let formatter = Self::new_formatter(&options, punctuation.clone());

        Self {
            options,
            punctuation,
            formatter,
        }
    }

    pub fn run(&self, inputs: Vec<InputHolder>, output: &mut Output) -> Result<()> {
        if self.options.follow {
            self.follow(inputs, output)
        } else if self.options.sort {
            self.sort(inputs, output)
        } else {
            self.cat(inputs, output)
        }
    }

    fn cat(&self, inputs: Vec<InputHolder>, output: &mut Output) -> Result<()> {
        let descriptors = inputs
            .iter()
            .map(|x| &x.reference)
            .enumerate()
            .map(Into::into)
            .collect_vec();
        let input_badges = self.input_badges(&descriptors);

        let inputs = inputs
            .into_iter()
            .map(|x| x.open())
            .collect::<std::io::Result<Vec<_>>>()?;

        let n = self.options.concurrency;
        let sfi = Arc::new(SegmentBufFactory::new(self.options.buffer_size.into()));
        let bfo = Arc::new(BufFactory::new(self.options.buffer_size.into()));
        let parser = self.parser();
        thread::scope(|scope| -> Result<()> {
            // prepare receive/transmit channels for input data
            let (txi, rxi): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded(1)).unzip();
            // prepare receive/transmit channels for output data
            let (txo, rxo): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded::<(usize, Chunk)>(1)).unzip();
            // spawn reader thread
            let reader = scope.spawn(closure!(clone sfi, |_| -> Result<()> {
                let mut tx = StripedSender::new(txi);
                let scanner = Scanner::new(sfi, &self.options.delimiter);
                for (i, mut input) in inputs.into_iter().enumerate() {
                    let mut offset = 0;
                    for item in scanner.items(&mut input.stream.as_sequential()).with_max_segment_size(self.options.max_message_size.into()) {
                        let item = item?;
                        let len = item.data().len();
                        if tx.send((i, offset, item)).is_none() {
                            break;
                        }
                        offset += len as u64;
                    }
                }
                Ok(())
            }));
            // spawn processing threads
            for (rxi, txo) in izip!(rxi, txo) {
                scope.spawn(closure!(ref bfo, ref parser, ref descriptors, ref input_badges, |_| {
                    let mut processor = self.new_segment_processor(parser);
                    let show_unparsed = self.allow_unparsed_data();
                    for (i, offset, segment) in rxi.iter() {
                        let prefix = input_badges.as_ref().map(|b|b[i].as_str()).unwrap_or("");
                        match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                let origin = Origin::new(&descriptors[i], offset, segment.data().len());
                                processor.process(segment.data(), &mut buf, origin, prefix, None, &mut RecordIgnorer{});
                                drop(segment);
                                if txo.send((i, Chunk::Output(buf))).is_err() {
                                    break;
                                };
                            }
                            Segment::Incomplete(segment, part, delimiter) => {
                                // A log entry too large to be read at once is shown as is, part by part,
                                // like other text that is not a record. Every segment is sent, even
                                // with nothing to show, as the writer takes them in order.
                                // The last part ends with the output delimiter instead of the input one, if any.
                                let segment = if !show_unparsed {
                                    drop(segment);
                                    SegmentBuf::from(b"")
                                } else if part == PartialPlacement::Last {
                                    end_last_part(segment, delimiter, self.options.output_delimiter.as_bytes())
                                } else {
                                    segment
                                };
                                if txo.send((i, Chunk::Segment(segment))).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }));
            }
            // spawn writer thread
            let writer = scope.spawn(|_| -> Result<()> {
                for (_, buf) in StripedReceiver::new(rxo) {
                    output.write_all(buf.data())?;
                }
                Ok(())
            });
            // collect errors from reader and writer threads
            reader.join().unwrap()?;
            writer.join().unwrap()?;
            Ok(())
        })
        .unwrap()?;

        Ok(())
    }

    fn sort(&self, inputs: Vec<InputHolder>, output: &mut Output) -> Result<()> {
        let mut output = BufWriter::new(output);
        let indexer_settings = IndexerSettings {
            buffer_size: self.options.buffer_size.try_into()?,
            max_message_size: self.options.max_message_size.try_into()?,
            fields: &self.options.fields.settings.predefined,
            delimiter: self.options.delimiter.clone(),
            allow_prefix: self.options.allow_prefix,
            unix_ts_unit: self.options.unix_ts_unit,
            format: self.options.input_format,
            ..IndexerSettings::with_fs(LocalFileSystem)
        };
        let param_hash = hex::encode(indexer_settings.hash()?);
        let cache_dir = self
            .options
            .app_dirs
            .as_ref()
            .map(|dirs| dirs.cache_dir.clone())
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join(param_hash);
        fs::create_dir_all(&cache_dir)?;

        let indexer = Indexer::new(self.options.concurrency, cache_dir, indexer_settings);
        let descriptors = inputs
            .iter()
            .map(|x| &x.reference)
            .enumerate()
            .map(Into::into)
            .collect_vec();
        let input_badges = self.input_badges(&descriptors);

        let inputs = inputs
            .into_iter()
            .map(|x| x.index(&indexer, self.options.delimiter.clone()))
            .collect::<Result<Vec<_>>>()?;

        if self.options.dump_index {
            for input in inputs {
                for block in input.into_blocks().sorted() {
                    writeln!(output, "block at {} with size {}", block.offset(), block.size())?;
                    writeln!(output, "{:#?}", block.source_block())?;
                    let block_offset = block.offset();
                    for line in block.into_entries()? {
                        writeln!(
                            output,
                            "{} bytes at {} (absolute {})",
                            line.len(),
                            line.offset(),
                            block_offset + line.offset() as u64
                        )?;
                    }
                }
            }
            return Ok(());
        }

        let n = self.options.concurrency;
        let parser = self.parser();
        thread::scope(|scope| -> Result<()> {
            // prepare transmit/receive channels for data produced by pusher thread
            let (txp, rxp): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded(1)).unzip();
            // prepare transmit/receive channels for data produced by worker threads
            let (txw, rxw): (Vec<_>, Vec<_>) = (0..n)
                .map(|_| channel::bounded::<(OutputBlock, usize, usize)>(1))
                .unzip();
            // spawn pusher thread
            let pusher = scope.spawn(closure!(|_| -> Result<()> {
                let mut blocks: Vec<_> = inputs
                    .into_iter()
                    .enumerate()
                    .flat_map(|(i, input)| input.into_blocks().map(move |block| (block, i)))
                    .filter_map(|(block, i)| {
                        let src = block.source_block();
                        if src.stat.entries_valid == 0 {
                            return None;
                        }
                        if let Some((ts_min, ts_max)) = src.stat.ts_min_max {
                            if let Some(until) = self.options.filter.basic.until
                                && ts_min > until.into()
                            {
                                return None;
                            }
                            if let Some(since) = self.options.filter.basic.since
                                && ts_max < since.into()
                            {
                                return None;
                            }
                            if let Some(level) = self.options.filter.basic.level
                                && !src.match_level(level)
                            {
                                return None;
                            }
                            let offset = block.offset();
                            Some((block, ts_min, ts_max, i, offset))
                        } else {
                            None
                        }
                    })
                    .collect();

                blocks.sort_by(|a, b| (a.1, a.2, a.3, a.4).partial_cmp(&(b.1, b.2, b.3, b.4)).unwrap());

                let mut output = StripedSender::new(txp);
                for (j, (block, ts_min, _, i, _)) in blocks.into_iter().enumerate() {
                    if output.send((block, ts_min, i, j)).is_none() {
                        break;
                    }
                }
                Ok(())
            }));
            // spawn worker threads
            let mut workers = Vec::with_capacity(n);
            for (rxp, txw) in izip!(rxp, txw) {
                workers.push(scope.spawn(closure!(
                    ref parser,
                    ref descriptors,
                    ref input_badges,
                    |_| -> Result<()> {
                        let mut processor = self.new_segment_processor(parser);
                        for (block, ts_min, i, j) in rxp.iter() {
                            let block_offset = block.offset();
                            let mut buf = Vec::with_capacity(2 * usize::try_from(block.size())?);
                            let mut items = Vec::with_capacity(2 * usize::try_from(block.entries_valid())?);
                            for line in block.into_entries()? {
                                if line.is_empty() {
                                    continue;
                                }
                                let prefix = input_badges.as_ref().map(|b| b[i].as_str()).unwrap_or("");
                                let origin =
                                    Origin::new(&descriptors[i], block_offset + line.offset() as u64, line.len());
                                processor.process(
                                line.bytes(),
                                &mut buf,
                                origin,
                                prefix,
                                Some(1),
                                &mut |record: &Record, location: Range<usize>| {
                                    if let Some(ts) = &record.ts {
                                        if let Some(unix_ts) = ts.unix_utc() {
                                            items.push((unix_ts.into(), location));
                                        } else {
                                            log::warn!(
                                                "skipped a message because its timestamp could not be parsed: {:#?}",
                                                ts.raw()
                                            )
                                        }
                                    }
                                },
                            );
                            }

                            let buf = Arc::new(buf);
                            if txw.send((OutputBlock { ts_min, buf, items }, i, j)).is_err() {
                                break;
                            }
                        }
                        Ok(())
                    }
                )));
            }
            // spawn merger thread
            let merger = scope.spawn(|_| -> Result<()> {
                let mut input = StripedReceiver::new(rxw);
                let (mut tsi, mut tso) = (None, None);
                let mut workspace = Vec::new();
                let mut done = false;

                // Workspace rules
                // 1. Can process messages up to max `ts_min` of the blocks in workspace
                // 2. Can process any messages if workspace is complete (has all remaining blocks)
                // 3. Should be sorted by (head (next line timestamp), input, block number, offset)

                loop {
                    while tso >= tsi || workspace.is_empty() {
                        if let Some((block, i, j)) = input.next() {
                            tsi = Some(block.ts_min);
                            tso = tso.or(tsi);
                            let mut tail = block.into_lines();
                            let head = tail.next();
                            if let Some(head) = head {
                                workspace.push((head, tail, i, j));
                            }
                        } else {
                            done = true;
                            break;
                        }
                    }

                    if done && workspace.is_empty() {
                        break;
                    }

                    workspace.sort_by_key(|v| Reverse(((v.0).0, v.2, v.3, (v.0).1.offset())));
                    let k = workspace.len() - 1;
                    let item = &mut workspace[k];
                    let ts = (item.0).0;
                    tso = Some(ts);
                    if tso >= tsi && !done {
                        continue;
                    }
                    output.write_all((item.0).1.bytes())?;
                    output.write_all(self.options.output_delimiter.as_bytes())?;
                    match item.1.next() {
                        Some(head) => item.0 = head,
                        None => drop(workspace.swap_remove(k)),
                    }
                }

                Ok(())
            });

            pusher.join().unwrap()?;
            for worker in workers {
                worker.join().unwrap()?;
            }
            merger.join().unwrap()?;

            Ok(())
        })
        .unwrap()?;

        Ok(())
    }

    fn prepare_follow_badges(&self, inputs: &[InputDescriptor]) -> FollowBadges {
        let si = SyncIndicator::from(&self.options.theme.indicators.sync);

        let mut badges = self.input_badges(inputs);
        if let Some(badges) = &mut badges {
            for badge in badges.iter_mut() {
                *badge = format!("{}{}", si.placeholder, badge);
            }
        }

        FollowBadges { si, input: badges }
    }

    fn follow(&self, inputs: Vec<InputHolder>, output: &mut Output) -> Result<()> {
        let descriptors = inputs
            .iter()
            .map(|x| &x.reference)
            .enumerate()
            .map(Into::into)
            .collect_vec();
        let badges = self.prepare_follow_badges(&descriptors);

        let m = inputs.len();
        let n = self.options.concurrency;
        let parser = self.parser();
        let sfi = Arc::new(SegmentBufFactory::new(self.options.buffer_size.into()));
        let bfo = Arc::new(BufFactory::new(self.options.buffer_size.into()));
        let follow = fsmon::Follow::new(fsmon::FollowOptions::default())?;
        let stopper = follow.stopper();

        thread::scope(|scope| -> Result<()> {
            // prepare receive/transmit channels for input data
            let (txi, rxi) = channel::bounded(1);
            // prepare receive/transmit channels for output data
            let (txo, rxo) = channel::bounded(1);
            // spawn reader threads
            let mut readers = Vec::with_capacity(m);
            for (i, holder) in inputs.into_iter().enumerate() {
                let reader = scope.spawn(
                    closure!(clone sfi, clone txi, ref follow, ref stopper, |_| -> Result<()> {
                        let result = self.follow_input(follow, holder, &sfi, i, &txi);
                        // An error ends following, as a read error does with `tail -F`, while one opening a file is
                        // waited out in the input itself. It is reported only once all the inputs end, so the others
                        // are stopped.
                        if result.is_err() {
                            stopper.stop();
                        }
                        result
                    }),
                );
                readers.push(reader);
            }
            drop(txi);

            // spawn processing threads
            let mut workers = Vec::with_capacity(n);
            for _ in 0..n {
                let worker = scope.spawn(
                    closure!(ref bfo, ref parser, ref badges, ref descriptors, clone rxi, clone txo, |_| {
                        self.process_segments(parser, bfo, badges, descriptors, rxi, txo);
                    }),
                );
                workers.push(worker);
            }
            drop(txo);

            // spawn merger thread
            let merger = scope.spawn(|_| -> Result<()> {
                let result = self.merge_segments(&badges, rxo, output, n);
                // Nothing more can be shown, so the inputs are not followed any further.
                stopper.stop();
                result
            });

            let read = readers.into_iter().map(|reader| reader.join().unwrap()).collect_vec();

            for worker in workers {
                worker.join().unwrap();
            }

            // An error of the merger is reported first, as it stops the inputs, which may fail because of that.
            merger.join().unwrap()?;
            read.into_iter().collect()
        })
        .unwrap()?;

        Ok(())
    }

    /// Reads the input `i` of a follow, sending its segments to `txi`, until it ends or the follow stops.
    ///
    /// A followed file delivers a stream per file that appears at its path, the first one from its tail, and each of
    /// the others from its start. A compressed file is decoded, and so are the standard input and a FIFO, as when they
    /// are not followed.
    fn follow_input(
        &self,
        follow: &fsmon::Follow,
        holder: InputHolder,
        sfi: &Arc<SegmentBufFactory>,
        i: usize,
        txi: &Sender<(usize, usize, u64, Segment)>,
    ) -> Result<()> {
        let delimiter = &self.options.delimiter;
        let scanner = Scanner::new(sfi.clone(), delimiter.clone());
        let mut blocks = BlockScanner::new(&scanner, self.options.max_message_size.into());
        let reference = holder.reference.clone();
        let context =
            |e: io::Error| io::Error::new(e.kind(), format!("failed to follow {}: {e}", reference.description()));

        let (mut input, mut start, mut decode) = match (&holder.reference, holder.file()) {
            (InputReference::Stdin, _) if holder.stream.is_none() => (follow.add_stdin(), 0, true),
            (InputReference::File(path), Some(file)) => {
                // A link is followed as given, so that it is resolved again once it points at another file, as with `tail -F`.
                let path = path.original.clone();
                if holder.metadata().map_err(context)?.is_some_and(|meta| meta.is_file()) {
                    // The clone shares the position of the file, so it is where the tail starts.
                    let mut file = file.try_clone().map_err(context)?;
                    let mut input = holder
                        .open()
                        .and_then(|input| input.tail(self.options.tail, delimiter.clone()))
                        .map_err(context)?;
                    let position = input.position().map_err(context)?;
                    // A compressed file is decoded as a sequential stream, from its start.
                    let decode = matches!(input.stream, Stream::Sequential(_));
                    drop(input);
                    let start = if decode {
                        file.seek(SeekFrom::Start(0)).map_err(context)?
                    } else {
                        position
                    };
                    (follow.add_file(&path, file).map_err(context)?, start, decode)
                } else {
                    let file = holder.into_file().ok().expect("the file is held");
                    (follow.add_file(&path, file).map_err(context)?, 0, true)
                }
            }
            // A file that could not be opened as following started is waited for, and read from its start once it can
            // be, as with `tail -F`.
            (InputReference::File(path), None) if holder.stream.is_none() => {
                (follow.add_path_from_start(&path.original).map_err(context)?, 0, true)
            }
            // A stream given as is, rather than held by its reference, is read once to its end.
            _ => {
                let mut input = holder.open()?.tail(self.options.tail, delimiter.clone())?;
                blocks.start(input.position()?);
                return blocks.scan(&mut input.stream.as_sequential(), i, txi);
            }
        };

        while let Some(stream) = input.next_stream().map_err(context)? {
            blocks.start(start);
            start = 0;
            // An unterminated entry at the end of a stream is complete, as nothing more can come to finish it, and so
            // is one at the end of the decoded data.
            if decode {
                let mut decoder = AnyDecoder::new(BufReader::new(stream));
                blocks.scan(&mut TaggedStream::new(reference.clone(), &mut decoder), i, txi)?;
                // The decoded data of some formats, such as bzip2, ends where the compressed data does, which may come
                // before the end of the file, whose rest is skipped, so that the file is not taken as replaced and
                // read again.
                io::copy(
                    &mut TaggedStream::new(reference.clone(), decoder.get_mut()),
                    &mut io::sink(),
                )?;
            } else {
                blocks.scan(&mut TaggedStream::new(reference.clone(), stream), i, txi)?;
            }
            // The file that takes the place of this one is read from its start, and may be compressed.
            decode = true;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_segments(
        &self,
        parser: &Parser,
        bfo: &Arc<BufFactory>,
        badges: &FollowBadges,
        inputs: &[InputDescriptor],
        rxi: Receiver<(usize, usize, u64, Segment)>,
        txo: Sender<(usize, OutputBuf, TimestampIndex)>,
    ) {
        let mut processor = self.new_segment_processor(parser);
        for (i, j, offset, segment) in rxi.iter() {
            let prefix = badges.prefix(i);
            match segment {
                Segment::Complete(segment) => {
                    let mut buf = bfo.new_buf();
                    let mut index_builder = TimestampIndexBuilder {
                        result: TimestampIndex::new(j),
                    };
                    let origin = Origin::new(&inputs[i], offset, segment.data().len());
                    processor.process(segment.data(), &mut buf, origin, prefix, None, &mut index_builder);
                    drop(segment);
                    if txo.send((i, buf, index_builder.result)).is_err() {
                        return;
                    };
                }
                Segment::Incomplete(segment, part, delimiter) => {
                    // A log entry too large to be read at once is shown as is, part by part,
                    // like other text that is not a record. Every block is sent, even
                    // with nothing to show, as the merger takes them in order.
                    let mut index = TimestampIndex::new(j);
                    let buf = if !self.allow_unparsed_data() {
                        drop(segment);
                        Vec::new().into()
                    } else {
                        index.part = Some(part);
                        split_entry_part(segment, part, delimiter, prefix)
                    };
                    if txo.send((i, buf, index)).is_err() {
                        return;
                    };
                }
            }
        }
    }

    fn merge_segments(
        &self,
        badges: &FollowBadges,
        rxo: Receiver<(usize, OutputBuf, TimestampIndex)>,
        output: &mut Output,
        concurrency: usize,
    ) -> Result<()> {
        let delim = self.options.output_delimiter.as_bytes();
        let mem_limit = concurrency * usize::from(self.options.buffer_size);
        let mut window = MergeWindow::new(delim, mem_limit, self.options.sync_interval);
        let mut writer = MergeWriter::new(badges, delim);
        let mut disconnected = false;

        loop {
            let now = Instant::now();
            let mut written = false;
            // Once all inputs are closed nothing can arrive to be sorted in
            // between, so the rest of the window is flushed right away.
            while let Some(due) = window.pop_due(now, disconnected) {
                match due {
                    Due::Entry(ts, entry) => writer.write(output, ts, &entry)?,
                    Due::Resumed(i, entry) => writer.resume(output, i, &entry)?,
                    Due::SplitEnd => writer.end_split(output)?,
                }
                written = true;
            }

            // Stdout flushes by itself only at a newline, so records ending with
            // another delimiter are flushed here, before waiting for more input.
            if written {
                output.flush()?;
            }

            if disconnected {
                break;
            }

            match rxo.recv_timeout(window.wait(now)) {
                Ok((i, buf, index)) => window.insert(i, buf, index, Instant::now()),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => disconnected = true,
            }
        }

        Ok(())
    }

    fn parser(&self) -> Parser {
        Parser::new(ParserSettings::new(
            &self.options.fields.settings.predefined,
            &self.options.fields.settings.ignore,
            self.options.unix_ts_unit,
        ))
    }

    fn input_badges(&self, inputs: &[InputDescriptor]) -> Option<Vec<String>> {
        let names = inputs.iter().map(|input| input.name.as_str()).collect_vec();

        let ii = self.options.input_info;

        const II: InputInfoSet = enum_set!(InputInfo::Minimal | InputInfo::Compact | InputInfo::Full);

        if ii.intersection(II).is_empty() {
            return None;
        }

        if ii.contains(InputInfo::None) && names.len() < 2 {
            return None;
        }

        let names = if ii.contains(InputInfo::Compact) {
            self.compact_input_names(&names)
        } else {
            names.iter().map(|x| x.graphemes(true).collect_vec()).collect_vec()
        };

        Some(self.render_input_badges(names))
    }

    /// Shortens the input names for compact input badges.
    ///
    /// A common prefix longer than 7 characters is replaced with the common-part
    /// marker, keeping its last 4 characters, and names that are still too long
    /// are clipped in the middle. The names are returned as grapheme clusters.
    fn compact_input_names<'a>(&'a self, names: &[&'a str]) -> Vec<Vec<&'a str>> {
        let mut names = names.iter().map(|x| x.graphemes(true).collect_vec()).collect_vec();
        let opt = &self.punctuation;

        let pl = common_prefix_len(&names);
        for name in names.iter_mut() {
            let cl = opt.input_name_clipping.width();
            if name.len() > 24 + cl + pl {
                if pl > 7 {
                    *name = opt
                        .input_name_common_part
                        .graphemes(true)
                        .chain(name[pl - 4..pl + 8].iter().cloned())
                        .chain(opt.input_name_clipping.graphemes(true))
                        .chain(name[name.len() - 16..].iter().cloned())
                        .collect();
                } else {
                    *name = name[0..pl + 12]
                        .iter()
                        .cloned()
                        .chain(opt.input_name_clipping.graphemes(true))
                        .chain(name[name.len() - 12..].iter().cloned())
                        .collect();
                }
            } else if pl > 7 {
                *name = opt
                    .input_name_common_part
                    .graphemes(true)
                    .chain(name[pl - 4..].iter().cloned())
                    .collect();
            }
        }

        names
    }

    /// Renders themed input badges with the given input names, as grapheme clusters,
    /// padded to the same width.
    fn render_input_badges(&self, names: Vec<Vec<&str>>) -> Vec<String> {
        let ii = self.options.input_info;
        let mut badges = names;
        let num_width = format!("{}", badges.len()).len();
        let opt = &self.punctuation;

        if let Some(max_width) = badges.iter().map(|badge| grapheme_slice_width(badge)).max() {
            for badge in badges.iter_mut() {
                badge.extend(std::iter::repeat_n(" ", max_width - grapheme_slice_width(badge)));
            }
        }

        let mut result = Vec::with_capacity(badges.len());

        for (i, badge) in badges.iter_mut().enumerate() {
            let mut buf = Vec::with_capacity(badge.len() * 2);
            self.options.theme.apply(&mut buf, &None, |s| {
                s.element(Element::Input, |s| {
                    s.element(Element::InputNumber, |s| {
                        s.batch(|buf| buf.extend(opt.input_number_left_separator.as_bytes()));
                        s.element(Element::InputNumberInner, |s| {
                            s.batch(|buf| {
                                aligned(
                                    buf,
                                    Some(Adjustment {
                                        alignment: Alignment::Right,
                                        padding: Padding {
                                            pad: b' ',
                                            width: num_width + 1,
                                        },
                                    }),
                                    |mut buf| {
                                        buf.extend_from_slice(opt.input_number_prefix.as_bytes());
                                        buf.extend_from_slice(format!("{}", i).as_bytes());
                                    },
                                );
                                buf.extend(opt.input_name_left_separator.as_bytes());
                            });
                        });
                        s.batch(|buf| buf.extend(opt.input_number_right_separator.as_bytes()));
                    });

                    if ii.intersects(InputInfo::Compact | InputInfo::Full) {
                        s.element(Element::InputName, |s| {
                            s.batch(|buf| buf.extend(opt.input_name_left_separator.as_bytes()));
                            s.element(Element::InputNameInner, |s| {
                                s.batch(|buf| {
                                    buf.extend_from_slice(badge.join("").as_bytes());
                                })
                            });
                            s.batch(|buf| buf.extend(opt.input_name_right_separator.as_bytes()));
                        });
                    }
                });
            });

            result.push(String::from_utf8(buf).unwrap());
        }

        result
    }

    /// Returns whether text that is not a record is shown.
    fn allow_unparsed_data(&self) -> bool {
        self.options.filter.is_empty() && self.options.input_format.is_none()
    }

    fn new_segment_processor<'a>(&'a self, parser: &'a Parser) -> impl SegmentProcess + 'a {
        let options = SegmentProcessorOptions {
            allow_prefix: self.options.allow_prefix,
            allow_unparsed_data: self.allow_unparsed_data(),
            delimiter: self.options.delimiter.clone(),
            input_format: self.options.input_format,
            output_delimiter: self.options.output_delimiter.clone(),
        };

        SegmentProcessor::new(
            parser,
            self.formatter.clone(),
            Query::from(&self.options.filter),
            options,
        )
    }

    /// Creates a formatter based on the provided options.
    ///
    /// Returns either a RawRecordFormatter or a RecordFormatter depending on the options.
    fn new_formatter(options: &Options, punctuation: Arc<ResolvedPunctuation>) -> DynRecordWithSourceFormatter {
        if options.raw {
            Arc::new(RawRecordFormatter {
                delimiter: options.output_delimiter.clone(),
            })
        } else {
            let predefined_filter = Self::build_predefined_filter(options);
            Arc::new(
                RecordFormatterBuilder::new()
                    .with_theme(options.theme.clone())
                    .with_timestamp_formatter(DateTimeFormatter::new(options.time_format.clone(), options.time_zone))
                    .with_empty_fields_hiding(options.hide_empty_fields)
                    .with_field_filter(options.fields.filter.clone())
                    .with_predefined_field_filter(predefined_filter)
                    .with_options(options.formatting.clone())
                    .with_raw_fields(options.raw_fields)
                    .with_flatten(options.flatten)
                    .with_ascii(options.ascii)
                    .with_expansion(Expansion::from(options.formatting.expansion.clone()).with_mode(options.expand))
                    .with_always_show_time(options.fields.settings.predefined.time.show == FieldShowOption::Always)
                    .with_always_show_level(options.fields.settings.predefined.level.show == FieldShowOption::Always)
                    .with_punctuation(punctuation)
                    .with_expansion(Expansion::from(options.formatting.expansion.clone()).with_mode(options.expand))
                    .build(),
            )
        }
    }

    /// Builds an IncludeExcludeKeyFilter for nested predefined fields.
    ///
    /// This filter is used to silently exclude nested predefined fields from formatting
    /// without triggering the "..." hidden fields indicator.
    /// Uses exact matching (no normalization) to match field names precisely.
    fn build_predefined_filter(options: &Options) -> Arc<ExactIncludeExcludeKeyFilter> {
        let mut filter = ExactIncludeExcludeKeyFilter::new(MatchOptions::<NoNormalizing>::default());
        for name in options.fields.settings.predefined.nested_field_names() {
            filter.entry(name).exclude();
        }
        Arc::new(filter)
    }
}

// ---

pub trait SegmentProcess {
    fn process<O: RecordObserver>(
        &mut self,
        data: &[u8],
        buf: &mut Vec<u8>,
        origin: Origin<'_>,
        prefix: &str,
        limit: Option<usize>,
        observer: &mut O,
    );
}

// ---

#[derive(Default)]
pub struct SegmentProcessorOptions {
    pub allow_prefix: bool,
    pub allow_unparsed_data: bool,
    pub delimiter: Delimiter,
    pub input_format: Option<InputFormat>,
    pub output_delimiter: String,
}

// ---

pub struct SegmentProcessor<'a, Formatter, Filter> {
    parser: &'a Parser,
    formatter: Formatter,
    filter: Filter,
    options: SegmentProcessorOptions,
    delim: <Delimiter as Delimit>::Searcher,
}

impl<'a, Formatter: RecordWithSourceFormatter, Filter: RecordFilter> SegmentProcessor<'a, Formatter, Filter> {
    pub fn new(parser: &'a Parser, formatter: Formatter, filter: Filter, options: SegmentProcessorOptions) -> Self {
        let delim = options.delimiter.clone().into_searcher();

        Self {
            parser,
            formatter,
            filter,
            options,
            delim,
        }
    }

    #[inline(always)]
    fn show_unparsed(&self) -> bool {
        self.options.allow_unparsed_data
    }
}

impl<'a, Formatter: RecordWithSourceFormatter, Filter: RecordFilter> SegmentProcess
    for SegmentProcessor<'a, Formatter, Filter>
{
    fn process<O>(
        &mut self,
        data: &[u8],
        buf: &mut Vec<u8>,
        origin: Origin<'_>,
        prefix: &str,
        limit: Option<usize>,
        observer: &mut O,
    ) where
        O: RecordObserver,
    {
        let mut i = 0;
        let limit = limit.unwrap_or(usize::MAX);

        for chunk in self.delim.split(data) {
            let chunk_offset = chunk.as_ptr() as usize - data.as_ptr() as usize;
            if chunk.is_empty() {
                if self.show_unparsed() {
                    buf.extend(prefix.as_bytes());
                    buf.extend(self.options.output_delimiter.as_bytes());
                }
                continue;
            }

            let mut stream = RawRecord::parser()
                .allow_prefix(self.options.allow_prefix)
                .format(self.options.input_format)
                .parse(chunk);
            let mut parsed_some = false;
            let mut produced_some = false;
            let mut last_offset = 0;
            while let Some(Ok(ar)) = stream.next() {
                i += 1;
                last_offset = ar.offsets.end;
                parsed_some = true;
                let record = self.parser.parse(&ar.record);
                if record.matches(&self.filter) {
                    if produced_some {
                        buf.extend(self.options.output_delimiter.as_bytes());
                    }
                    let begin = buf.len();
                    if ar.prefix.is_empty() {
                        buf.extend(prefix.as_bytes());
                    } else {
                        let mut first = true;
                        for line in Newline.into_searcher().split(ar.prefix) {
                            if !first {
                                buf.extend(self.options.output_delimiter.as_bytes());
                            }
                            first = false;
                            buf.extend(prefix.as_bytes());
                            buf.extend(line);
                        }
                        if ar.prefix.last().map(|&x| x == b' ') == Some(false) {
                            buf.push(b' ');
                        }
                    }
                    let prefix_range = begin..buf.len();
                    let source_range = chunk_offset + ar.offsets.start..chunk_offset + ar.offsets.end;
                    self.formatter.format_record(
                        buf,
                        prefix_range,
                        record.with_source(&chunk[ar.offsets], origin.slice(source_range)),
                    );
                    let end = buf.len();
                    observer.observe_record(&record, begin..end);
                    produced_some = true;
                }
                if i >= limit {
                    break;
                }
            }
            let remainder = if parsed_some { &chunk[last_offset..] } else { chunk };
            if !remainder.is_empty() && self.show_unparsed() {
                if !parsed_some || produced_some {
                    let mut should_prefix = !parsed_some;
                    for line in Newline.into_searcher().split(remainder) {
                        if should_prefix {
                            buf.extend(prefix.as_bytes());
                        }
                        should_prefix = true;
                        buf.extend_from_slice(line);
                        buf.extend(self.options.output_delimiter.as_bytes());
                    }
                }
            } else if produced_some {
                buf.extend(self.options.output_delimiter.as_bytes());
            }
        }
    }
}

// ---

pub trait RecordObserver {
    fn observe_record<'a>(&mut self, record: &Record<'a>, location: Range<usize>);
}

// ---

pub struct RecordIgnorer {}

impl RecordObserver for RecordIgnorer {
    #[inline]
    fn observe_record<'a>(&mut self, _: &Record<'a>, _: Range<usize>) {}
}

// ---

/// Scans the streams of an input into segments and sends them numbered by block index.
///
/// A followed input delivers a stream per file that appears at its path, so the
/// counter lives here and keeps counting across streams: it is part of the
/// merge window key `(ts, input, block, offset)`, and restarting it makes entries
/// of different streams collide on one key, where all but the last are lost.
///
/// Each segment is also sent with its byte offset in the stream, which keeps
/// counting until [`BlockScanner::start`] starts a new stream.
struct BlockScanner<'a, D> {
    scanner: &'a Scanner<D>,
    max_segment_size: usize,
    block: usize,
    offset: u64,
}

impl<'a, D: Delimit> BlockScanner<'a, D> {
    fn new(scanner: &'a Scanner<D>, max_segment_size: usize) -> Self {
        Self {
            scanner,
            max_segment_size,
            block: 0,
            offset: 0,
        }
    }

    /// Starts counting byte offsets from `offset` for a new stream, such as a replaced file,
    /// or for a stream that is read from the middle, such as a file seeked to its tail.
    /// Block numbers keep counting, as they must stay unique for the input.
    fn start(&mut self, offset: u64) {
        self.offset = offset;
    }

    /// Scans `stream` up to its end, sending all its data, including an unterminated entry at its end, and each entry
    /// as soon as it is read, as the stream is still written to.
    ///
    /// It stops early, without an error, once the receiver is gone.
    fn scan(&mut self, stream: &mut dyn Read, i: usize, txi: &Sender<(usize, usize, u64, Segment)>) -> Result<()> {
        let items = self
            .scanner
            .items(stream)
            .live()
            .with_max_segment_size(self.max_segment_size);
        for item in items {
            let segment = item?;
            let len = segment.data().len();
            if txi.send((i, self.block, self.offset, segment)).is_err() {
                break;
            }
            self.block += 1;
            self.offset += len as u64;
        }
        Ok(())
    }
}

// ---

/// Returns how long a line received at `received` still has to stay in the
/// follow mode merge window, as of `now`; zero means it is due for output.
fn flush_wait(received: Instant, now: Instant, sync_interval: Duration) -> Duration {
    sync_interval.saturating_sub(now.saturating_duration_since(received))
}

/// Returns the data of `segment`, a part of a log entry too large to be read at once, to be sent to the follow mode merger.
///
/// The merger counts only the data toward its memory limit, so a part is sent in its own buffer only if it fills most of it,
/// as a part in the middle usually does. Any other part is copied into a buffer of its size: the first part, which also gets
/// `prefix`, and the last part, which loses its trailing `delimiter` and a line break before it, see [`last_part_len`].
fn split_entry_part(segment: SegmentBuf, part: PartialPlacement, delimiter: usize, prefix: &str) -> OutputBuf {
    if part == PartialPlacement::Next && segment.len() * 2 >= segment.capacity() {
        return segment.into_data().into();
    }
    let mut data = segment.data();
    let prefix = if part == PartialPlacement::First { prefix } else { "" };
    if part == PartialPlacement::Last {
        data = &data[..last_part_len(data, delimiter)];
    }
    let mut buf = Vec::with_capacity(prefix.len() + data.len());
    buf.extend_from_slice(prefix.as_bytes());
    buf.extend_from_slice(data);
    buf.into()
}

/// Ends the last part of a log entry too large to be read at once with the output delimiter instead of the input one, if any,
/// and a line break before it, see [`last_part_len`].
fn end_last_part(mut segment: SegmentBuf, delimiter: usize, output_delimiter: &[u8]) -> SegmentBuf {
    segment.truncate(last_part_len(segment.data(), delimiter));
    segment.extend_from_slice(output_delimiter);
    segment
}

/// Returns the length of the last part of a log entry too large to be read at once without the delimiter it ends with,
/// and without a line break before it, which is dropped too when text that is not a record is split into lines.
///
/// Only the last part itself is looked at, so a line break that ends the part before it is kept, see #1709.
#[inline]
fn last_part_len(data: &[u8], delimiter: usize) -> usize {
    // Only the end is checked, as searching for the line break would scan the whole part when it has none.
    match &data[..data.len() - delimiter] {
        [rest @ .., b'\n'] => rest.len() - usize::from(rest.last() == Some(&b'\r')),
        rest => rest.len(),
    }
}

// ---

struct TimestampIndexBuilder {
    result: TimestampIndex,
}

impl RecordObserver for TimestampIndexBuilder {
    #[inline]
    fn observe_record<'a>(&mut self, record: &Record<'a>, location: Range<usize>) {
        if let Some(ts) = record.ts.as_ref().and_then(|ts| ts.unix_utc()).map(|ts| ts.into()) {
            self.result.lines.push(TimestampIndexLine { location, ts });
        }
    }
}

// ---

impl<T: FnMut(&Record, Range<usize>)> RecordObserver for T {
    #[inline]
    fn observe_record<'a>(&mut self, record: &Record<'a>, location: Range<usize>) {
        self(record, location)
    }
}

// ---

struct TimestampIndex {
    block: usize,
    lines: Vec<TimestampIndexLine>,
    /// The part of a log entry too large to be read at once that the block holds, if it holds one.
    part: Option<PartialPlacement>,
}

impl TimestampIndex {
    fn new(block: usize) -> Self {
        Self {
            block,
            lines: Vec::new(),
            part: None,
        }
    }
}

// ---

struct TimestampIndexLine {
    location: Range<usize>,
    ts: Timestamp,
}

// ---

struct OutputBlock {
    ts_min: crate::index::Timestamp,
    buf: Arc<Vec<u8>>,
    items: Vec<(Timestamp, Range<usize>)>,
}

impl OutputBlock {
    pub fn into_lines(self) -> impl Iterator<Item = (Timestamp, BlockEntry)> {
        let buf = self.buf;
        self.items
            .into_iter()
            .map(move |(ts, range)| (ts, BlockEntry::new(buf.clone(), range.clone())))
    }
}

// ---

struct StripedReceiver<T> {
    input: Vec<Receiver<T>>,
    sn: usize,
}

impl<T> StripedReceiver<T> {
    fn new(input: Vec<Receiver<T>>) -> Self {
        Self { input, sn: 0 }
    }
}

impl<T> Iterator for StripedReceiver<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.input[self.sn].recv().ok()?;
        self.sn = (self.sn + 1) % self.input.len();
        Some(item)
    }
}

// ---

struct StripedSender<T> {
    output: Vec<Sender<T>>,
    sn: usize,
}

impl<T> StripedSender<T> {
    fn new(output: Vec<Sender<T>>) -> Self {
        Self { output, sn: 0 }
    }

    fn send(&mut self, value: T) -> Option<()> {
        self.output[self.sn].send(value).ok()?;
        self.sn = (self.sn + 1) % self.output.len();
        Some(())
    }
}

// ---

struct FollowBadges {
    si: SyncIndicator,
    input: Option<Vec<String>>,
}

impl FollowBadges {
    /// Returns what an entry of input `i` starts with: its input badge, or the placeholder for the sync indicator.
    fn prefix(&self, i: usize) -> &str {
        self.input
            .as_ref()
            .map(|b| b[i].as_str())
            .unwrap_or(&self.si.placeholder)
    }
}

struct SyncIndicator {
    width: usize,
    synced: String,
    failed: String,
    placeholder: String,
}

impl From<&SyncIndicatorPack> for SyncIndicator {
    fn from(si: &SyncIndicatorPack) -> Self {
        let width = max(si.synced.width, si.failed.width);
        let synced = si.synced.value.to_owned() + &" ".repeat(width - si.synced.width);
        let failed = si.failed.value.to_owned() + &" ".repeat(width - si.failed.width);
        let placeholder = " ".repeat(width);

        Self {
            width,
            synced,
            failed,
            placeholder,
        }
    }
}

// ---

fn common_prefix_len<'a, V, I>(items: &'a Vec<I>) -> usize
where
    V: 'a + Eq + PartialEq,
    I: AsRef<[V]> + 'a,
{
    let mut i: usize = 0;
    loop {
        let mut b = None;
        for item in items {
            let item = item.as_ref();
            if item.len() <= i {
                return i;
            }
            if let Some(b) = b {
                if &item[i] != b {
                    return i;
                }
            } else {
                b = Some(&item[i])
            }
        }
        i += 1;
    }
}

#[allow(dead_code)]
fn grapheme_slice_width(graphemes: &[impl AsRef<str>]) -> usize {
    graphemes.iter().map(|g| g.as_ref().width()).sum()
}

// ---

#[cfg(test)]
mod tests;
