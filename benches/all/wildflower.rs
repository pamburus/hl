// third-party imports
use criterion::{criterion_group, Criterion};
use stats_alloc::Region;
use std::hint::black_box;
use wildflower::Pattern;

// local imports
use super::GA;

criterion_group!(benches, benchmark);

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("wildflower");
    let pattern = Pattern::new(r"_*");

    let mut c1 = None;
    let mut n1 = 0;
    c.bench_function("wildflower-short-match", |b| {
        let what = "_TEST";
        let reg = Region::new(&GA);
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), true);
            n1 += 1;
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);

    let mut c2 = None;
    let mut n2 = 0;
    c.bench_function("wildflower-long-match", |b| {
        let what = "_TEST_SOME_VERY_VERY_LONG_NAME";
        let reg = Region::new(&GA);
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), true);
            n2 += 1;
        });
        c2 = Some(reg.change());
    });
    println!("allocations at 2 ({:?} iterations): {:#?}", n2, c2);

    let mut c2 = None;
    let mut n2 = 0;
    c.bench_function("wildflower-long-prefix-match", |b| {
        let pattern = Pattern::new(r"SOME_VERY_VERY_LONG_PREFIX_*");
        let what = "SOME_VERY_VERY_LONG_PREFIX_AND_SOMEWHAT";
        let reg = Region::new(&GA);
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), true);
            n2 += 1;
        });
        c2 = Some(reg.change());
    });
    println!("allocations at 2 ({:?} iterations): {:#?}", n2, c2);

    c.bench_function("wildflower-short-non-match", |b| {
        let what = "TEST";
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), false);
        });
    });
    c.bench_function("wildflower-long-non-match", |b| {
        let what = "TEST_SOME_VERY_VERY_LONG_NAME";
        b.iter(|| {
            assert_eq!(black_box(&pattern).matches(black_box(&what)), false);
        });
    });
}
