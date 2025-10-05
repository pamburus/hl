// std imports
use std::sync::Arc;

// third-party imports
use chrono::{Offset, Utc};
use const_str::concat as strcat;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

// local imports
use super::{BencherExt, ND, hash, samples};
use hl::{
    DateTimeFormatter, DateTimeFormatter, Filter, Filter, IncludeExcludeKeyFilter, IncludeExcludeKeyFilter,
    LinuxDateFormat, LinuxDateFormat, Parser, ParserSettings, ParserSettings, RecordFormatter, RecordFormatter,
    SegmentProcessor, Settings, Settings, Theme,
    app::{RecordIgnorer, SegmentProcess, SegmentProcessorOptions},
    formatting::{NoOpRecordWithSourceFormatter, RecordFormatterBuilder},
    processing::{RecordIgnorer, SegmentProcess, SegmentProcessor, SegmentProcessorOptions},
    settings,
    timezone::Tz,
};

const GROUP: &str = strcat!(super::GROUP, ND, "combined");

const THEME: &str = "universal";
const SAMPLES: [(&str, &[u8]); 4] = [
    ("json", samples::log::elk01::JSON),
    ("logfmt", samples::log::elk01::LOGFMT),
    ("json", samples::log::int01::JSON),
    ("logfmt", samples::log::int01::LOGFMT),
];

pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);

    for (format, input) in SAMPLES {
        let param = format!("{}:{}:{}", format, input.len(), hash(input));

        c.throughput(Throughput::Bytes(input.len() as u64));

        let settings = Settings::default();
        let parser = ParserSettings::new(&settings.fields.predefined);
        let filter = Filter::default();
        let formatter = RecordFormatterBuilder::new()
            .with_theme(Arc::new(Theme::embedded(THEME).unwrap()))
            .with_timestamp_formatter(DateTimeFormatter::new(
                LinuxDateFormat::new("%b %d %T.%3N").compile(),
                Tz::FixedOffset(Utc.fix()),
            ))
            .with_options(settings::Formatting::default())
            .build();

        c.bench_function(BenchmarkId::new("parse-and-format", &param), |b| {
            let mut processor = SegmentProcessor::new(&parser, &formatter, &filter, SegmentProcessorOptions::default());
            let setup = || Vec::with_capacity(4096);

            b.iter_batched_ref_fixed(
                setup,
                |buf| {
                    processor.process(input, buf, "", None, &mut RecordIgnorer {});
                },
                BatchSize::SmallInput,
            );
        });

        c.bench_function(BenchmarkId::new("parse-only", &param), |b| {
            let formatter = NoOpRecordWithSourceFormatter;
            let mut processor = SegmentProcessor::new(&parser, formatter, &filter, SegmentProcessorOptions::default());
            let setup = || Vec::with_capacity(4096);

            b.iter_batched_ref_fixed(
                setup,
                |buf| {
                    processor.process(input, buf, "", None, &mut RecordIgnorer {});
                },
                BatchSize::SmallInput,
            );
        });
    }
}
