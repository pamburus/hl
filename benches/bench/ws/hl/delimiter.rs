// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use const_str::concat as strcat;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

// local imports
use super::{BencherExt, ND, hash, samples};
use hl::{Delimit, Delimiter, SearchExt};

const GROUP: &str = strcat!(super::GROUP, ND, "delimiter");

pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);
    c.warm_up_time(Duration::from_secs(2));
    c.measurement_time(Duration::from_secs(3));

    let variants = [
        (
            "s1:byte:e",
            Vec::from(samples::log::elk01::JSON),
            Delimiter::Byte(b'\n'),
            BatchSize::NumIterations(8192),
        ),
        (
            "s1:byte:s",
            rotated(samples::log::elk01::JSON, 1),
            Delimiter::Byte(b'\n'),
            BatchSize::NumIterations(8192),
        ),
        (
            "s1:new-line:lf:e",
            Vec::from(samples::log::elk01::JSON),
            Delimiter::NewLine,
            BatchSize::NumIterations(8192),
        ),
        (
            "s1:new-line:lf:s",
            rotated(samples::log::elk01::JSON, 1),
            Delimiter::NewLine,
            BatchSize::NumIterations(8192),
        ),
    ];

    for edge in [false, true] {
        for (title, input, delim, batch) in &variants {
            let param = format!(
                "{}:{}:{}:{}",
                title,
                input.len(),
                hash(input),
                if edge { "edge" } else { "center" }
            );

            let bytes = Throughput::Bytes(
                delim
                    .clone()
                    .into_searcher()
                    .search_l(input, edge)
                    .map(|x| x.end as u64)
                    .unwrap_or(0),
            );
            c.throughput(bytes)
                .bench_function(BenchmarkId::new("search-l", &param), |b| {
                    let setup = || (input.clone(), delim.clone().into_searcher());
                    b.iter_batched_ref_fixed(setup, |(input, searcher)| searcher.search_l(input, edge), *batch)
                });

            let bytes = Throughput::Bytes(input.len() as u64);
            c.throughput(bytes)
                .bench_function(BenchmarkId::new("split", &param), |b| {
                    let setup = || (input.clone(), delim.clone().into_searcher());
                    b.iter_batched_ref_fixed(
                        setup,
                        |(input, searcher)| {
                            for x in searcher.split(input) {
                                black_box(x);
                            }
                        },
                        *batch,
                    )
                });

            let bytes = Throughput::Bytes(
                delim
                    .clone()
                    .into_searcher()
                    .search_r(input, edge)
                    .map(|x| (input.len() - x.start) as u64)
                    .unwrap_or(0),
            );
            c.throughput(bytes)
                .bench_function(BenchmarkId::new("search-r", &param), |b| {
                    let setup = || (input.clone(), delim.clone().into_searcher());
                    b.iter_batched_ref_fixed(setup, |(input, searcher)| searcher.search_r(input, edge), *batch)
                });
        }
    }
}

fn rotated(data: &[u8], n: usize) -> Vec<u8> {
    let mut v = Vec::from(data);
    v.rotate_right(n);
    v
}
