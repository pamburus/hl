// std imports
use std::sync::Arc;

// third-party imports
use async_std::{io::Write, stream::Stream};
use chrono::FixedOffset;
use futures::{future::ready, io::AsyncWriteExt, stream::StreamExt};
use futures_util::pin_mut;
use serde_json as json;

// local imports
use crate::{
    chop::{BufFactory, Chopper, Segment, SegmentBuf, SegmentBufFactory},
    datefmt::{DateTimeFormat, DateTimeFormatter},
    error::*,
    formatting::RecordFormatter,
    input::Input,
    model::{Filter, Record},
    theme::Theme,
    IncludeExcludeKeyFilter,
};

pub struct Options {
    pub theme: Arc<Theme>,
    pub time_format: DateTimeFormat,
    pub raw_fields: bool,
    pub buffer_size: usize,
    pub max_message_size: usize,
    pub concurrency: usize,
    pub filter: Filter,
    pub fields: Arc<IncludeExcludeKeyFilter>,
    pub time_zone: FixedOffset,
    pub hide_empty_fields: bool,
}

pub struct App {
    options: Options,
}

impl App {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    pub async fn run(&self, inputs: impl Stream<Item = Input>, output: impl Write) -> Result<()> {
        let n = self.options.concurrency;
        let sbf = Arc::new(SegmentBufFactory::new(self.options.buffer_size));
        let bfo = BufFactory::new(self.options.buffer_size);
        let formatter = RecordFormatter::new(
            self.options.theme.clone(),
            DateTimeFormatter::new(self.options.time_format.clone(), self.options.time_zone),
            self.options.hide_empty_fields,
            self.options.fields.clone(),
        )
        .with_field_unescaping(!self.options.raw_fields);

        pin_mut!(inputs);
        pin_mut!(output);
        while let Some(input) = inputs.next().await {
            let segments = Chopper::new(sbf.clone(), "\n".to_string())
                .chop_jumbo(input.stream, self.options.max_message_size)
                .map(|x| ready(x))
                .buffered(n);

            let outbufs = segments.map(|segment| {
                segment.map(|segment| match segment {
                    Segment::Regular(segment) => {
                        let mut buf = bfo.new_buf();
                        self.process_segement(&segment, &formatter, &mut buf);
                        sbf.recycle(segment);
                        buf
                    }
                    Segment::Partial(segment, _) => segment.to_vec(),
                })
            });

            pin_mut!(outbufs);
            while let Some(buf) = outbufs.next().await {
                let buf = buf?;
                output.write(&buf[..]).await?;
                bfo.recycle(buf);
            }
        }

        Ok(())
    }

    fn process_segement(
        &self,
        segment: &SegmentBuf,
        formatter: &RecordFormatter,
        buf: &mut Vec<u8>,
    ) {
        for data in segment.data().split(|c| *c == b'\n') {
            let data = trim_right(data, |ch| ch == b'\r');
            if data.len() == 0 {
                continue;
            }
            let mut stream = json::Deserializer::from_slice(data).into_iter::<Record>();
            while let Some(Ok(record)) = stream.next() {
                if record.matches(&self.options.filter) {
                    formatter.format_record(buf, &record);
                }
            }
            let remainder = trim_right(&data[stream.byte_offset()..], |ch| match ch {
                b'\r' | b'\n' | b' ' | b'\t' => true,
                _ => false,
            });
            if remainder.len() > 0 {
                buf.extend_from_slice(remainder);
                buf.push(b'\n');
            }
        }
    }
}

fn trim_right<'a, F: Fn(u8) -> bool>(slice: &'a [u8], predicate: F) -> &'a [u8] {
    if let Some(pos) = slice.iter().rposition(|&ch| !predicate(ch)) {
        &slice[..pos + 1]
    } else {
        &slice[0..0]
    }
}
