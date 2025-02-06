// std imports
use std::time::Duration;

// third-party imports
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};
use serde_json::de::{Read, StrRead};

// local imports
use super::{hash, samples};
use encstr::{json::JsonEncodedString, raw::RawString, AnyEncodedString, Builder, Handler, Ignorer};

criterion_group!(benches, bench);

const GROUP: &str = "encstr";

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
    batch_size: BatchSize,
) {
    let param = format!("{}:{}:{}", title, input.len(), hash(input));

    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));
    group.throughput(Throughput::Bytes(input.len() as u64));

    if title == "json" {
        group.bench_function(BenchmarkId::new("serde-json:parse-str", &param), |b| {
            let setup = || String::from(input);
            let perform = |input: String| {
                let _: serde_json::Value = serde_json::from_str(&input).unwrap();
            };
            b.iter_batched(setup, perform, batch_size);
        });

        group.bench_function(BenchmarkId::new("serde-json:parse-str-raw", &param), |b| {
            let setup = || (Vec::with_capacity(4096), String::from(input));
            let perform = |(mut buf, input): (Vec<u8>, String)| {
                let mut reader = StrRead::new(&input[1..]);
                reader.parse_str_raw(&mut buf).unwrap();
            };
            b.iter_batched(setup, perform, batch_size);
        });

        group.bench_function(BenchmarkId::new("serde-json:ignore-str", &param), |b| {
            let setup = || String::from(input);
            let perform = |input: String| {
                let mut reader = StrRead::new(&input[1..]);
                reader.ignore_str().unwrap()
            };
            b.iter_batched(setup, perform, batch_size);
        });
    }

    group.bench_function(BenchmarkId::new("decode:ignore", &param), |b| {
        let mut target = Ignorer;
        let setup = || String::from(input);
        let perform = |input: String| {
            let input = constructor.new_input(&input);
            input.decode(&mut target).unwrap()
        };
        b.iter_batched(setup, perform, batch_size);
    });

    group.bench_function(BenchmarkId::new("decode:build", &param), |b| {
        let setup = || (Builder::with_capacity(4096), String::from(input));
        let perform = |(mut buf, input): (Builder, String)| {
            let input = constructor.new_input(&input);
            input.decode(&mut buf).unwrap()
        };
        b.iter_batched(setup, perform, batch_size);
    });

    group.bench_function(BenchmarkId::new("tokens:ignore", &param), |b| {
        let setup = || String::from(input);
        let perform = |input: String| {
            let input = constructor.new_input(&input);
            for token in input.tokens() {
                token.unwrap();
            }
        };
        b.iter_batched(setup, perform, batch_size);
    });

    group.bench_function(BenchmarkId::new("tokens:build", &param), |b| {
        let setup = || (Builder::with_capacity(4096), String::from(input));
        let perform = |(mut buf, input): (Builder, String)| {
            let input = constructor.new_input(&input);
            for token in input.tokens() {
                buf.handle(token.unwrap()).unwrap();
            }
        };
        b.iter_batched(setup, perform, batch_size);
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
