// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};
use serde_json::de::{Read, StrRead};

// local imports
use super::{hash, samples};
use encstr::{json::JsonEncodedString, AnyEncodedString, Builder, Handler, Ignorer};

criterion_group!(benches, bench);

const GROUP: &str = "encstr";

fn bench(c: &mut Criterion) {
    for input in [samples::str::query01::JSON] {
        bench_with(c, "json", JsonEncodedString::new(input));
    }
    for input in [samples::str::query01::RAW] {
        bench_with(c, "raw", JsonEncodedString::new(input));
    }
}

fn bench_with<'a, I: AnyEncodedString<'a>>(c: &mut Criterion, title: &str, input: I) {
    let param = format!("{}:{}:{}", title, input.source().len(), hash(input.source()));

    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));
    group.throughput(Throughput::Bytes(input.source().len() as u64));

    group.bench_function(BenchmarkId::new("serde:parse_str_raw", &param), |b| {
        let setup = || (Vec::with_capacity(4096), StrRead::new(black_box(&input.source()[1..])));
        b.iter_with_setup(setup, |(mut buf, mut reader)| {
            black_box(reader.parse_str_raw(black_box(&mut buf)).unwrap());
        });
    });

    group.bench_function(BenchmarkId::new("serde:ignore_str", &param), |b| {
        let setup = || StrRead::new(black_box(&input.source()[1..]));
        b.iter_with_setup(setup, |mut reader| {
            black_box(reader.ignore_str().unwrap());
        });
    });

    group.bench_function(BenchmarkId::new("decode:ignore", &param), |b| {
        let mut result = Ignorer;
        b.iter(|| {
            black_box(&input).decode(&mut result).unwrap();
        });
    });

    group.bench_function(BenchmarkId::new("decode:build", &param), |b| {
        let setup = || Builder::with_capacity(4096);
        b.iter_with_setup(setup, |mut buf| {
            black_box(&input).decode(&mut buf).unwrap();
        });
    });

    group.bench_function(BenchmarkId::new("tokens:ignore", &param), |b| {
        b.iter(|| {
            for token in black_box(&input).tokens() {
                token.unwrap();
            }
        });
    });

    group.bench_function(BenchmarkId::new("tokens:build", &param), |b| {
        let setup = || Builder::with_capacity(4096);
        b.iter_with_setup(setup, |mut buf| {
            for token in black_box(&input).tokens() {
                buf.handle(token.unwrap()).unwrap();
            }
        });
    });

    group.finish();
}
