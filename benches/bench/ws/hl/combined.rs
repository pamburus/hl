// std imports
use std::{iter::empty, sync::Arc};

// third-party imports
use chrono::{Offset, Utc};
use const_str::concat as strcat;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

// local imports
use super::{BencherExt, ND, hash, samples};
use hl::{
    DateTimeFormatter, Filter, LinuxDateFormat, Origin, Parser, ParserSettings, SegmentProcessor, Settings, Theme,
    app::{RecordIgnorer, SegmentProcess, SegmentProcessorOptions},
    formatting::{
        NoOpRecordWithSourceFormatter, RecordFormatterBuilder, RecordWithSourceFormatter,
        structured::{Encoder, JsonEncoder, LogfmtEncoder, StructuredRecordFormatterBuilder},
    },
    input::InputDescriptor,
    settings::{self, ExpansionMode},
    timezone::Tz,
};

const GROUP: &str = strcat!(super::GROUP, ND, "combined");

const THEME: &str = "universal";
const SAMPLES: [(&str, &[u8], ExpansionMode); 6] = [
    ("json", samples::log::elk01::JSON, ExpansionMode::Inline),
    ("logfmt", samples::log::elk01::LOGFMT, ExpansionMode::Inline),
    ("json", samples::log::elk01::JSON, ExpansionMode::Always),
    ("logfmt", samples::log::elk01::LOGFMT, ExpansionMode::Always),
    ("json", samples::log::int01::JSON, ExpansionMode::Inline),
    ("logfmt", samples::log::int01::LOGFMT, ExpansionMode::Inline),
];

pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);

    for (format, input, expansion) in SAMPLES {
        let mut param = format!("{}:{}:{}", format, input.len(), hash(input));
        if expansion != ExpansionMode::Inline {
            param = format!("{}:x={}", param, expansion);
        }

        c.throughput(Throughput::Bytes(input.len() as u64));

        let settings = Settings::default();
        let parser = Parser::new(ParserSettings::new(&settings.fields.predefined, empty(), None));
        let filter = Filter::default();
        let descriptor = InputDescriptor::new(0, "<stdin>");
        let formatter = RecordFormatterBuilder::new()
            .with_theme(Arc::new(Theme::embedded(THEME).unwrap()))
            .with_timestamp_formatter(DateTimeFormatter::new(
                LinuxDateFormat::new("%b %d %T.%3N").compile(),
                Tz::FixedOffset(Utc.fix()),
            ))
            .with_expansion(expansion.into())
            .with_options(settings::Formatting::default())
            .build();

        c.bench_function(BenchmarkId::new("parse-and-format", &param), |b| {
            let mut processor = SegmentProcessor::new(&parser, &formatter, &filter, SegmentProcessorOptions::default());
            let setup = || Vec::with_capacity(4096);

            b.iter_batched_ref_fixed(
                setup,
                |buf| {
                    processor.process(
                        input,
                        buf,
                        Origin::new(&descriptor, 0, input.len()),
                        "",
                        None,
                        &mut RecordIgnorer {},
                    );
                },
                BatchSize::SmallInput,
            );
        });

        if expansion == ExpansionMode::Inline {
            // Structured output formats have no expansion, and logfmt output is always flattened.
            let structured = |flatten| {
                StructuredRecordFormatterBuilder::new()
                    .with_flatten(flatten)
                    .with_predefined_fields(&settings.fields.predefined)
            };
            bench_structured::<JsonEncoder>(
                &mut c,
                "parse-and-format-json",
                &param,
                input,
                &parser,
                structured(false),
            );
            bench_structured::<LogfmtEncoder>(
                &mut c,
                "parse-and-format-logfmt",
                &param,
                input,
                &parser,
                structured(true),
            );
        }

        c.bench_function(BenchmarkId::new("parse-only", &param), |b| {
            let formatter = NoOpRecordWithSourceFormatter;
            let mut processor = SegmentProcessor::new(&parser, formatter, &filter, SegmentProcessorOptions::default());
            let setup = || Vec::with_capacity(4096);

            b.iter_batched_ref_fixed(
                setup,
                |buf| {
                    processor.process(
                        input,
                        buf,
                        Origin::new(&descriptor, 0, input.len()),
                        "",
                        None,
                        &mut RecordIgnorer {},
                    );
                },
                BatchSize::SmallInput,
            );
        });
    }
}

fn bench_structured<E: Encoder + 'static>(
    c: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    param: &str,
    input: &[u8],
    parser: &Parser,
    builder: StructuredRecordFormatterBuilder,
) where
    hl::formatting::structured::StructuredRecordFormatter<E>: RecordWithSourceFormatter,
{
    let filter = Filter::default();
    let descriptor = InputDescriptor::new(0, "<stdin>");
    let formatter = builder.build::<E>();
    let options = || SegmentProcessorOptions {
        records_only: true,
        ..Default::default()
    };

    c.bench_function(BenchmarkId::new(name, param), |b| {
        let mut processor = SegmentProcessor::new(parser, &formatter, &filter, options());
        let setup = || Vec::with_capacity(4096);

        b.iter_batched_ref_fixed(
            setup,
            |buf| {
                processor.process(
                    input,
                    buf,
                    Origin::new(&descriptor, 0, input.len()),
                    "",
                    None,
                    &mut RecordIgnorer {},
                );
            },
            BatchSize::SmallInput,
        );
    });
}
