// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use const_str::concat as strcat;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group};
use logos::Logos;

// workspace imports
use json_ast::{container::Container, token::Token};

// local imports
use super::{ND, hash, samples};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "json-ast");

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
                let mut container = Container::new();
                container.reserve(512);
                (container, String::from(std::str::from_utf8(sample).unwrap()))
            };

            b.iter_batched(
                setup,
                |(mut container, sample)| {
                    let mut lexer = Token::lexer(&sample);
                    black_box(container.extend(&mut lexer)).unwrap()
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
                    while black_box(lexer.next()).is_some() {}
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
