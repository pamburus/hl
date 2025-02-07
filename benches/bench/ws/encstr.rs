// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};
use serde_json::de::{Read, StrRead};

// local imports
use super::{hash, samples, ND};
use encstr::{json::JsonEncodedString, raw::RawString, AnyEncodedString, Builder, Handler, Ignorer};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "encstr");

fn bench(c: &mut Criterion) {
    for (input, batch) in [(samples::str::query01::JSON, BatchSize::SmallInput)] {
        bench_with(c, "json", input, Json, batch);
    }
    for (input, batch) in [(samples::str::query01::RAW, BatchSize::SmallInput)] {
        bench_with(c, "raw", input, Raw, batch);
    }
}

fn bench_with<I: InputConstruct>(
    c: &mut Criterion,
    title: &str,
    input: &'static str,
    constructor: I,
    batch: BatchSize,
) {
    let param = format!("{}:{}:{}", title, input.len(), hash(input));

    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Bytes(input.len() as u64));

    if title == "json" {
        group.bench_function(BenchmarkId::new("serde-json:parse-str", &param), |b| {
            let setup = || String::from(input);
            b.iter_batched_ref(
                setup,
                |input| {
                    let _: serde_json::Value = serde_json::from_str(input).unwrap();
                },
                batch,
            );
        });

        group.bench_function(BenchmarkId::new("serde-json:parse-str-raw", &param), |b| {
            let setup = || (Vec::with_capacity(4096), String::from(input));
            b.iter_batched_ref(
                setup,
                |(buf, input)| {
                    let mut reader = black_box(StrRead::new(&input[1..]));
                    reader.parse_str_raw(buf).unwrap();
                },
                batch,
            );
        });

        group.bench_function(BenchmarkId::new("serde-json:ignore-str", &param), |b| {
            let setup = || String::from(input);
            b.iter_batched_ref(
                setup,
                |input| {
                    let mut reader = black_box(StrRead::new(&input[1..]));
                    reader.ignore_str().unwrap()
                },
                batch,
            );
        });
    }

    group.bench_function(BenchmarkId::new("decode:ignore", &param), |b| {
        let mut target = Ignorer;
        let setup = || String::from(input);
        b.iter_batched_ref(
            setup,
            |input| {
                let input = black_box(constructor.new_input(input));
                input.decode(&mut target).unwrap()
            },
            batch,
        );
    });

    group.bench_function(BenchmarkId::new("decode:build", &param), |b| {
        let setup = || (Builder::with_capacity(4096), String::from(input));
        b.iter_batched_ref(
            setup,
            |(buf, input)| {
                let input = black_box(constructor.new_input(input));
                input.decode(buf).unwrap()
            },
            batch,
        );
    });

    group.bench_function(BenchmarkId::new("tokens:ignore", &param), |b| {
        let setup = || String::from(input);
        b.iter_batched_ref(
            setup,
            |input| {
                let input = black_box(constructor.new_input(input));
                for token in input.tokens() {
                    token.unwrap();
                }
            },
            batch,
        );
    });

    group.bench_function(BenchmarkId::new("tokens:build", &param), |b| {
        let setup = || (Builder::with_capacity(4096), String::from(input));
        b.iter_batched_ref(
            setup,
            |(buf, input)| {
                let input = black_box(constructor.new_input(input));
                for token in input.tokens() {
                    buf.handle(token.unwrap()).unwrap();
                }
            },
            batch,
        );
    });

    group.finish();
}

trait InputConstruct {
    type Output<'a>: AnyEncodedString<'a>;

    fn new_input<'a>(&self, input: &'a str) -> Self::Output<'a>;
}

struct Json;

impl InputConstruct for Json {
    type Output<'a> = JsonEncodedString<'a>;

    #[inline(always)]
    fn new_input<'a>(&self, input: &'a str) -> Self::Output<'a> {
        JsonEncodedString::new(input)
    }
}

struct Raw;

impl InputConstruct for Raw {
    type Output<'a> = RawString<'a>;

    #[inline(always)]
    fn new_input<'a>(&self, input: &'a str) -> Self::Output<'a> {
        RawString::new(input)
    }
}
