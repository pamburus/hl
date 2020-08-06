use std::io::{Read, Write};
use std::sync::Arc;

use closure::closure;
use crossbeam_channel as channel;
use crossbeam_channel::RecvError;
use crossbeam_utils::thread;
use itertools::izip;
use serde_json as json;

use crate::error::*;
use crate::formatting::MessageFormatter;
use crate::model::{Filter, Message};
use crate::scanning::{BufFactory, ScannedSegment, Scanner, Segment, SegmentFactory};
use crate::theme::Theme;

pub struct Options {
    pub theme: Arc<Theme>,
    pub time_format: String,
    pub raw_fields: bool,
    pub buffer_size: usize,
    pub concurrency: usize,
    pub filter: Filter,
}

pub struct App {
    options: Options,
}

impl App {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    pub fn run(
        &self,
        input: &mut (dyn Read + Send + Sync),
        output: &mut (dyn Write + Send + Sync),
    ) -> Result<()> {
        let n = self.options.concurrency;
        let sfi = Arc::new(SegmentFactory::new(self.options.buffer_size));
        let bfo = BufFactory::new(self.options.buffer_size);
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
                for item in scanner.items(input) {
                    if let Err(_) = txi[sn % n].send(item?) {
                        break;
                    }
                    sn += 1;
                }
                Ok(())
            }));
            // spawn writer thread
            let writer = scope.spawn(closure!(ref bfo, |_| -> Result<()> {
                let mut sn = 0;
                loop {
                    match rxo[sn % n].recv() {
                        Ok(buf) => {
                            output.write(&buf[..])?;
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
            // spawn processing threads
            for (rxi, txo) in izip!(rxi, txo) {
                scope.spawn(closure!(ref bfo, ref sfi, |_| {
                    let mut formatter = MessageFormatter::new(
                        self.options.theme.clone(),
                        &self.options.time_format,
                    )
                    .with_field_unescaping(!self.options.raw_fields);
                    for segment in rxi.iter() {
                        match segment {
                            ScannedSegment::Complete(segment) => {
                                let mut buf = bfo.new_buf();
                                self.process_segement(&segment, &mut formatter, &mut buf);
                                sfi.recycle(segment);
                                if let Err(_) = txo.send(buf) {
                                    break;
                                };
                            }
                            ScannedSegment::Incomplete(segment) => {
                                if let Err(_) = txo.send(segment.to_vec()) {
                                    break;
                                }
                            }
                        }
                    }
                }));
            }
            // collect errors from reader and writer threads
            reader.join().unwrap()?;
            writer.join().unwrap()?;
            Ok(())
        })
        .unwrap()?;

        return Ok(());
    }

    fn process_segement<'a>(
        &self,
        segment: &Segment,
        formatter: &mut MessageFormatter<'a>,
        buf: &mut Vec<u8>,
    ) {
        for data in segment.data().split(|c| *c == b'\n') {
            let data = strip(data, b'\r');
            if data.len() == 0 {
                continue;
            }
            match json::from_slice::<Message>(data) {
                Ok(msg) => {
                    if msg.matches(&self.options.filter) {
                        formatter.format_message(buf, &msg);
                    }
                }
                _ => {
                    buf.extend_from_slice(data);
                    buf.push(b'\n');
                }
            }
        }
    }
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
