// std imports
use std::ops::Range;

// local imports
use crate::{
    formatting::v2::{AbstractRecordFormatter, RawRecordFormatter, RecordFormatter},
    model::{
        v2::{
            parse::{NewParser, Parse, Parser, Settings as ParserSettings},
            record::{Filter as RecordFilter, Record, RecordWithSourceConstructor},
        },
        Filter,
    },
    scanning::{BufFactory, Delimit, Delimiter, Scanner, SearchExt, Segment, SegmentBuf, SegmentBufFactory},
    types::InputFormat,
};

pub trait SegmentProcess {
    fn process<'s, 'a, 'b, 'o, O: RecordObserver>(
        &'s mut self,
        data: &'a [u8],
        buf: &'b mut Vec<u8>,
        prefix: &str,
        limit: Option<usize>,
        observer: &'o mut O,
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

pub struct SegmentProcessor<'p, Formatter, Filter> {
    parser: &'p ParserSettings,
    formatter: Formatter,
    filter: Filter,
    options: SegmentProcessorOptions,
    delim: <Delimiter as Delimit>::Searcher,
}

impl<'p, Formatter: AbstractRecordFormatter, Filter: RecordFilter> SegmentProcessor<'p, Formatter, Filter> {
    pub fn new(
        parser: &'p ParserSettings,
        formatter: Formatter,
        filter: Filter,
        options: SegmentProcessorOptions,
    ) -> Self {
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

impl<'p, Formatter: AbstractRecordFormatter, Filter: RecordFilter> SegmentProcess
    for SegmentProcessor<'p, Formatter, Filter>
{
    fn process<'s, 'a, 'b, 'o, O>(
        &'s mut self,
        data: &'a [u8],
        buf: &'b mut Vec<u8>,
        prefix: &str,
        limit: Option<usize>,
        observer: &'o mut O,
    ) where
        O: RecordObserver,
    {
        let mut i = 0;
        let limit = limit.unwrap_or(usize::MAX);

        for line in self.delim.split(data) {
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

            let mut parser = self
                .parser
                .new_parser(crate::format::Auto::default(), &line[offset..])
                .unwrap();

            while let Some(Ok(record)) = parser.next() {
                i += 1;
                last_offset = record.span.end.clone();

                if parsed_some {
                    buf.push(b'\n');
                }
                parsed_some = true;

                if record.matches(&self.filter) {
                    let begin = buf.len();
                    buf.extend(prefix.as_bytes());
                    buf.extend(&line[offset..record.span.start]);
                    self.formatter
                        .format_record(buf, record.with_source(&line[record.span.clone()]));
                    let end = buf.len();
                    observer.observe_record(&record, begin..end);
                    produced_some = true;
                }
                if i >= limit {
                    break;
                }

                offset = record.span.end;

                parser.recycle(record);
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
