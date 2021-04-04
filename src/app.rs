// std imports
use std::sync::Arc;

// third-party imports
use async_std::{
    io::{Read, Write},
    stream::Stream,
};
use chrono::FixedOffset;
use closure::closure;
use crossbeam_channel::{self as channel, RecvError};
use crossbeam_utils::thread;
use futures::{future::ready, stream::StreamExt};
use futures_util::pin_mut;
use itertools::izip;
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

    pub async fn run(&self, inputs: impl Stream<Item = Input>, output: &impl Write) -> Result<()> {
        let n = self.options.concurrency;
        let sbf = Arc::new(SegmentBufFactory::new(self.options.buffer_size));
        let bfo = BufFactory::new(self.options.buffer_size);

        pin_mut!(inputs);
        while let Some(input) = inputs.next().await {
            let segments = Chopper::new(sbf, "\n".to_string())
                .chop_jumbo(input.stream, self.options.max_message_size)
                .map(|x| ready(x))
                .buffered(self.options.concurrency);
            pin_mut!(segments);
        }

        thread::scope(|scope| -> Result<()> {
            // prepare receive/transmit channels for input data
            let (txi, rxi): (Vec<_>, Vec<_>) = (0..n).map(|_| channel::bounded(1)).unzip();
            // prepare receive/transmit channels for output data
            let (txo, rxo): (Vec<_>, Vec<_>) = (0..n)
                .into_iter()
                .map(|_| channel::bounded::<Vec<u8>>(1))
                .unzip();
            // spawn reader thread
            let reader = scope.spawn(closure!(clone sfi, |_| -> Result<()> {
                let mut sn: usize = 0;
                let scanner = Scanner::new(sfi, "\n".to_string());
                for item in scanner.items(input).with_max_segment_size(self.options.max_message_size) {
                    if let Err(_) = txi[sn % n].send(item?) {
                        break;
                    }
                    sn += 1;
                }
                Ok(())
            }));
            // spawn processing threads
            for (rxi, txo) in izip!(rxi, txo) {
                scope.spawn(closure!(ref bfo, ref sfi, |_| {
                    let formatter = RecordFormatter::new(
                        self.options.theme.clone(),
                        DateTimeFormatter::new(
                            self.options.time_format.clone(),
                            self.options.time_zone,
                        ),
                        self.options.hide_empty_fields,
                        self.options.fields.clone(),
                    )
                    .with_field_unescaping(!self.options.raw_fields);
                    for segment in rxi.iter() {
                        match segment {
                            Segment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                self.process_segement(&segment, &formatter, &mut buf);
                                sfi.recycle(segment);
                                if let Err(_) = txo.send(buf) {
                                    break;
                                };
                            }
                            Segment::Incomplete(segment, _) => {
                                if let Err(_) = txo.send(segment.to_vec()) {
                                    break;
                                }
                            }
                        }
                    }
                }));
            }
            // spawn writer thread
            let writer = scope.spawn(closure!(ref bfo, |_| -> Result<()> {
                let mut sn = 0;
                loop {
                    match rxo[sn % n].recv() {
                        Ok(buf) => {
                            output.write_all(&buf[..])?;
                            bfo.recycle(buf);
                        }
                        Err(RecvError) => {
                            break;
                        }
                    }
                    sn += 1;
                }
                Ok(())
            }));
            // collect errors from reader and writer threads
            reader.join().unwrap()?;
            writer.join().unwrap()?;
            Ok(())
        })
        .unwrap()?;

        return Ok(());
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
