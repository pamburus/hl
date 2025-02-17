// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};
use logos::Logos;

// workspace imports
use super::{hash, samples, ND};
use hl::{
    format::json::{parse_all_into, Token},
    model::v2::ast,
};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "flat-tree");

const SAMPLES: &[&[u8]] = &[samples::log::elk01::JSON];

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    for sample in SAMPLES {
        let param = format!("json:{}:{}", sample.len(), hash(sample));

        group.throughput(Throughput::Bytes(sample.len() as u64));

        group.bench_function(BenchmarkId::new("parse-all-into:flat-tree", &param), |b| {
            let setup = || {
                let mut container = ast::Container::new();
                container.reserve(512);
                (container, String::from(std::str::from_utf8(sample).unwrap()))
            };

            b.iter_batched(
                setup,
                |(mut container, sample)| {
                    let mut lexer = Token::lexer(&sample);
                    black_box(parse_all_into(&mut lexer, container.metaroot())).1.is_ok()
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("parse-all-into:drain", &param), |b| {
            let setup = || String::from(std::str::from_utf8(sample).unwrap());

            b.iter_batched_ref(
                setup,
                |sample| {
                    let mut lexer = Token::lexer(sample);
                    while let Some(_) = black_box(lexer.next()) {}
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
