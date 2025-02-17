// std imports
use std::{hint::black_box, sync::Arc, time::Duration, vec::Vec};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};

// workspace imports
use log_ast::{
    ast::Container,
    model::{FormatExt, Segment},
};
use log_format::{ast::Discarder, Format};
use log_format_auto::AutoFormat;

// local imports
use super::{hash, samples, ND};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "log-format-auto");

const SAMPLES: &[(&str, &[u8])] = &[
    ("json", samples::log::elk01::JSON),
    ("logfmt", samples::log::elk01::LOGFMT),
];

pub(super) fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    for &(name, sample) in SAMPLES {
        let param = format!("{}:{}:{}", name, sample.len(), hash(sample));

        group.throughput(Throughput::Bytes(sample.len() as u64));

        group.bench_function(BenchmarkId::new("lex:drain", &param), |b| {
            let setup = || (AutoFormat::default(), Vec::from(sample));

            b.iter_batched_ref(
                setup,
                |(format, sample)| {
                    let mut lexer = format.lexer(sample);
                    while let Some(token) = black_box(lexer.next()) {
                        token.unwrap();
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:discard", &param), |b| {
            let setup = || (Vec::from(sample), AutoFormat::default());

            b.iter_batched_ref(
                setup,
                |(sample, format)| format.parse(&sample, Discarder::new()).unwrap(),
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:ast", &param), |b| {
            let setup = || (Container::with_capacity(160), Vec::from(sample), AutoFormat::default());

            b.iter_batched_ref(
                setup,
                |(container, sample, format)| format.parse(&sample, container.metaroot()).map_err(|x| x.0).unwrap().0,
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:ast:segment", &param), |b| {
            let setup = || {
                (
                    Segment::with_capacity(160),
                    Arc::<[u8]>::from(Vec::from(sample)),
                    AutoFormat::default(),
                )
            };

            b.iter_batched_ref(
                setup,
                |(segment, sample, format)| format.parse_into(sample.clone(), segment).1.unwrap(),
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
