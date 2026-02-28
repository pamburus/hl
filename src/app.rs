// std imports
use std::{
    cmp::{Reverse, max},
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    fs,
    io::{BufWriter, Write},
    num::NonZeroUsize,
    ops::Range,
    path::PathBuf,
    rc::Rc,
    str,
    sync::Arc,
    time::{Duration, Instant},
};

// unix-only std imports
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

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
    error::*,
    filtering::{MatchOptions, NoNormalizing},
    fmtx::{Adjustment, Alignment, Padding, aligned},
    formatting::{
        DynRecordWithSourceFormatter, Expansion, RawRecordFormatter, RecordFormatterBuilder, RecordWithSourceFormatter,
    },
    fsmon::{self, EventKind},
    help,
    index::{Indexer, IndexerSettings, Timestamp},
    input::{BlockEntry, Input, InputHolder, InputReference},
    model::{Filter, Parser, ParserSettings, RawRecord, Record, RecordFilter, RecordWithSourceConstructor},
    query::Query,
    scanning::{BufFactory, Delimit, Delimiter, Newline, Scanner, SearchExt, Segment, SegmentBuf, SegmentBufFactory},
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
            self.follow(inputs.into_iter().map(|x| x.reference).collect(), output)
        } else if self.options.sort {
            self.sort(inputs, output)
        } else {
            self.cat(inputs, output)
        }
    }

    fn cat(&self, inputs: Vec<InputHolder>, output: &mut Output) -> Result<()> {
        let input_badges = self.input_badges(inputs.iter().map(|x| &x.reference));

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
                    for item in scanner.items(&mut input.stream.as_sequential()).with_max_segment_size(self.options.max_message_size.into()) {
                        if tx.send((i, item?)).is_none() {
                            break;
                        }
                    }
                }
                Ok(())
            }));
            // spawn processing threads
            for (rxi, txo) in izip!(rxi, txo) {
                scope.spawn(closure!(ref bfo, ref parser, ref sfi, ref input_badges, |_| {
                    let mut processor = self.new_segment_processor(parser);
                    for (i, segment) in rxi.iter() {
                        let prefix = input_badges.as_ref().map(|b|b[i].as_str()).unwrap_or("");
                        match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                processor.process(segment.data(), &mut buf, prefix, None, &mut RecordIgnorer{});
                                sfi.recycle(segment);
                                if txo.send((i, buf.into())).is_err() {
                                    break;
                                };
                            }
                            Segment::Incomplete(segment, _) => {
                                if txo.send((i, segment)).is_err() {
                                    break;
                                }
                            }
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
        let input_badges = self.input_badges(inputs.iter().map(|x| &x.reference));

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
                    let mut processor = self.new_segment_processor(parser);
                    for (block, ts_min, i, j) in rxp.iter() {
                        let mut buf = Vec::with_capacity(2 * usize::try_from(block.size())?);
                        let mut items = Vec::with_capacity(2 * usize::try_from(block.entries_valid())?);
                        for line in block.into_entries()? {
                            if line.is_empty() {
                                continue;
                            }
                            let prefix = input_badges.as_ref().map(|b| b[i].as_str()).unwrap_or("");
                            processor.process(
                                line.bytes(),
                                &mut buf,
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

    fn prepare_follow_badges<'a, I: IntoIterator<Item = &'a InputReference>>(&self, inputs: I) -> FollowBadges {
        let si = SyncIndicator::from(&self.options.theme.indicators.sync);

        let mut badges = self.input_badges(inputs);
        if let Some(badges) = &mut badges {
            for badge in badges.iter_mut() {
                *badge = format!("{}{}", si.placeholder, badge);
            }
        }

        FollowBadges { si, input: badges }
    }

    fn follow(&self, inputs: Vec<InputReference>, output: &mut Output) -> Result<()> {
        let badges = self.prepare_follow_badges(inputs.iter());

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
            for (i, input_ref) in inputs.into_iter().enumerate() {
                let delimiter = &self.options.delimiter;
                let reader = scope.spawn(closure!(clone sfi, clone txi, |_| -> Result<()> {
                    let scanner = Scanner::new(sfi.clone(), delimiter.clone());
                    let mut meta = None;
                    if let InputReference::File(path) = &input_ref {
                        meta = Some(fs::metadata(&path.canonical)?);
                    }
                    let mut input = Some(input_ref.open()?.tail(self.options.tail, delimiter.clone())?);
                    let is_file = |meta: &Option<fs::Metadata>| meta.as_ref().map(|m|m.is_file()).unwrap_or(false);
                    let process = |input: &mut Option<Input>, is_file: bool| {
                        if let Some(input) = input {
                            for (j, item) in scanner.items(&mut input.stream.as_sequential()).with_max_segment_size(self.options.max_message_size.into()).enumerate() {
                                if txi.send((i, j, item?)).is_err() {
                                    break;
                                }
                            }
                            Ok(!is_file)
                        } else {
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
                                        if old_meta.len() > new_meta.len() {
                                            input = None;
                                        }
                                        #[cfg(unix)]
                                        if old_meta.ino() != new_meta.ino() || old_meta.dev() != new_meta.dev() {
                                            input = None;
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
                                    Ok(())
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
                let worker = scope.spawn(closure!(ref bfo, ref parser, ref sfi, ref badges, clone rxi, clone txo, |_| {
                    self.process_segments(parser, bfo, sfi, badges, rxi, txo);
                }));
                workers.push(worker);
            }
            drop(txo);

            // spawn merger thread
            let merger = scope.spawn(|_| -> Result<()> {
                self.merge_segments(&badges, rxo, output, n)
            });

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

    fn process_segments(
        &self,
        parser: &Parser,
        bfo: &BufFactory,
        sfi: &SegmentBufFactory,
        badges: &FollowBadges,
        rxi: Receiver<(usize, usize, Segment)>,
        txo: Sender<(usize, Vec<u8>, TimestampIndex)>,
    ) {
        let mut processor = self.new_segment_processor(parser);
        for (i, j, segment) in rxi.iter() {
            let prefix = badges
                .input
                .as_ref()
                .map(|b| b[i].as_str())
                .unwrap_or(&badges.si.placeholder);
            match segment {
                Segment::Complete(segment) => {
                    let mut buf = bfo.new_buf();
                    let mut index_builder = TimestampIndexBuilder {
                        result: TimestampIndex::new(j),
                    };
                    processor.process(segment.data(), &mut buf, prefix, None, &mut index_builder);
                    sfi.recycle(segment);
                    if txo.send((i, buf, index_builder.result)).is_err() {
                        return;
                    };
                }
                Segment::Incomplete(_, _) => {}
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
        type Key = (Timestamp, usize, usize, usize); // (ts, input, block, offset)
        type Line = (Rc<Vec<u8>>, Range<usize>, Instant); // (buf, location, instant)

        let mut window = BTreeMap::<Key, Line>::new();
        let mut last_ts: Option<Timestamp> = None;
        let mut prev_ts: Option<Timestamp> = None;
        let mut mem_usage = 0;
        let mem_limit = concurrency * usize::from(self.options.buffer_size);

        loop {
            let deadline = Instant::now().checked_sub(self.options.sync_interval);
            while let Some(first) = window.first_key_value() {
                if deadline.map(|deadline| first.1.2 > deadline).unwrap_or(true) && mem_usage < mem_limit {
                    break;
                }
                if let Some(entry) = window.pop_first() {
                    let sync_indicator = if prev_ts.map(|ts| ts <= entry.0.0).unwrap_or(true) {
                        &badges.si.synced
                    } else {
                        &badges.si.failed
                    };
                    prev_ts = Some(entry.0.0);
                    mem_usage -= entry.1.1.end - entry.1.1.start;
                    output.write_all(sync_indicator.as_bytes())?;
                    output.write_all(&entry.1.0[entry.1.1.clone()][badges.si.width..])?;
                    output.write_all(self.options.output_delimiter.as_bytes())?;
                }
            }

            let next_ts = window.first_entry().map(|e| e.get().2);
            let timeout = if let (Some(next_ts), Some(deadline)) = (next_ts, deadline) {
                Some(max(deadline, next_ts) - next_ts)
            } else {
                None
            };
            match rxo.recv_timeout(timeout.unwrap_or(std::time::Duration::MAX)) {
                Ok((i, buf, index)) => {
                    let buf = Rc::new(buf);
                    for line in index.lines {
                        last_ts = Some(
                            last_ts
                                .map(|last_ts| std::cmp::max(last_ts, line.ts))
                                .unwrap_or(line.ts),
                        );
                        mem_usage += line.location.end - line.location.start;
                        let key = (line.ts, i, index.block, line.location.start);
                        let value = (buf.clone(), line.location, Instant::now());
                        window.insert(key, value);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    if timeout.is_none() {
                        break;
                    }
                }
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

    fn input_badges<'a, I: IntoIterator<Item = &'a InputReference>>(&self, inputs: I) -> Option<Vec<String>> {
        let name = |input: &InputReference| match input {
            InputReference::Stdin => "<stdin>".to_owned(),
            InputReference::File(path) => path.original.to_string_lossy().to_string(),
        };

        let names = inputs.into_iter().map(name).collect_vec();
        let mut badges = names.iter().map(|x| x.graphemes(true).collect_vec()).collect_vec();

        let ii = self.options.input_info;

        const II: InputInfoSet = enum_set!(InputInfo::Minimal | InputInfo::Compact | InputInfo::Full);

        if ii.intersection(II).is_empty() {
            return None;
        }

        if ii.contains(InputInfo::None) && badges.len() < 2 {
            return None;
        }

        let num_width = format!("{}", badges.len()).len();
        let opt = &self.punctuation;

        if ii.contains(InputInfo::Compact) {
            let pl = common_prefix_len(&badges);
            for badge in badges.iter_mut() {
                let cl = opt.input_name_clipping.width();
                if badge.len() > 24 + cl + pl {
                    if pl > 7 {
                        *badge = opt
                            .input_name_common_part
                            .graphemes(true)
                            .chain(badge[pl - 4..pl + 8].iter().cloned())
                            .chain(opt.input_name_clipping.graphemes(true))
                            .chain(badge[badge.len() - 16..].iter().cloned())
                            .collect();
                    } else {
                        *badge = badge[0..pl + 12]
                            .iter()
                            .cloned()
                            .chain(opt.input_name_clipping.graphemes(true))
                            .chain(badge[badge.len() - 12..].iter().cloned())
                            .collect();
                    }
                } else if pl > 7 {
                    *badge = opt
                        .input_name_common_part
                        .graphemes(true)
                        .chain(badge[pl - 4..].iter().cloned())
                        .collect();
                }
            }
        }

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

        Some(result)
    }

    fn new_segment_processor<'a>(&'a self, parser: &'a Parser) -> impl SegmentProcess + 'a {
        let options = SegmentProcessorOptions {
            allow_prefix: self.options.allow_prefix,
            allow_unparsed_data: self.options.filter.is_empty() && self.options.input_format.is_none(),
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
    fn process<O>(&mut self, data: &[u8], buf: &mut Vec<u8>, prefix: &str, limit: Option<usize>, observer: &mut O)
    where
        O: RecordObserver,
    {
        let mut i = 0;
        let limit = limit.unwrap_or(usize::MAX);

        for chunk in self.delim.split(data) {
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
                if parsed_some {
                    buf.extend(self.options.output_delimiter.as_bytes());
                }
                parsed_some = true;
                let record = self.parser.parse(&ar.record);
                if record.matches(&self.filter) {
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
                    self.formatter
                        .format_record(buf, prefix_range, record.with_source(&chunk[ar.offsets]));
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
}

impl TimestampIndex {
    fn new(block: usize) -> Self {
        Self {
            block,
            lines: Vec::new(),
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
