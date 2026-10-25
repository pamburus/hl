// std imports
use std::{
    cmp::{Reverse, max},
    convert::{TryFrom, TryInto},
    fs,
    io::{BufWriter, Cursor, Read, Write},
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
    diagnostics::{self, DiagnosticSink, Diagnostics, LogSink, UnparsedKind},
    error::*,
    fileid::MetadataExt,
    filtering::{MatchOptions, NoNormalizing},
    fmtx::{Adjustment, Alignment, Padding, aligned},
    formatting::{
        DropSink, DynRecordWithSourceFormatter, Expansion, RawRecordFormatter, RecordFormatterBuilder,
        RecordWithSourceFormatter, UnparsedData,
        structured::{JsonEncoder, LogfmtEncoder, StructuredRecordFormatterBuilder},
    },
    fsmon::{self, EventKind},
    help,
    index::{Indexer, IndexerSettings, Timestamp},
    input::{BlockEntry, Input, InputDescriptor, InputHolder, InputReference},
    model::{
        Filter, Origin, Parser, ParserSettings, RawRecord, Record, RecordFilter, RecordWithSource,
        RecordWithSourceConstructor,
    },
    query::Query,
    scanning::{
        BufFactory, Delimit, Delimiter, PartialPlacement, Scanner, Search, SearchExt, Segment, SegmentBuf,
        SegmentBufFactory,
    },
    settings::{
        AsciiMode, ExpansionMode, FieldShowOption, Fields, Formatting, InputInfo, ResolvedPunctuation,
        StructuredFormatting,
    },
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
    pub output_format: OutputFormat,
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

/// Format of the output records.
#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum OutputFormat {
    /// Human-readable output.
    #[default]
    Pretty,
    /// One JSON object per record.
    Json,
    /// One line of logfmt pairs per record.
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
    diagnostic_sink: Arc<dyn DiagnosticSink>,
}

impl App {
    pub fn new(options: Options) -> Self {
        Self::with_diagnostic_sink(options, Arc::new(LogSink))
    }

    /// Creates an app that writes its diagnostic entries to `sink`.
    pub fn with_diagnostic_sink(mut options: Options, sink: Arc<dyn DiagnosticSink>) -> Self {
        if options.raw && options.input_info.intersects(InputInfo::None | InputInfo::Auto) {
            options.input_info = InputInfo::None.into()
        }
        if options.raw {
            // Raw output takes precedence over a structured output format.
            options.output_format = OutputFormat::Pretty;
        }
        options.input_info = InputInfo::resolve(options.input_info);

        let punctuation = Arc::new(options.formatting.punctuation.resolve(options.ascii));

        let formatter = Self::new_formatter(&options, punctuation.clone(), sink.clone());

        Self {
            options,
            punctuation,
            formatter,
            diagnostic_sink: sink,
        }
    }

    pub fn run(&self, inputs: Vec<InputHolder>, output: &mut Output) -> Result<()> {
        let descriptors = inputs
            .iter()
            .map(|x| &x.reference)
            .enumerate()
            .map(Into::into)
            .collect_vec();
        let diagnostics = Arc::new(Diagnostics::new(self.diagnostic_sink.clone(), &descriptors));
        diagnostics::register(diagnostics.clone());

        let result = if self.options.follow {
            self.follow(inputs, &descriptors, &diagnostics, output)
        } else if self.options.sort {
            self.sort(inputs, &descriptors, &diagnostics, output)
        } else {
            self.cat(inputs, &descriptors, &diagnostics, output)
        };

        diagnostics.write_summary();
        result
    }

    fn cat(
        &self,
        inputs: Vec<InputHolder>,
        descriptors: &[InputDescriptor],
        diagnostics: &Diagnostics,
        output: &mut Output,
    ) -> Result<()> {
        let input_badges = self.input_badges(descriptors);

        let inputs = inputs
            .into_iter()
            .map(|x| x.open())
            .collect::<std::io::Result<Vec<_>>>()?;

        let n = self.options.concurrency;
        let sfi = Arc::new(SegmentBufFactory::new(self.options.buffer_size.into()));
        let bfo = BufFactory::new(self.options.buffer_size.into());
        let parser = self.parser();
        thread::scope(|scope| -> Result<()> {
            // prepare receive/transmit channels for input data
            let (txi, rxi): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded(1)).unzip();
            // prepare receive/transmit channels for output data
            let (txo, rxo): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded::<(usize, SegmentBuf)>(1)).unzip();
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
                scope.spawn(closure!(ref bfo, ref parser, ref sfi, ref input_badges, |_| {
                    let mut processor = self.new_segment_processor(parser, diagnostics);
                    let delim = self.options.delimiter.clone().into_searcher();
                    for (i, offset, segment) in rxi.iter() {
                        let prefix = input_badges.as_ref().map(|b|b[i].as_str()).unwrap_or("");
                        let origin = Origin::new(&descriptors[i], offset, segment.data().len());
                        let buf = match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                processor.process(segment.data(), &mut buf, origin, prefix, None, &mut RecordIgnorer{});
                                sfi.recycle(segment);
                                buf.into()
                            }
                            // A log entry too large to be read at once is shown as is, part by part,
                            // like other text that is not a record, if the output shows such text.
                            // Every segment is sent, even with nothing to show, as the writer takes them in order.
                            Segment::Incomplete(segment, part) => {
                                let mut buf = bfo.new_buf();
                                let data = segment.data();
                                let entry = if part == PartialPlacement::Last { strip_delimiter(&delim, data) } else { data };
                                // The delimiter is written as read, if the entry is written.
                                if processor.process_fragment(entry, &mut buf, origin, "") {
                                    buf.extend(&data[entry.len()..]);
                                }
                                sfi.recycle(segment);
                                buf.into()
                            }
                        };
                        if txo.send((i, buf)).is_err() {
                            break;
                        }
                    }
                }));
            }
            // spawn writer thread
            let writer = scope.spawn(closure!(ref bfo, |_| -> Result<()> {
                for (_, buf) in StripedReceiver::new(rxo) {
                    output.write_all(buf.data())?;
                    bfo.recycle(buf.into_inner());
                }
                Ok(())
            }));
            // collect errors from reader and writer threads
            reader.join().unwrap()?;
            writer.join().unwrap()?;
            Ok(())
        })
        .unwrap()?;

        Ok(())
    }

    fn sort(
        &self,
        inputs: Vec<InputHolder>,
        descriptors: &[InputDescriptor],
        diagnostics: &Diagnostics,
        output: &mut Output,
    ) -> Result<()> {
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
        let input_badges = self.input_badges(descriptors);

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
                            if let Some(until) = self.options.filter.basic.until {
                                if ts_min > until.into() {
                                    return None;
                                }
                            }
                            if let Some(since) = self.options.filter.basic.since {
                                if ts_max < since.into() {
                                    return None;
                                }
                            }
                            if let Some(level) = self.options.filter.basic.level {
                                if !src.match_level(level) {
                                    return None;
                                }
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
                workers.push(scope.spawn(closure!(ref parser, ref input_badges, |_| -> Result<()> {
                    let mut processor = self.new_segment_processor(parser, diagnostics);
                    for (block, ts_min, i, j) in rxp.iter() {
                        let block_offset = block.offset();
                        let mut buf = Vec::with_capacity(2 * usize::try_from(block.size())?);
                        let mut items = Vec::with_capacity(2 * usize::try_from(block.entries_valid())?);
                        for line in block.into_entries()? {
                            if line.is_empty() {
                                continue;
                            }
                            let prefix = input_badges.as_ref().map(|b| b[i].as_str()).unwrap_or("");
                            let origin = Origin::new(&descriptors[i], block_offset + line.offset() as u64, line.len());
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
                })));
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
        let si = if !self.formatter.decorations() {
            SyncIndicator::default()
        } else {
            SyncIndicator::from(&self.options.theme.indicators.sync)
        };

        let mut badges = self.input_badges(inputs);
        if let Some(badges) = &mut badges {
            for badge in badges.iter_mut() {
                *badge = format!("{}{}", si.placeholder, badge);
            }
        }

        FollowBadges { si, input: badges }
    }

    fn follow(
        &self,
        inputs: Vec<InputHolder>,
        descriptors: &[InputDescriptor],
        diagnostics: &Diagnostics,
        output: &mut Output,
    ) -> Result<()> {
        let badges = self.prepare_follow_badges(descriptors);

        let m = inputs.len();
        let n = self.options.concurrency;
        let parser = self.parser();
        let sfi = Arc::new(SegmentBufFactory::new(self.options.buffer_size.into()));
        let bfo = BufFactory::new(self.options.buffer_size.into());

        thread::scope(|scope| -> Result<()> {
            // prepare receive/transmit channels for input data
            let (txi, rxi) = channel::bounded(1);
            // prepare receive/transmit channels for output data
            let (txo, rxo) = channel::bounded(1);
            // spawn reader threads
            let mut readers = Vec::with_capacity(m);
            for (i, holder) in inputs.into_iter().enumerate() {
                let delimiter = &self.options.delimiter;
                let reader = scope.spawn(closure!(clone sfi, clone txi, |_| -> Result<()> {
                    let scanner = Scanner::new(sfi.clone(), delimiter.clone());
                    let input_ref = holder.reference.clone();
                    // The file read is the one opened before, which may already be
                    // rotated, so its replacement is detected by its own metadata.
                    let mut meta = holder.metadata()?;
                    let mut input = holder.open()?.tail(self.options.tail, delimiter.clone())?;
                    let is_file = |meta: &Option<fs::Metadata>| meta.as_ref().map(|m|m.is_file()).unwrap_or(false);
                    let mut blocks = BlockScanner::new(&scanner, delimiter.clone(), self.options.max_message_size.into());
                    blocks.start(input.position()?);
                    let mut input = Some(input);
                    // Only a regular file is scanned again when it grows, so only there
                    // an unterminated tail is held back to be completed by the next scan.
                    // Without an input, a tail held from the previous one is flushed.
                    let mut process = |input: &mut Option<Input>, is_file: bool| {
                        if let Some(input) = input {
                            blocks.scan(&mut input.stream.as_sequential(), i, &txi, is_file)?;
                            Ok(!is_file)
                        } else {
                            blocks.scan(&mut std::io::empty(), i, &txi, false)?;
                            // The next input is a newly opened file read from its start.
                            blocks.start(0);
                            Ok(false)
                        }
                    };
                    if let InputReference::File(path) = &input_ref {
                        if process(&mut input, is_file(&meta))? {
                            return Ok(())
                        }
                        fsmon::run(vec![path.canonical.clone()], |event| {
                            match event.kind {
                                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any | EventKind::Other => {
                                    if let (Some(old_meta), Ok(new_meta)) = (&meta, fs::metadata(&path.canonical)) {
                                        let replaced = old_meta.len() > new_meta.len() || old_meta.file_id() != new_meta.file_id();
                                        if replaced && input.is_some() {
                                            // The rest of the old file will not come, flush its tail.
                                            input = None;
                                            process(&mut input, false)?;
                                        }
                                        meta = Some(new_meta);
                                    }
                                    if input.is_none() {
                                        input = input_ref.open().ok();
                                    }
                                    if process(&mut input, is_file(&meta))? {
                                        return Ok(())
                                    }
                                    Ok(())
                                }
                                EventKind::Remove(_) => {
                                    input = None;
                                    process(&mut input, false).map(|_| ())
                                },
                                EventKind::Access(_) => Ok(()),
                            }
                        })
                    } else {
                        process(&mut input, is_file(&meta)).map(|_|())
                    }
                }));
                readers.push(reader);
            }
            drop(txi);

            // spawn processing threads
            let mut workers = Vec::with_capacity(n);
            for _ in 0..n {
                let worker = scope.spawn(
                    closure!(ref bfo, ref parser, ref sfi, ref badges, clone rxi, clone txo, |_| {
                        self.process_segments(parser, bfo, sfi, badges, descriptors, diagnostics, rxi, txo);
                    }),
                );
                workers.push(worker);
            }
            drop(txo);

            // spawn merger thread
            let merger = scope.spawn(|_| -> Result<()> { self.merge_segments(&badges, rxo, output, n) });

            for reader in readers {
                reader.join().unwrap()?;
            }

            for worker in workers {
                worker.join().unwrap();
            }

            merger.join().unwrap()?;

            Ok(())
        })
        .unwrap()?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_segments(
        &self,
        parser: &Parser,
        bfo: &BufFactory,
        sfi: &SegmentBufFactory,
        badges: &FollowBadges,
        inputs: &[InputDescriptor],
        diagnostics: &Diagnostics,
        rxi: Receiver<(usize, usize, u64, Segment)>,
        txo: Sender<(usize, Vec<u8>, TimestampIndex)>,
    ) {
        let mut processor = self.new_segment_processor(parser, diagnostics);
        let delim = self.options.delimiter.clone().into_searcher();
        for (i, j, offset, segment) in rxi.iter() {
            let prefix = badges
                .input
                .as_ref()
                .map(|b| b[i].as_str())
                .unwrap_or(&badges.si.placeholder);
            let origin = Origin::new(&inputs[i], offset, segment.data().len());
            match segment {
                Segment::Complete(segment) => {
                    let mut buf = bfo.new_buf();
                    let mut index_builder = TimestampIndexBuilder {
                        result: TimestampIndex::new(j),
                    };
                    processor.process(segment.data(), &mut buf, origin, prefix, None, &mut index_builder);
                    sfi.recycle(segment);
                    if txo.send((i, buf, index_builder.result)).is_err() {
                        return;
                    };
                }
                Segment::Incomplete(segment, part) => {
                    // A log entry too large to be read at once is shown as is, part by part,
                    // like other text that is not a record, if the output shows such text.
                    // Every block is sent, even with nothing to show, as the merger takes them in order.
                    let mut buf = bfo.new_buf();
                    let mut index = TimestampIndex::new(j);
                    let mut data = segment.data();
                    if part == PartialPlacement::Last {
                        data = strip_delimiter(&delim, data);
                    }
                    let prefix = if part == PartialPlacement::First { prefix } else { "" };
                    if processor.process_fragment(data, &mut buf, origin, prefix) {
                        index.part = Some(part);
                    }
                    sfi.recycle(segment);
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
        rxo: Receiver<(usize, Vec<u8>, TimestampIndex)>,
        output: &mut Output,
        concurrency: usize,
    ) -> Result<()> {
        let delim = self.options.output_delimiter.as_bytes();
        let mem_limit = concurrency * usize::from(self.options.buffer_size);
        let mut window = MergeWindow::new(delim, mem_limit, self.options.sync_interval);
        let mut writer = MergeWriter::new(&badges.si, delim);
        let mut disconnected = false;

        loop {
            let now = Instant::now();
            let mut written = false;
            // Once all inputs are closed nothing can arrive to be sorted in
            // between, so the rest of the window is flushed right away.
            while let Some(due) = window.pop_due(now, disconnected) {
                match due {
                    Due::Entry(ts, entry) => writer.write(output, ts, &entry)?,
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
        if !self.formatter.decorations() {
            return None;
        }

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

    /// Returns whether text that is not a record is shown in output that shows such text.
    fn allow_unparsed_data(&self) -> bool {
        self.options.filter.is_empty() && self.options.input_format.is_none()
    }

    /// Creates a segment processor that passes the input data it drops to `drops`.
    fn new_segment_processor<'a, D: DropSink + 'a>(&'a self, parser: &'a Parser, drops: D) -> impl SegmentProcess + 'a {
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
        .with_drop_sink(drops)
    }

    /// Creates a formatter based on the provided options.
    ///
    /// Returns either a RawRecordFormatter or a RecordFormatter depending on the options.
    fn new_formatter(
        options: &Options,
        punctuation: Arc<ResolvedPunctuation>,
        diagnostic_sink: Arc<dyn DiagnosticSink>,
    ) -> DynRecordWithSourceFormatter {
        if options.raw {
            Arc::new(RawRecordFormatter {
                delimiter: options.output_delimiter.clone(),
            })
        } else if let Some(formatter) = Self::new_structured_formatter(options, diagnostic_sink) {
            formatter
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

    /// Creates a formatter for a structured output format, or returns `None` for the pretty format.
    fn new_structured_formatter(
        options: &Options,
        diagnostic_sink: Arc<dyn DiagnosticSink>,
    ) -> Option<DynRecordWithSourceFormatter> {
        let builder = |format: &StructuredFormatting| {
            StructuredRecordFormatterBuilder::new()
                .with_flatten(options.flatten)
                .with_unique_keys(format.unique_keys.unwrap_or(true))
                .with_empty_fields_hiding(options.hide_empty_fields)
                .with_field_filter(options.fields.filter.clone())
                .with_predefined_fields(&options.fields.settings.predefined)
                .with_diagnostic_sink(diagnostic_sink.clone())
        };
        match options.output_format {
            OutputFormat::Pretty => None,
            OutputFormat::Json => Some(Arc::new(builder(&options.formatting.json).build::<JsonEncoder>())),
            OutputFormat::Logfmt => Some(Arc::new(builder(&options.formatting.logfmt).build::<LogfmtEncoder>())),
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

    /// Processes `data`, a part of an entry too long to be read as a record, which is written or dropped,
    /// and returns whether it is written.
    fn process_fragment(&mut self, data: &[u8], buf: &mut Vec<u8>, origin: Origin<'_>, prefix: &str) -> bool;
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

pub struct SegmentProcessor<'a, Formatter, Filter, Drops = ()> {
    parser: &'a Parser,
    filter: Filter,
    options: SegmentProcessorOptions,
    delim: <Delimiter as Delimit>::Searcher,
    emitter: Emitter<Formatter, Drops>,
}

impl<'a, Formatter: RecordWithSourceFormatter, Filter: RecordFilter> SegmentProcessor<'a, Formatter, Filter> {
    pub fn new(parser: &'a Parser, formatter: Formatter, filter: Filter, options: SegmentProcessorOptions) -> Self {
        let delim = options.delimiter.clone().into_searcher();
        let emitter = Emitter {
            formatter,
            drops: (),
            delimiter: options.output_delimiter.clone(),
        };

        Self {
            parser,
            filter,
            options,
            delim,
            emitter,
        }
    }
}

impl<'a, Formatter: RecordWithSourceFormatter, Filter: RecordFilter, Drops: DropSink>
    SegmentProcessor<'a, Formatter, Filter, Drops>
{
    /// Sets where the input data that the processor drops goes, which is nowhere by default.
    pub fn with_drop_sink<D: DropSink>(self, drops: D) -> SegmentProcessor<'a, Formatter, Filter, D> {
        SegmentProcessor {
            parser: self.parser,
            filter: self.filter,
            options: self.options,
            delim: self.delim,
            emitter: Emitter {
                formatter: self.emitter.formatter,
                drops,
                delimiter: self.emitter.delimiter,
            },
        }
    }

    #[inline(always)]
    fn show_unparsed(&self) -> bool {
        self.options.allow_unparsed_data
    }
}

/// Returns `data` without the delimiter it ends with, if any.
#[inline]
fn strip_delimiter<'d>(delim: &impl Search, data: &'d [u8]) -> &'d [u8] {
    match delim.search_r(data, true) {
        Some(found) if found.end == data.len() => &data[..found.start],
        _ => data,
    }
}

/// Returns `unparsed`, a part of `data` read from `origin`, as input data that is not a record.
#[inline]
fn unparsed_data<'a>(
    data: &[u8],
    origin: &Origin<'a>,
    unparsed: &'a [u8],
    kind: UnparsedKind,
    hidden: bool,
) -> UnparsedData<'a> {
    let offset = unparsed.as_ptr() as usize - data.as_ptr() as usize;
    UnparsedData {
        data: unparsed,
        origin: origin.slice(offset..offset + unparsed.len()),
        kind,
        hidden,
    }
}

impl<'a, Formatter: RecordWithSourceFormatter, Filter: RecordFilter, Drops: DropSink> SegmentProcess
    for SegmentProcessor<'a, Formatter, Filter, Drops>
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
                let hidden = !self.show_unparsed();
                let unparsed = unparsed_data(data, &origin, chunk, UnparsedKind::Entry, hidden);
                self.emitter.unparsed(buf, prefix, unparsed);
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
                        self.emitter.delimiter(buf);
                    }
                    let begin = buf.len();
                    if ar.prefix.is_empty() {
                        buf.extend(prefix.as_bytes());
                    } else {
                        let unparsed = unparsed_data(data, &origin, ar.prefix, UnparsedKind::Prefix, false);
                        self.emitter.unparsed(buf, prefix, unparsed);
                    }
                    let prefix_range = begin..buf.len();
                    let source_range = chunk_offset + ar.offsets.start..chunk_offset + ar.offsets.end;
                    self.emitter.record(
                        buf,
                        prefix_range,
                        record.with_source(&chunk[ar.offsets], origin.slice(source_range)),
                    );
                    let end = buf.len();
                    observer.observe_record(&record, begin..end);
                    produced_some = true;
                } else if !ar.prefix.is_empty() {
                    // Data before a record that is filtered out is hidden along with it.
                    let unparsed = unparsed_data(data, &origin, ar.prefix, UnparsedKind::Prefix, true);
                    self.emitter.unparsed(buf, prefix, unparsed);
                }
                if i >= limit {
                    break;
                }
            }
            let remainder = if parsed_some { &chunk[last_offset..] } else { chunk };
            if !remainder.is_empty() {
                let (kind, hidden) = if parsed_some {
                    // Data after records that are filtered out is hidden along with them.
                    (UnparsedKind::Remainder, !self.show_unparsed() || !produced_some)
                } else {
                    (UnparsedKind::Entry, !self.show_unparsed())
                };
                self.emitter
                    .unparsed(buf, prefix, unparsed_data(data, &origin, remainder, kind, hidden));
            }
            if produced_some {
                self.emitter.delimiter(buf);
            }
        }
    }

    #[inline]
    fn process_fragment(&mut self, data: &[u8], buf: &mut Vec<u8>, origin: Origin<'_>, prefix: &str) -> bool {
        let unparsed = UnparsedData {
            data,
            origin,
            kind: UnparsedKind::Fragment,
            hidden: !self.show_unparsed(),
        };
        self.emitter.unparsed(buf, prefix, unparsed)
    }
}

// ---

/// Emits what the segment processor finds in the input: records are formatted, and data
/// that is not a record is written or dropped, as the formatter decides.
struct Emitter<Formatter, Drops> {
    formatter: Formatter,
    drops: Drops,
    /// The output delimiter.
    delimiter: String,
}

impl<Formatter: RecordWithSourceFormatter, Drops: DropSink> Emitter<Formatter, Drops> {
    #[inline]
    fn record(&self, buf: &mut Vec<u8>, prefix: Range<usize>, rec: RecordWithSource) {
        self.formatter.format_record(buf, prefix, rec);
    }

    /// Writes or drops `unparsed` and returns whether it is written.
    #[inline]
    fn unparsed(&mut self, buf: &mut Vec<u8>, prefix: &str, unparsed: UnparsedData) -> bool {
        let delimiter = self.delimiter.as_bytes();
        self.formatter
            .format_unparsed(buf, prefix.as_bytes(), delimiter, unparsed, &mut self.drops)
    }

    /// Writes the output delimiter.
    #[inline]
    fn delimiter(&self, buf: &mut Vec<u8>) {
        buf.extend(self.delimiter.as_bytes());
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

/// Scans an input stream into segments and sends them numbered by block index.
///
/// In follow mode a fresh scan starts on every filesystem event, resuming the
/// same open stream where the previous one stopped at end of file. The counter
/// therefore lives here and keeps counting across scans: it is part of the
/// merge window key `(ts, input, block, offset)`, and restarting it makes lines
/// of different scans collide on one key, where all but the last are lost.
///
/// For the same reason it also keeps the unterminated tail of the stream: end of
/// file in the middle of a line usually means the writer has not finished it
/// yet, so the tail is held back and prepended to the data of the next scan.
///
/// Each segment is also sent with its byte offset in the stream, which keeps
/// counting across scans until [`BlockScanner::start`] starts a new stream.
struct BlockScanner<'a, D: Delimit> {
    scanner: &'a Scanner<D>,
    searcher: D::Searcher,
    max_segment_size: usize,
    block: usize,
    offset: u64,
    tail: Vec<u8>,
}

impl<'a, D: Delimit> BlockScanner<'a, D> {
    fn new(scanner: &'a Scanner<D>, delimiter: D, max_segment_size: usize) -> Self {
        Self {
            scanner,
            searcher: delimiter.into_searcher(),
            max_segment_size,
            block: 0,
            offset: 0,
            tail: Vec::new(),
        }
    }

    /// Starts counting byte offsets from `offset` for a new stream, such as a replaced file,
    /// or for a stream that is read from the middle, such as a file seeked to its tail.
    /// Block numbers keep counting, as they must stay unique for the input.
    fn start(&mut self, offset: u64) {
        self.offset = offset;
    }

    /// Scans the held tail followed by `stream` up to its end.
    ///
    /// With `hold_tail`, the data after the last delimiter is held back for the
    /// next scan instead of being sent as a complete segment. Without it, all
    /// data is sent, which also flushes a tail held by a previous scan.
    fn scan(
        &mut self,
        stream: &mut dyn Read,
        i: usize,
        txi: &Sender<(usize, usize, u64, Segment)>,
        hold_tail: bool,
    ) -> Result<()> {
        let tail = std::mem::take(&mut self.tail);
        let mut holding = None;
        let mut chained = None;
        let reader: &mut dyn Read = if hold_tail {
            holding.insert(TailHoldingReader::new(
                tail,
                stream,
                &self.searcher,
                self.max_segment_size,
            ))
        } else {
            chained.insert(Cursor::new(tail).chain(stream))
        };
        Self::send(
            self.scanner,
            self.max_segment_size,
            (&mut self.block, &mut self.offset),
            reader,
            i,
            txi,
        )?;
        if let Some(reader) = holding {
            self.tail = reader.into_tail();
        }
        Ok(())
    }

    fn send(
        scanner: &Scanner<D>,
        max_segment_size: usize,
        (block, offset): (&mut usize, &mut u64),
        stream: &mut dyn Read,
        i: usize,
        txi: &Sender<(usize, usize, u64, Segment)>,
    ) -> Result<()> {
        for item in scanner.items(stream).with_max_segment_size(max_segment_size) {
            let item = item?;
            let len = item.data().len();
            if txi.send((i, *block, *offset, item)).is_err() {
                break;
            }
            *block += 1;
            *offset += len as u64;
        }
        Ok(())
    }
}

// ---

/// Reads a previously held tail followed by an inner stream, and holds back the
/// data after the last delimiter once the inner stream reaches its end.
///
/// The held data stops growing at `limit` bytes: a line that long is passed on
/// even if unterminated, as the scanner would split it into parts anyway.
struct TailHoldingReader<'a, S> {
    inner: &'a mut dyn Read,
    searcher: &'a S,
    limit: usize,
    buf: Vec<u8>,
    // `buf[pos..ready]` can be passed on, `buf[ready..]` is held back.
    pos: usize,
    ready: usize,
}

impl<'a, S: Search> TailHoldingReader<'a, S> {
    const CHUNK_SIZE: usize = 64 * 1024;

    fn new(tail: Vec<u8>, inner: &'a mut dyn Read, searcher: &'a S, limit: usize) -> Self {
        Self {
            inner,
            searcher,
            limit,
            buf: tail,
            pos: 0,
            ready: 0,
        }
    }

    fn into_tail(mut self) -> Vec<u8> {
        self.buf.drain(..self.pos);
        self.buf
    }
}

impl<S: Search> Read for TailHoldingReader<'_, S> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.pos == self.ready {
            self.buf.drain(..self.pos);
            self.pos = 0;
            self.ready = 0;

            let len = self.buf.len();
            self.buf.resize(len + Self::CHUNK_SIZE, 0);
            let n = self.inner.read(&mut self.buf[len..]);
            self.buf.truncate(len + n.as_ref().copied().unwrap_or(0));
            if n? == 0 {
                return Ok(0);
            }

            self.ready = if self.buf.len() >= self.limit {
                self.buf.len()
            } else {
                self.searcher.search_r(&self.buf, true).map(|r| r.end).unwrap_or(0)
            };
        }

        let n = out.len().min(self.ready - self.pos);
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

// ---

/// Returns how long a line received at `received` still has to stay in the
/// follow mode merge window, as of `now`; zero means it is due for output.
fn flush_wait(received: Instant, now: Instant, sync_interval: Duration) -> Duration {
    sync_interval.saturating_sub(now.saturating_duration_since(received))
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

#[derive(Default)]
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
