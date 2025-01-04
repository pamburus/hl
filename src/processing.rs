// std imports
use std::ops::Range;

// local imports
use crate::{
    formatting::v2::{RawRecordFormatter, RecordFormatter, RecordWithSourceFormatter},
    model::{
        v2::{
            parse::ParserSetup,
            record::{Filter as RecordFilter, Record, RecordWithSourceConstructor},
        },
        Filter,
    },
    scanning::{BufFactory, Delimit, Delimiter, Scanner, SearchExt, Segment, SegmentBuf, SegmentBufFactory},
    types::InputFormat,
};

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
    parser_setup: &'a ParserSetup,
    formatter: Formatter,
    filter: Filter,
    options: SegmentProcessorOptions,
}

impl<'a, Formatter: RecordWithSourceFormatter, Filter: RecordFilter> SegmentProcessor<'a, Formatter, Filter> {
    pub fn new(
        parser: &'a ParserSetup,
        formatter: Formatter,
        filter: Filter,
        options: SegmentProcessorOptions,
    ) -> Self {
        Self {
            parser_setup: parser,
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
        let mut parser = self.parser_setup.new_parser();

        let mut i = 0;
        let limit = limit.unwrap_or(usize::MAX);

        for line in self.options.delimiter.clone().into_searcher().split(data) {
            if line.len() == 0 {
                if self.show_unparsed() {
                    buf.push(b'\n');
                }
                continue;
            }

            // TODO: select input format
            // TODO: setup allow prefix
            let mut parsed_some = false;
            let mut produced_some = false;
            let mut last_offset = 0;
            let mut offset = 0;

            while let Ok(Some((record, span))) = parser.parse(crate::format::Json, &line[offset..]) {
                i += 1;
                last_offset = span.end.clone();
                if parsed_some {
                    buf.push(b'\n');
                }
                parsed_some = true;
                if record.matches(&self.filter) {
                    let begin = buf.len();
                    buf.extend(prefix.as_bytes());
                    buf.extend(&line[offset..span.start]);
                    self.formatter
                        .format_record(buf, record.with_source(&line[span.clone()]));
                    let end = buf.len();
                    observer.observe_record(&record, begin..end);
                    produced_some = true;
                }
                if i >= limit {
                    break;
                }
                offset = span.end;
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
