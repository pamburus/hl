// std imports
use std::time::Duration;

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, Criterion};

// local imports
use super::{BencherExt, ND};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "fncall");

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("add42", |b| {
        let setup = || 1 as u64;
        b.iter_batched_ref_fixed(setup, |x| add42(x), BatchSize::NumIterations(65536));
    });

    group.bench_function("add42:inline", |b| {
        let setup = || 1 as u64;
        b.iter_batched_ref_fixed(setup, |x| add42_inline(x), BatchSize::NumIterations(65536));
    });

    group.finish();
}

#[inline(never)]
fn add42(x: &mut u64) {
    add42_inline(x)
}

#[inline(always)]
fn add42_inline(x: &mut u64) {
    *x = *x + 42;
}
