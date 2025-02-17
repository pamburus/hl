// std imports
use std::{hint::black_box, sync::Arc, time::Duration, vec::Vec};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};
use logos::Logos;

// workspace imports
use log_ast::{
    ast::Container,
    model::{FormatExt, Segment},
};
use log_format::{ast::Discarder, Format};
use log_format_json::{JsonFormat, Lexer, Token};

// local imports
use super::{hash, samples, ND};
use crate::utf8;

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "log-format-json");

const SAMPLES: &[&[u8]] = &[samples::log::elk01::JSON];

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
                |sample| JsonFormat.parse(&sample, Discarder::new()).unwrap(),
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:ast", &param), |b| {
            let setup = || (Container::with_capacity(160), Vec::from(sample));

            b.iter_batched_ref(
                setup,
                |(container, sample)| {
                    JsonFormat
                        .parse(&sample, container.metaroot())
                        .map_err(|x| x.0)
                        .unwrap()
                        .0
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse:ast:segment", &param), |b| {
            let setup = || (Segment::with_capacity(160), Arc::<str>::from(utf8!(sample)), JsonFormat);

            b.iter_batched_ref(
                setup,
                |(segment, sample, format)| format.parse_into(sample.clone(), segment).1.unwrap(),
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
