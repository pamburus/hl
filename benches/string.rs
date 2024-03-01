// std imports
use std::alloc::System;

// third-party imports
use criterion::{criterion_group, criterion_main, Criterion};
use stats_alloc::{StatsAlloc, INSTRUMENTED_SYSTEM};
use std::hint::black_box;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("string");
    let prefix = String::from("_");

    c.bench_function("string-short-match", |b| {
        let what = String::from("_TEST");
        b.iter(|| {
            assert_eq!(black_box(&what).starts_with(black_box(&prefix)), true);
        });
    });
    c.bench_function("string-long-match", |b| {
        let what = String::from("_TEST_SOME_VERY_VERY_LONG_NAME");
        b.iter(|| {
            assert_eq!(black_box(&what).starts_with(black_box(&prefix)), true);
        });
    });
    c.bench_function("string-short-non-match", |b| {
        let what = String::from("TEST");
        b.iter(|| {
            assert_eq!(black_box(&what).starts_with(black_box(&prefix)), false);
        });
    });
    c.bench_function("string-long-non-match", |b| {
        let what = String::from("TEST_SOME_VERY_VERY_LONG_NAME");
        b.iter(|| {
            assert_eq!(black_box(&what).starts_with(black_box(&prefix)), false);
        });
    });
    c.bench_function("string-long-prefix-match", |b| {
        let prefix = String::from("TEST_SOME_VERY_VERY_LONG_PREFIX_");
        let what = String::from("TEST_SOME_VERY_VERY_LONG_PREFIX_AND_SOMEWHAT");
        b.iter(|| {
            assert_eq!(black_box(&what).starts_with(black_box(&prefix)), true);
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
