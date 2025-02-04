// std imports
use std::{iter::empty, sync::Arc};

// third-party imports
use chrono::{Offset, Utc};
use const_str::concat as strcat;
use criterion::{BenchmarkId, Criterion, Throughput};

// local imports
use super::{hash, samples, ND};
use hl::{
    app::{RecordIgnorer, SegmentProcess, SegmentProcessorOptions},
    settings,
    timezone::Tz,
    DateTimeFormatter, Filter, IncludeExcludeKeyFilter, LinuxDateFormat, Parser, ParserSettings, RecordFormatter,
    SegmentProcessor, Settings, Theme,
};

const GROUP: &str = strcat!(super::GROUP, ND, "combined");

const THEME: &str = "universal";
const SAMPLES: [(&str, &[u8]); 2] = [
    ("json", samples::log::elk01::JSON),
    ("logfmt", samples::log::elk01::LOGFMT),
];

pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);

    for (format, input) in SAMPLES {
        let param = format!("{}:{}:{}", format, input.len(), hash(input));

        c.throughput(Throughput::Bytes(input.len() as u64));

        let settings = Settings::default();
        let parser = Parser::new(ParserSettings::new(&settings.fields.predefined, empty(), None));
        let filter = Filter::default();
        let formatter = RecordFormatter::new(
            Arc::new(Theme::embedded(THEME).unwrap()),
            DateTimeFormatter::new(
                LinuxDateFormat::new("%b %d %T.%3N").compile(),
                Tz::FixedOffset(Utc.fix()),
            ),
            false,
            Arc::new(IncludeExcludeKeyFilter::default()),
            settings::Formatting::default(),
        );

        c.bench_function(BenchmarkId::new("parse-and-format", param), |b| {
            let setup = || {
                let processor = SegmentProcessor::new(&parser, &formatter, &filter, SegmentProcessorOptions::default());
                let buf = Vec::new();
                (processor, buf)
            };

            b.iter_with_setup(setup, |(mut processor, mut buf)| {
                processor.process(input, &mut buf, "", None, &mut RecordIgnorer {});
            });
        });
    }
}
