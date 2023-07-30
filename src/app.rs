// std imports
use std::cmp::{Reverse, max};
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::io::{BufWriter, Write};
use std::iter::repeat;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration,Instant};

// unix-only std imports
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

// third-party imports
use closure::closure;
use crossbeam_channel::{self as channel, Receiver, RecvError, Sender,RecvTimeoutError};
use crossbeam_utils::thread;
use itertools::{izip, Itertools};
use platform_dirs::AppDirs;
use serde_json as json;
use sha2::{Digest, Sha256};
use std::num::{NonZeroU32, NonZeroUsize};

// local imports
use crate::datefmt::{DateTimeFormat, DateTimeFormatter};
use crate::error::*;
use crate::fmtx::aligned_left;
use crate::fsmon::{self, EventKind};
use crate::formatting::RecordFormatter;
use crate::index::{Indexer, Timestamp};
use crate::input::{BlockLine, InputHolder, InputReference, Input};
use crate::model::{Filter, Parser, ParserSettings, RawRecord, Record};
use crate::scanning::{BufFactory, Scanner, Segment, SegmentBufFactory};
use crate::settings::{Fields, Formatting};
use crate::theme::{Element, StylingPush, Theme};
use crate::timezone::Tz;
use crate::IncludeExcludeKeyFilter;

// TODO: merge Options to Settings and replace Options with Settings.

// ---

pub struct Options {
    pub theme: Arc<Theme>,
    pub time_format: DateTimeFormat,
    pub raw_fields: bool,
    pub buffer_size: NonZeroUsize,
    pub max_message_size: NonZeroUsize,
    pub concurrency: usize,
    pub filter: Filter,
    pub fields: FieldOptions,
    pub formatting: Formatting,
    pub time_zone: Tz,
    pub hide_empty_fields: bool,
    pub sort: bool,
    pub follow: bool,
    pub sync_interval: Duration,
    pub input_info: Option<InputInfo>,
    pub dump_index: bool,
    pub app_dirs: Option<AppDirs>,
}

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

pub struct App {
    options: Options,
}

pub type Output = dyn Write + Send + Sync;

impl App {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    pub fn run(&self, inputs: Vec<InputHolder>, output: &mut Output) -> Result<()> {
        if self.options.follow {
            self.follow(inputs.into_iter().map(|x|x.reference).collect(), output)
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
            let (txo, rxo): (Vec<_>, Vec<_>) = (0..n).into_iter().map(|_| channel::bounded::<(usize, Vec<u8>)>(1)).unzip();
            // spawn reader thread
            let reader = scope.spawn(closure!(clone sfi, |_| -> Result<()> {
                let mut tx = StripedSender::new(txi);
                let scanner = Scanner::new(sfi, "\n".to_string());
                for (i, mut input) in inputs.into_iter().enumerate() {
                    for item in scanner.items(&mut input.stream).with_max_segment_size(self.options.max_message_size.into()) {
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
                    let mut formatter = self.formatter();
                    let mut processor = SegmentProcessor::new(&parser, &mut formatter, &self.options.filter);
                    for (i, segment) in rxi.iter() {
                        let prefix = input_badges.as_ref().map(|b|b[i].as_str()).unwrap_or("");
                        match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                processor.run(segment.data(), &mut buf, prefix, &mut RecordIgnorer{});
                                sfi.recycle(segment);
                                if let Err(_) = txo.send((i, buf)) {
                                    break;
                                };
                            }
                            Segment::Incomplete(segment, _) => {
                                if let Err(_) = txo.send((i, segment.to_vec())) {
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
                    output.write_all(&buf[..])?;
                    bfo.recycle(buf);
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
        let param_hash = hex::encode(self.parameters_hash()?);
        let cache_dir = self
            .options
            .app_dirs
            .as_ref()
            .map(|dirs| dirs.cache_dir.clone())
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join(param_hash);
        fs::create_dir_all(&cache_dir)?;
        let indexer = Indexer::new(
            self.options.concurrency,
            NonZeroU32::try_from(self.options.buffer_size)?.try_into()?,
            NonZeroU32::try_from(self.options.max_message_size)?.try_into()?,
            cache_dir,
            &self.options.fields.settings.predefined,
        );

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
                        if let Some(level) = self.options.filter.level {
                            if !src.match_level(level) {
                                return None;
                            }
                        }
                        let offset = block.offset();
                        src.stat
                            .ts_min_max
                            .map(|(ts_min, ts_max)| (block, ts_min, ts_max, i, offset))
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
                    let mut formatter = self.formatter();
                    for (block, ts_min, i, j) in rxp.iter() {
                        let mut buf = Vec::with_capacity(2 * usize::try_from(block.size())?);
                        let mut items = Vec::with_capacity(2 * usize::try_from(block.lines_valid())?);
                        for line in block.into_lines()? {
                            if line.len() == 0 {
                                continue;
                            }
                            if let Ok(record) = json::from_slice(line.bytes()) {
                                let record = parser.parse(record);
                                if record.matches(&self.options.filter) {
                                    let offset = buf.len();
                                    formatter.format_record(&mut buf, &record);
                                    if let Some(ts) = record.ts {
                                        if let Some(unix_ts) = ts.unix_utc() {
                                            items.push((unix_ts.into(), offset..buf.len()));
                                        } else {
                                            eprintln!("skipped message because timestamp cannot be parsed: {:#?}", ts)
                                        }
                                    } else {
                                        eprintln!("skipped message with missing timestamp")
                                    }
                                }
                            }
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
                    let scanner = Scanner::new(sfi.clone(), "\n".to_string());
                    let mut meta = None;
                    if let InputReference::File(filename) = &input_ref { 
                        meta = Some(fs::metadata(filename)?);
                    }
                    let mut input = Some(input_ref.open()?);
                    let is_file = |meta: &Option<fs::Metadata>| meta.as_ref().map(|m|m.is_file()).unwrap_or(false);
                    let process = |input: &mut Option<Input>, is_file: bool| {
                        if let Some(input) = input {
                            for (j, item) in scanner.items(&mut input.stream).with_max_segment_size(self.options.max_message_size.into()).enumerate() {
                                if txi.send((i, j, item?)).is_err() {
                                    break;
                                }
                            }
                            Ok(!is_file)
                        } else {
                            Ok(false)
                        }
                    };
                    if let InputReference::File(filename) = &input_ref {
                        if process(&mut input, is_file(&meta))? {
                            return Ok(())
                        }
                        fsmon::run(vec![filename.clone()], |event| {
                            match event.kind {
                                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any | EventKind::Other => {
                                    if let (Some(old_meta), Ok(new_meta)) = (&meta, fs::metadata(&filename)) {
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
                    let mut formatter = self.formatter();
                    let mut processor = SegmentProcessor::new(&parser, &mut formatter, &self.options.filter);
                    for (i, j, segment) in rxi.iter() {
                        let prefix = input_badges.as_ref().map(|b|b[i].as_str()).unwrap_or("");
                        match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                let mut index_builder = TimestampIndexBuilder{result: TimestampIndex::new(j)};
                                processor.run(segment.data(), &mut buf, prefix, &mut index_builder);
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

    fn parameters_hash(&self) -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();
        bincode::serialize_into(
            &mut hasher,
            &(
                &self.options.buffer_size,
                &self.options.max_message_size,
                &self.options.fields.settings.predefined,
            ),
        )?;
        Ok(hasher.finalize().into())
    }

    fn parser(&self) -> Parser {
        Parser::new(ParserSettings::new(
            &self.options.fields.settings.predefined,
            &self.options.fields.settings.ignore,
            self.options.filter.since.is_some() || self.options.filter.until.is_some() || self.options.follow,
        ))
    }

    fn formatter(&self) -> RecordFormatter {
        RecordFormatter::new(
            self.options.theme.clone(),
            DateTimeFormatter::new(self.options.time_format.clone(), self.options.time_zone),
            self.options.hide_empty_fields,
            self.options.fields.filter.clone(),
            self.options.formatting.clone(),
        )
        .with_field_unescaping(!self.options.raw_fields)
    }

    fn input_badges<'a, I: IntoIterator<Item = &'a InputReference>>(&self, inputs: I) -> Option<Vec<String>> {
        let name = |input: &InputReference| match input {
            InputReference::Stdin => "<stdin>".to_owned(),
            InputReference::File(path) => path.to_string_lossy().to_string(),
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
}

// ---

pub struct SegmentProcessor<'a> {
    parser: &'a Parser,
    formatter: &'a mut RecordFormatter,
    filter: &'a Filter,
}

impl<'a> SegmentProcessor<'a> {
    pub fn new(parser: &'a Parser, formatter: &'a mut RecordFormatter, filter: &'a Filter) -> Self {
        Self {
            parser,
            formatter,
            filter,
        }
    }

    pub fn run<O>(&mut self, data: &[u8], buf: &mut Vec<u8>, prefix: &str, observer: &mut O)
    where
        O: RecordObserver,
    {
        for data in rtrim(data, b'\n').split(|c| *c == b'\n') {
            if data.len() == 0 {
                continue;
            }
            let mut stream = json::Deserializer::from_slice(data).into_iter::<RawRecord>();
            let mut some = false;
            while let Some(Ok(record)) = stream.next() {
                some = true;
                let record = self.parser.parse(record);
                if record.matches(self.filter) {
                    let begin = buf.len();
                    buf.extend(prefix.as_bytes());
                    self.formatter.format_record(buf, &record);
                    let end = buf.len();
                    observer.observe_record(&record, begin..end);
                }
            }
            let remainder = if some { &data[stream.byte_offset()..] } else { data };
            if remainder.len() != 0 && self.filter.is_empty() {
                buf.extend_from_slice(remainder);
                buf.push(b'\n');
            }
        }
    }
}

// ---

pub trait RecordObserver {
    fn observe_record<'a>(&mut self, record: &'a Record<'a>, location: Range<usize>);
}

// ---

pub struct RecordIgnorer {}

impl RecordObserver for RecordIgnorer {
    fn observe_record<'a>(&mut self, _: &'a Record<'a>, _: Range<usize>) {}
}

// ---

struct TimestampIndexBuilder {
    result: TimestampIndex,
}

impl RecordObserver for TimestampIndexBuilder {
    fn observe_record<'a>(&mut self, record: &'a Record<'a>, location: Range<usize>) {
        if let Some(ts) = record.ts.as_ref().and_then(|ts| ts.unix_utc()).map(|ts| ts.into()) {
            self.result.lines.push(TimestampIndexLine { location, ts });
        }
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

fn rtrim<'a>(s: &'a [u8], c: u8) -> &'a [u8] {
    if s.len() > 0 && s[s.len() - 1] == c {
        &s[..s.len() - 1]
    } else {
        s
    }
}

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
