// std imports
use std::alloc::System;

// third-party imports
use criterion::{criterion_group, criterion_main, Criterion};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::hint::black_box;
use wildmatch::WildMatch;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("wildmatch");
    let pattern = WildMatch::new(r"_*");

    let mut c1 = None;
    let mut n1 = 0;
    c.bench_function("wildmatch-short-match", |b| {
        let what = "_TEST";
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), true);
            n1 += 1;
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);

    let mut c2 = None;
    let mut n2 = 0;
    c.bench_function("wildmatch-long-match", |b| {
        let what = "_TEST_SOME_VERY_VERY_LONG_NAME";
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), true);
            n2 += 1;
        });
        c2 = Some(reg.change());
    });
    println!("allocations at 2 ({:?} iterations): {:#?}", n2, c2);

    let mut c2 = None;
    let mut n2 = 0;
    c.bench_function("wildmatch-long-prefix-match", |b| {
        let pattern = WildMatch::new(r"SOME_VERY_VERY_LONG_PREFIX_*");
        let what = "SOME_VERY_VERY_LONG_PREFIX_AND_SOMEWHAT";
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), true);
            n2 += 1;
        });
        c2 = Some(reg.change());
    });
    println!("allocations at 2 ({:?} iterations): {:#?}", n2, c2);

    c.bench_function("wildmatch-short-non-match", |b| {
        let what = "TEST";
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), false);
        });
    });
    c.bench_function("wildmatch-long-non-match", |b| {
        let what = "TEST_SOME_VERY_VERY_LONG_NAME";
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), false);
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
