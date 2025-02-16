// std imports
use std::{hint::black_box, sync::Arc, time::Duration, vec::Vec};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};

// workspace imports
use log_ast::ast::{Container, Segment};
use log_format::{ast::Discarder, Format};
use log_format_logfmt::{Lexer, LogfmtFormat, Token};

// local imports
use super::{hash, samples, ND};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "log-format-logfmt");

const SAMPLES: &[&[u8]] = &[samples::log::elk01::LOGFMT];

pub(super) fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    for &sample in SAMPLES {
        let param = format!("{}:{}", sample.len(), hash(sample));

        group.throughput(Throughput::Bytes(sample.len() as u64));

        group.bench_function(BenchmarkId::new("lex:inner:drain", &param), |b| {
            let setup = || Vec::from(sample);

            b.iter_batched_ref(
                setup,
                |sample| {
                    let mut lexer = Token::lexer(sample);
                    while let Some(_) = black_box(lexer.next()) {}
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("lex:log-format:drain", &param), |b| {
            let setup = || Vec::from(sample);

            b.iter_batched_ref(
                setup,
                |sample| {
                    let mut lexer = Lexer::from_slice(sample);
                    while let Some(_) = black_box(lexer.next()) {}
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:discard", &param), |b| {
            let setup = || Vec::from(sample);

            b.iter_batched_ref(
                setup,
                |sample| LogfmtFormat.parse(&sample, Discarder::new()).unwrap(),
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:ast", &param), |b| {
            let setup = || (Container::with_capacity(160), Vec::from(sample));

            b.iter_batched_ref(
                setup,
                |(container, sample)| {
                    LogfmtFormat
                        .parse(&sample, container.metaroot())
                        .map_err(|x| x.0)
                        .unwrap()
                        .0
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:ast:segment", &param), |b| {
            let setup = || {
                (
                    Segment::with_capacity(160).with_buf::<Arc<[u8]>>(),
                    Arc::<[u8]>::from(Vec::from(sample)),
                    LogfmtFormat,
                )
            };

            b.iter_batched_ref(
                setup,
                |(segment, sample, format)| segment.set(sample.clone(), format).unwrap(),
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
