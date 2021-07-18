// std imports
use std::alloc::System;

// third-party imports
use criterion::{criterion_group, criterion_main, Criterion};
use regex::Regex;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("regex");

    let re = Regex::new(r"^_").unwrap();

    let mut c1 = None;
    let mut n1 = 0;
    c.bench_function("regex-short-match", |b| {
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            assert_eq!(re.is_match("_TEST"), true);
            n1 += 1;
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);

    let mut c2 = None;
    let mut n2 = 0;
    c.bench_function("regex-long-match", |b| {
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            assert_eq!(re.is_match("_TEST_SOME_VERY_VERY_LONG_NAME"), true);
            n2 += 1;
        });
        c2 = Some(reg.change());
    });
    println!("allocations at 2 ({:?} iterations): {:#?}", n2, c2);

    c.bench_function("regex-short-non-match", |b| {
        b.iter(|| {
            assert_eq!(re.is_match("TEST"), false);
        });
    });
    c.bench_function("regex-long-non-match", |b| {
        b.iter(|| {
            assert_eq!(re.is_match("TEST_SOME_VERY_VERY_LONG_NAME"), false);
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
