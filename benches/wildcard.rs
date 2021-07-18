// std imports
use std::alloc::System;

// third-party imports
use criterion::{criterion_group, criterion_main, Criterion};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use wildmatch::WildMatch;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("wildcard");
    let pattern = WildMatch::new(r"_*");
    let prefix = String::from("_");

    let mut c1 = None;
    let mut n1 = 0;
    c.bench_function("wild-short-match", |b| {
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            assert_eq!(pattern.matches("_TEST"), true);
            n1 += 1;
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);

    let mut c2 = None;
    let mut n2 = 0;
    c.bench_function("wild-long-match", |b| {
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            assert_eq!(pattern.matches("_TEST_SOME_VERY_VERY_LONG_NAME"), true);
            n2 += 1;
        });
        c2 = Some(reg.change());
    });
    println!("allocations at 2 ({:?} iterations): {:#?}", n2, c2);

    c.bench_function("wild-short-non-match", |b| {
        b.iter(|| {
            assert_eq!(pattern.matches("TEST"), false);
        });
    });
    c.bench_function("wild-long-non-match", |b| {
        b.iter(|| {
            assert_eq!(pattern.matches("TEST_SOME_VERY_VERY_LONG_NAME"), false);
        });
    });
    c.bench_function("compare-short-match", |b| {
        let what = String::from("_TEST");
        b.iter(|| {
            assert_eq!(what.starts_with(&prefix), true);
        });
    });
    c.bench_function("compare-long-match", |b| {
        let what = String::from("_TEST_SOME_VERY_VERY_LONG_NAME");
        b.iter(|| {
            assert_eq!(what.starts_with(&prefix), true);
        });
    });
    c.bench_function("compare-short-non-match", |b| {
        let what = String::from("TEST");
        b.iter(|| {
            assert_eq!(what.starts_with(&prefix), false);
        });
    });
    c.bench_function("compare-long-non-match", |b| {
        let what = String::from("TEST_SOME_VERY_VERY_LONG_NAME");
        b.iter(|| {
            assert_eq!(what.starts_with(&prefix), false);
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
