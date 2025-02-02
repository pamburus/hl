// std imports
use std::{
    cmp::{max, Reverse},
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    fs,
    io::{BufWriter, Write},
    iter::repeat,
    num::NonZeroUsize,
    ops::Range,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

// unix-only std imports
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

// third-party imports
use closure::closure;
use crossbeam_channel::{self as channel, Receiver, RecvError, RecvTimeoutError, Sender};
use crossbeam_utils::thread;
use itertools::{izip, Itertools};
use serde::{Deserialize, Serialize};

// local imports
use crate::{
    appdirs::AppDirs,
    datefmt::{DateTimeFormat, DateTimeFormatter},
    fmtx::aligned_left,
    formatting::v2::{RawRecordFormatter, RecordFormatter, RecordWithSourceFormatter},
    fsmon::{self, EventKind},
    index::{Indexer, IndexerSettings, Timestamp},
    input::{BlockLine, Input, InputHolder, InputReference},
    model::v2::compat::{Filter, ParserSettings, Record, RecordFilter},
    processing::{RecordIgnorer, RecordObserver, SegmentProcess, SegmentProcessor, SegmentProcessorOptions},
    query::Query,
    scanning::{BufFactory, Delimiter, Scanner, Segment, SegmentBuf, SegmentBufFactory},
    settings::{FieldShowOption, Fields, Formatting},
    theme::{Element, StylingPush, Theme},
    timezone::Tz,
    types,
    vfs::LocalFileSystem,
    IncludeExcludeKeyFilter,
    {error::*, QueryNone},
};

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
    pub filter: Filter,
    pub query: Option<Query>,
    pub fields: FieldOptions,
    pub formatting: Formatting,
    pub time_zone: Tz,
    pub hide_empty_fields: bool,
    pub sort: bool,
    pub follow: bool,
    pub sync_interval: Duration,
    pub input_info: Option<InputInfo>,
    pub input_format: Option<InputFormat>,
    pub dump_index: bool,
    pub app_dirs: Option<AppDirs>,
    pub tail: u64,
    pub delimiter: Delimiter,
    pub unix_ts_unit: Option<UnixTimestampUnit>,
    pub flatten: bool,
}

impl Options {
    fn filter_and_query<'a>(&'a self) -> Box<dyn RecordFilter + 'a> {
        match (self.filter.is_empty(), &self.query) {
            (true, None) => Box::new(QueryNone {}),
            (false, None) => Box::new(&self.filter),
            (true, Some(query)) => Box::new(query),
            (false, Some(query)) => Box::new((&self.filter).and(query)),
        }
    }

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
    fn with_filter(self, filter: Filter) -> Self {
        Self { filter, ..self }
    }

    #[cfg(test)]
    fn with_input_info(self, input_info: Option<InputInfo>) -> Self {
        Self { input_info, ..self }
    }
}

#[derive(Default)]
pub struct FieldOptions {
    pub filter: Arc<IncludeExcludeKeyFilter>,
    pub settings: Fields,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum InputInfo {
    Auto,
    Full,
    Compact,
    Minimal,
}

pub type InputFormat = types::InputFormat;

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

pub struct App {
    options: Options,
}

pub type Output = dyn Write + Send + Sync;

impl App {
    pub fn new(mut options: Options) -> Self {
        if options.raw && options.input_info == Some(InputInfo::Auto) {
            options.input_info = None
        }
        Self { options }
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
        let sfi = Arc::new(SegmentBufFactory::new(self.options.buffer_size.try_into()?));
        let bfo = BufFactory::new(self.options.buffer_size.try_into()?);
        let parser = self.parser();
        thread::scope(|scope| -> Result<()> {
            // prepare receive/transmit channels for input data
            let (txi, rxi): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded(1)).unzip();
            // prepare receive/transmit channels for output data
            let (txo, rxo): (Vec<_>, Vec<_>) = (0..n).into_iter().map(|_| channel::bounded::<(usize, SegmentBuf)>(1)).unzip();
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
                    let mut processor = self.new_segment_processor(&parser);
                    for (i, segment) in rxi.iter() {
                        let prefix = input_badges.as_ref().map(|b|b[i].as_str()).unwrap_or("");
                        match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                processor.process(segment.data(), &mut buf, prefix, None, &mut RecordIgnorer{});
                                sfi.recycle(segment);
                                if let Err(_) = txo.send((i, buf.into())) {
                                    break;
                                };
                            }
                            Segment::Incomplete(segment, _) => {
                                if let Err(_) = txo.send((i, segment)) {
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
        let indexer_settings = IndexerSettings::new(
            LocalFileSystem,
            self.options.buffer_size.try_into()?,
            self.options.max_message_size.try_into()?,
            &self.options.fields.settings.predefined,
            self.options.delimiter.clone(),
            self.options.allow_prefix,
            self.options.unix_ts_unit,
            self.options.input_format,
        );
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
            .map(|x| x.index(&indexer))
            .collect::<Result<Vec<_>>>()?;

        if self.options.dump_index {
            for input in inputs {
                for block in input.into_blocks().sorted() {
                    writeln!(output, "block at {} with size {}", block.offset(), block.size())?;
                    writeln!(output, "{:#?}", block.source_block())?;
                    let block_offset = block.offset();
                    for line in block.into_lines()? {
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
                    .map(|(i, input)| input.into_blocks().map(move |block| (block, i)))
                    .flatten()
                    .filter_map(|(block, i)| {
                        let src = block.source_block();
                        if src.stat.lines_valid == 0 {
                            return None;
                        }
                        if let Some((ts_min, ts_max)) = src.stat.ts_min_max {
                            if let Some(until) = self.options.filter.until {
                                if ts_min > until.into() {
                                    return None;
                                }
                            }
                            if let Some(since) = self.options.filter.since {
                                if ts_max < since.into() {
                                    return None;
                                }
                            }
                            if let Some(level) = self.options.filter.level {
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
                workers.push(scope.spawn(closure!(ref parser, |_| -> Result<()> {
                    let mut processor = self.new_segment_processor(&parser);
                    for (block, ts_min, i, j) in rxp.iter() {
                        let mut buf = Vec::with_capacity(2 * usize::try_from(block.size())?);
                        let mut items = Vec::with_capacity(2 * usize::try_from(block.lines_valid())?);
                        for line in block.into_lines()? {
                            if line.len() == 0 {
                                continue;
                            }
                            processor.process(
                                line.bytes(),
                                &mut buf,
                                "",
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
                    while tso >= tsi || workspace.len() == 0 {
                        if let Some((block, i, j)) = input.next() {
                            tsi = Some(block.ts_min.clone());
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

                    if done && workspace.len() == 0 {
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
                    if let Some(badges) = &input_badges {
                        output.write_all(&badges[item.2].as_bytes())?;
                    }
                    output.write_all((item.0).1.bytes())?;
                    output.write_all(&[b'\n'])?;
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

    fn follow(&self, inputs: Vec<InputReference>, output: &mut Output) -> Result<()> {
        let input_badges = self.input_badges(inputs.iter());

        let m = inputs.len();
        let n = self.options.concurrency;
        let parser = self.parser();
        let sfi = Arc::new(SegmentBufFactory::new(self.options.buffer_size.try_into()?));
        let bfo = BufFactory::new(self.options.buffer_size.try_into()?);
        thread::scope(|scope| -> Result<()> {
            // prepare receive/transmit channels for input data
            let (txi, rxi) = channel::bounded(1);
            // prepare receive/transmit channels for output data
            let (txo, rxo) = channel::bounded(1);
            // spawn reader threads
            let mut readers = Vec::with_capacity(m);
            for (i, input_ref) in inputs.into_iter().enumerate() {
                let reader = scope.spawn(closure!(clone sfi, clone txi, |_| -> Result<()> {
                    let scanner = Scanner::new(sfi.clone(), &self.options.delimiter);
                    let mut meta = None;
                    if let InputReference::File(path) = &input_ref {
                        meta = Some(fs::metadata(&path.canonical)?);
                    }
                    let mut input = Some(input_ref.open()?.tail(self.options.tail)?);
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
                let worker = scope.spawn(closure!(ref bfo, ref parser, ref sfi, ref input_badges, clone rxi, clone txo, |_| {
                    let mut processor = self.new_segment_processor(&parser);
                    for (i, j, segment) in rxi.iter() {
                        let prefix = input_badges.as_ref().map(|b|b[i].as_str()).unwrap_or("");
                        match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                let mut index_builder = TimestampIndexBuilder{result: TimestampIndex::new(j)};
                                processor.process(segment.data(), &mut buf, prefix, None, &mut index_builder);
                                sfi.recycle(segment);
                                if txo.send((i, buf, index_builder.result)).is_err() {
                                    return;
                                };
                            }
                            Segment::Incomplete(_, _) => {}
                        }
                    }
                }));
                workers.push(worker);
            }
            drop(txo);

            // spawn merger thread
            let merger = scope.spawn(move |_| -> Result<()> {
                type Key = (Timestamp, usize, usize, usize); // (ts, input, block, offset)
                type Line = (Rc<Vec<u8>>, Range<usize>, Instant); // (buf, location, instant)

                let mut window = BTreeMap::<Key,Line>::new();
                let mut last_ts: Option<Timestamp> = None;
                let mut prev_ts: Option<Timestamp> = None;
                let mut mem_usage = 0;
                let mem_limit = n * usize::from(self.options.buffer_size);

                loop {
                    let deadline = Instant::now().checked_sub(self.options.sync_interval);
                    while let Some(first) = window.first_key_value() {
                        if deadline.map(|deadline| first.1.2 > deadline).unwrap_or(true) && mem_usage < mem_limit {
                            break;
                        }
                        if let Some(entry) = window.pop_first() {
                            let sync_indicator = if prev_ts.map(|ts| ts <= entry.0.0).unwrap_or(true) {
                                &self.options.theme.indicators.sync.synced
                            } else {
                                &self.options.theme.indicators.sync.failed
                            };
                            prev_ts = Some(entry.0.0);
                            mem_usage -= entry.1.1.end - entry.1.1.start;
                            output.write_all(sync_indicator.value.as_bytes())?;
                            output.write_all(&entry.1.0[entry.1.1.clone()])?;
                            output.write_all(&[b'\n'])?;
                        }
                    }

                    let next_ts = window.first_entry().map(|e|e.get().2);
                    let timeout = if let (Some(next_ts), Some(deadline)) = (next_ts, deadline) {
                        Some(max(deadline, next_ts) - next_ts)
                    } else {
                        None
                    };
                    match rxo.recv_timeout(timeout.unwrap_or(std::time::Duration::MAX)) {
                        Ok((i, buf, index)) => {
                            let buf = Rc::new(buf);
                            for line in index.lines {
                                last_ts = Some(last_ts.map(|last_ts| std::cmp::max(last_ts, line.ts)).unwrap_or(line.ts));
                                mem_usage += line.location.end - line.location.start;
                                let key = (line.ts, i, index.block, line.location.start);
                                let value = (buf.clone(), line.location, Instant::now());
                                window.insert(key, value);
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => {
                            if timeout.is_none() {
                                break
                            }
                        }
                    }
                }

                Ok(())
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

    fn parser(&self) -> ParserSettings {
        ParserSettings::new(&self.options.fields.settings.predefined)
            .with_ignore(&self.options.fields.settings.ignore)
            .with_unix_timestamp_unit(self.options.unix_ts_unit)
    }

    fn formatter(&self) -> Box<dyn RecordWithSourceFormatter> {
        if self.options.raw {
            Box::new(RawRecordFormatter {})
        } else {
            Box::new(
                RecordFormatter::new(
                    self.options.theme.clone(),
                    DateTimeFormatter::new(self.options.time_format.clone(), self.options.time_zone),
                    self.options.hide_empty_fields,
                    self.options.fields.filter.clone(),
                    self.options.formatting.clone(),
                )
                .with_field_unescaping(!self.options.raw_fields)
                .with_flatten(self.options.flatten)
                .with_always_show_time(self.options.fields.settings.predefined.time.show == FieldShowOption::Always)
                .with_always_show_level(self.options.fields.settings.predefined.level.show == FieldShowOption::Always),
            )
        }
    }

    fn input_badges<'a, I: IntoIterator<Item = &'a InputReference>>(&self, inputs: I) -> Option<Vec<String>> {
        let name = |input: &InputReference| match input {
            InputReference::Stdin => "<stdin>".to_owned(),
            InputReference::File(path) => path.original.to_string_lossy().to_string(),
        };

        let mut badges = inputs.into_iter().map(|x| name(x).chars().collect_vec()).collect_vec();

        match &self.options.input_info {
            None => return None,
            Some(InputInfo::Auto) => {
                if badges.len() < 2 {
                    return None;
                }
            }
            _ => {}
        };

        let num_width = format!("{}", badges.len()).len();
        let opt = &self.options.formatting.punctuation;

        if let Some(InputInfo::Compact | InputInfo::Auto) = self.options.input_info {
            let pl = common_prefix_len(&badges);
            for badge in badges.iter_mut() {
                let cl = opt.input_name_clipping.chars().count();
                if badge.len() > 24 + cl + pl {
                    if pl > 7 {
                        *badge = opt
                            .input_name_common_part
                            .chars()
                            .chain(badge[pl - 4..pl + 8].iter().cloned())
                            .chain(opt.input_name_clipping.chars())
                            .chain(badge[badge.len() - 16..].iter().cloned())
                            .collect();
                    } else {
                        *badge = badge[0..pl + 12]
                            .iter()
                            .cloned()
                            .chain(opt.input_name_clipping.chars())
                            .chain(badge[badge.len() - 12..].iter().cloned())
                            .collect();
                    }
                } else if pl > 7 {
                    *badge = opt
                        .input_name_common_part
                        .chars()
                        .chain(badge[pl - 4..].iter().cloned())
                        .collect();
                }
            }
        }

        if let Some(max_len) = badges.iter().map(|badge| badge.len()).max() {
            for badge in badges.iter_mut() {
                badge.extend(repeat(' ').take(max_len - badge.len()));
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
                                aligned_left(buf, num_width + 1, b' ', |mut buf| {
                                    buf.extend_from_slice(opt.input_number_prefix.as_bytes());
                                    buf.extend_from_slice(format!("{}", i).as_bytes());
                                });
                                buf.extend(opt.input_name_left_separator.as_bytes());
                            });
                        });
                        s.batch(|buf| buf.extend(opt.input_number_right_separator.as_bytes()));
                    });

                    if self.options.input_info != Some(InputInfo::Minimal) {
                        s.element(Element::InputName, |s| {
                            s.batch(|buf| buf.extend(opt.input_name_left_separator.as_bytes()));
                            s.element(Element::InputNameInner, |s| {
                                s.batch(|buf| {
                                    buf.extend_from_slice(badge.iter().collect::<String>().as_bytes());
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

    fn new_segment_processor<'s>(&'s self, parser: &'s ParserSettings) -> impl SegmentProcess + 's {
        let options = SegmentProcessorOptions {
            allow_prefix: self.options.allow_prefix,
            allow_unparsed_data: self.options.filter.is_empty() && self.options.query.is_none(),
            delimiter: self.options.delimiter.clone(),
            input_format: self.options.input_format,
        };

        SegmentProcessor::new(parser, self.formatter(), self.options.filter_and_query(), options)
    }
}

// ---
/*

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
}

// ---

pub struct SegmentProcessor<'a, Formatter, Filter> {
    parser: &'a Parser,
    formatter: Formatter,
    filter: Filter,
    options: SegmentProcessorOptions,
}

impl<'a, Formatter: RecordWithSourceFormatter, Filter: RecordFilter> SegmentProcessor<'a, Formatter, Filter> {
    pub fn new(parser: &'a Parser, formatter: Formatter, filter: Filter, options: SegmentProcessorOptions) -> Self {
        Self {
            parser,
            formatter,
            filter,
            options,
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

        for line in self.options.delimiter.clone().into_searcher().split(data) {
            if line.len() == 0 {
                if self.show_unparsed() {
                    buf.push(b'\n');
                }
                continue;
            }

            let mut stream = RawRecord::parser()
                .allow_prefix(self.options.allow_prefix)
                .format(self.options.input_format)
                .parse(line);
            let mut parsed_some = false;
            let mut produced_some = false;
            let mut last_offset = 0;
            while let Some(Ok(ar)) = stream.next() {
                i += 1;
                last_offset = ar.offsets.end;
                if parsed_some {
                    buf.push(b'\n');
                }
                parsed_some = true;
                let record = self.parser.parse(&ar.record);
                if record.matches(&self.filter) {
                    let begin = buf.len();
                    buf.extend(prefix.as_bytes());
                    buf.extend(ar.prefix);
                    if ar.prefix.last().map(|&x| x == b' ') == Some(false) {
                        buf.push(b' ');
                    }
                    self.formatter.format_record(buf, record.with_source(&line[ar.offsets]));
                    let end = buf.len();
                    observer.observe_record(&record, begin..end);
                    produced_some = true;
                }
                if i >= limit {
                    break;
                }
            }
            let remainder = if parsed_some { &line[last_offset..] } else { line };
            if remainder.len() != 0 && self.show_unparsed() {
                if !parsed_some {
                    buf.extend(prefix.as_bytes());
                }
                if !parsed_some || produced_some {
                    buf.extend_from_slice(remainder);
                    buf.push(b'\n');
                }
            } else if produced_some {
                buf.push(b'\n');
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
 */
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
    pub fn into_lines(self) -> impl Iterator<Item = (Timestamp, BlockLine)> {
        let buf = self.buf;
        self.items
            .into_iter()
            .map(move |(ts, range)| (ts, BlockLine::new(buf.clone(), range.clone())))
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
        let item = match self.input[self.sn].recv() {
            Ok(item) => Some(item),
            Err(RecvError) => None,
        }?;
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

fn common_prefix_len<'a, V, I>(items: &'a Vec<I>) -> usize
where
    V: 'a + Eq + PartialEq + Copy,
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
                if item[i] != b {
                    return i;
                }
            } else {
                b = Some(item[i])
            }
        }
        i += 1;
    }
}

// ---

#[cfg(test)]
mod tests {
    // super imports
    use super::*;

    // std imports
    use std::io::Cursor;

    // third-party imports
    use chrono_tz::UTC;
    use maplit::hashmap;

    // local imports
    use crate::{
        filtering::MatchOptions, level::Level, model::v2::compat::FieldFilterSet, settings, themecfg::testing,
        LinuxDateFormat,
    };

    #[test]
    fn test_common_prefix_len() {
        let items = vec!["abc", "abcd", "ab", "ab"];
        assert_eq!(common_prefix_len(&items), 2);
    }

    #[test]
    fn test_cat_empty() {
        let input = input("");
        let mut output = Vec::new();
        let app = App::new(options());
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(std::str::from_utf8(&output).unwrap(), "");
    }

    #[test]
    fn test_cat_one_line() {
        let input = input(
            r#"{"caller":"main.go:539","duration":"15d","level":"info","msg":"No time or size retention was set so using the default time retention","ts":"2023-12-07T20:07:05.949Z"}"#,
        );
        let mut output = Vec::new();
        let app = App::new(options());
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2023-12-07 20:07:05.949 |INF| No time or size retention was set so using the default time retention duration=15d @ main.go:539\n",
        );
    }

    #[test]
    fn test_cat_with_theme() {
        let input = input(
            r#"{"caller":"main.go:539","duration":"15d","level":"warning","msg":"No time or size retention was set so using the default time retention","ts":"2023-12-07T20:07:05.949Z"}"#,
        );
        let mut output = Vec::new();
        let app = App::new(options().with_theme(theme()));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;7;33m|WRN|\u{1b}[0m \u{1b}[0;1;39mNo time or size retention was set so using the default time retention \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0;39m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
        );
    }

    #[test]
    fn test_cat_no_msg() {
        let input =
            input(r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z"}"#);
        let mut output = Vec::new();
        let app = App::new(options().with_theme(theme()));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;36m|INF|\u{1b}[0m \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0;39m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
        );
    }

    #[test]
    fn test_cat_msg_array() {
        let input = input(
            r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":["x","y"]}"#,
        );
        let mut output = Vec::new();
        let app = App::new(options().with_theme(theme()));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;36m|INF| \u{1b}[0;32mmsg\u{1b}[0;2m=\u{1b}[0;93m[\u{1b}[0;39mx\u{1b}[0;93m \u{1b}[0;39my\u{1b}[0;93m] \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0;39m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
        );
    }

    #[test]
    fn test_cat_field_exclude() {
        let input = input(
            r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":"xy"}"#,
        );
        let mut output = Vec::new();
        let mut ff = IncludeExcludeKeyFilter::new(MatchOptions::default());
        ff.entry("duration").exclude();
        let app = App::new(options().with_fields(FieldOptions {
            filter: Arc::new(ff),
            ..FieldOptions::default()
        }));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2023-12-07 20:07:05.949 |INF| xy ... @ main.go:539\n",
        );
    }

    #[test]
    fn test_cat_raw_fields() {
        let input = input(
            r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":"xy"}"#,
        );
        let mut output = Vec::new();
        let mut ff = IncludeExcludeKeyFilter::new(MatchOptions::default());
        ff.entry("duration").exclude();
        let app = App::new(options().with_raw_fields(true));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2023-12-07 20:07:05.949 |INF| xy duration=15d @ main.go:539\n",
        );
    }

    #[test]
    fn test_cat_raw_multiple_inputs() {
        let input1 =
            r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":"xy"}"#;
        let input2 =
            r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:06.944Z","msg":"xy"}"#;
        let mut output = Vec::new();
        let mut ff = IncludeExcludeKeyFilter::new(MatchOptions::default());
        ff.entry("duration").exclude();
        let app = App::new(options().with_input_info(Some(InputInfo::Auto)).with_raw(true));
        app.run(vec![input(input1), input(input2)], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            format!("{}\n{}\n", input1, input2),
        );
    }

    #[test]
    fn test_smart_delim_combo() {
        const L1: &str = r#"{}"#;
        const L2: &str = r#"{}"#;

        let input = input(format!("{}\n\r\n{}\n", L1, L2));
        let mut output = Vec::new();
        let app = App::new(options().with_raw(true));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(std::str::from_utf8(&output).unwrap(), format!("{}\n\n{}\n", L1, L2),);
    }

    #[test]
    fn test_sort_with_blank_lines() {
        let input = input(concat!(
            r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
            "\n\r\n",
            r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
            "\n",
        ));

        let mut output = Vec::new();
        let app = App::new(options().with_sort(true));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            concat!(
                "2024-01-25 18:09:16.860 |DBG| m1\n",
                "2024-01-25 18:10:20.435 |DBG| m2\n",
            ),
        );
    }

    #[test]
    fn test_filter_with_blank_lines() {
        let input = input(concat!(
            r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
            "\n\r\n",
            r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
            "\n",
        ));

        let mut output = Vec::new();
        let app = App::new(options().with_filter(Filter {
            fields: FieldFilterSet::new(["msg=m2"]).unwrap(),
            ..Default::default()
        }));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2024-01-25 18:10:20.435 |DBG| m2\n",
        );
    }

    #[test]
    fn test_sort_with_clingy_lines() {
        let input = input(concat!(
            r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
            r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
            "\n",
        ));

        let mut output = Vec::new();
        let app = App::new(options().with_sort(true));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            concat!(
                "2024-01-25 18:09:16.860 |DBG| m1\n",
                "2024-01-25 18:10:20.435 |DBG| m2\n",
            ),
        );
    }

    #[test]
    fn test_sort_with_clingy_and_invalid_lines() {
        let input = input(concat!(
            r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
            r#"{invalid}"#,
            r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
            "\n",
        ));

        let mut output = Vec::new();
        let app = App::new(options().with_sort(true));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2024-01-25 18:10:20.435 |DBG| m2\n",
        );
    }

    #[test]
    fn test_hide_by_prefix() {
        let input = input(concat!(
            r#"level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11"#,
            "\n",
        ));

        let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::default());
        filter.entry("a.b").exclude();

        let mut output = Vec::new();
        let app = App::new(options().with_fields(FieldOptions {
            filter: Arc::new(filter),
            ..FieldOptions::default()
        }));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2024-01-25 18:10:20.435 |DBG| m1 a.c.b=11 ...\n",
        );
    }

    #[test]
    fn test_hide_by_prefix_and_reveal_child() {
        let input = input(concat!(
            r#"level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11"#,
            "\n",
        ));

        let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::default());
        filter.entry("a.b").exclude();
        filter.entry("a.b.d").include();

        let mut output = Vec::new();
        let app = App::new(options().with_fields(FieldOptions {
            filter: Arc::new(filter),
            ..FieldOptions::default()
        }));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2024-01-25 18:10:20.435 |DBG| m1 a.b.d=20 a.c.b=11 ...\n",
        );
    }

    #[test]
    fn test_incomplete_segment() {
        let input = input(concat!(
            "level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11\n",
            "level=debug time=2024-01-25T19:10:21.764733+01:00 msg=m2 x=2\n"
        ));

        let mut output = Vec::new();
        let app = App::new(Options {
            buffer_size: NonZeroUsize::new(32).unwrap(),
            max_message_size: NonZeroUsize::new(64).unwrap(),
            ..options()
        });
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            concat!(
                "level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11\n",
                "2024-01-25 18:10:21.764 |DBG| m2 x=2\n",
            )
        );
    }

    #[test]
    fn test_incomplete_segment_sorted() {
        let data = concat!(
            "level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11\n",
            "level=debug time=2024-01-25T19:10:21.764733+01:00 msg=m2 x=2\n",
        );
        let input = input(data);

        let mut output = Vec::new();
        let app = App::new(Options {
            buffer_size: NonZeroUsize::new(16).unwrap(),
            max_message_size: NonZeroUsize::new(64).unwrap(),
            sort: true,
            ..options()
        });
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2024-01-25 18:10:21.764 |DBG| m2 x=2\n"
        );
    }

    #[test]
    fn test_issue_288_t1() {
        let input = input(concat!(
            r#"time="2024-06-04 17:14:35.190733+0200" level=INF msg="An INFO log message" logger=aLogger caller=aCaller"#,
            "\n",
        ));

        let mut output = Vec::new();
        let app = App::new(options().with_fields(FieldOptions {
            settings: Fields {
                predefined: settings::PredefinedFields {
                    level: settings::LevelField {
                        variants: vec![settings::LevelFieldVariant {
                            names: vec!["level".to_string()],
                            values: hashmap! {
                                Level::Debug => vec!["dbg".to_string()],
                                Level::Info => vec!["INF".to_string()],
                                Level::Warning => vec!["wrn".to_string()],
                                Level::Error => vec!["ERR".to_string()],
                            },
                            level: None,
                        }],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }));
        app.run(vec![input], &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            "2024-06-04 15:14:35.190 |INF| aLogger: An INFO log message @ aCaller\n",
        );
    }

    fn input<S: Into<String>>(s: S) -> InputHolder {
        InputHolder::new(InputReference::Stdin, Some(Box::new(Cursor::new(s.into()))))
    }

    fn options() -> Options {
        Options {
            theme: Arc::new(Theme::none()),
            time_format: LinuxDateFormat::new("%Y-%m-%d %T.%3N").compile(),
            raw: false,
            raw_fields: false,
            allow_prefix: false,
            buffer_size: NonZeroUsize::new(4096).unwrap(),
            max_message_size: NonZeroUsize::new(4096 * 1024).unwrap(),
            concurrency: 1,
            filter: Filter::default(),
            query: None,
            fields: FieldOptions::default(),
            formatting: Formatting::default(),
            time_zone: Tz::IANA(UTC),
            hide_empty_fields: false,
            sort: false,
            follow: false,
            sync_interval: Duration::from_secs(1),
            input_info: None,
            input_format: None,
            dump_index: false,
            app_dirs: None,
            tail: 0,
            delimiter: Delimiter::default(),
            unix_ts_unit: None,
            flatten: false,
        }
    }

    fn theme() -> Arc<Theme> {
        Arc::new(Theme::from(testing::theme().unwrap()))
    }
}
