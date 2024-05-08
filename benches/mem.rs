// std imports
use std::alloc::System;

// third-party imports
use criterion::{criterion_group, criterion_main, Criterion};
use stats_alloc::{StatsAlloc, INSTRUMENTED_SYSTEM};
use std::hint::black_box;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("mem");

    let bufs = |size| {
        let vi: Vec<u8> = (0..size).into_iter().map(|x| x as u8).collect();
        let ve: Vec<u8> = Vec::with_capacity(size);
        (vi, ve)
    };

    for n in [512, 4096] {
        c.bench_function(format!("mem-rotate-{}", n), |b| {
            let (mut vi, _) = bufs(n);
            b.iter(|| {
                black_box(&mut vi).rotate_right(1);
            });
        });
        c.bench_function(format!("mem-copy-{}", n), |b| {
            let (vi, mut ve) = bufs(n);
            b.iter(|| {
                ve.clear();
                black_box(&mut ve).extend_from_slice(black_box(&vi).as_slice());
            });
        });
    }

    c.bench_function("mem-find-single-value-4096", |b| {
        let vi: Vec<u8> = (0..4096).into_iter().map(|x| (x / 16) as u8).collect();
        b.iter(|| {
            black_box(vi.iter().position(|&x| x == 128));
        });
    });

    c.bench_function("mem-find-one-of-two-values-4096", |b| {
        let vi: Vec<u8> = (0..4096).into_iter().map(|x| (x / 16) as u8).collect();
        b.iter(|| {
            black_box(vi.iter().position(|&x| matches!(x, 128 | 192)));
        });
    });

    c.bench_function("mem-find-one-of-four-values-4096", |b| {
        let vi: Vec<u8> = (0..4096).into_iter().map(|x| (x / 16) as u8).collect();
        b.iter(|| {
            black_box(vi.iter().position(|&x| matches!(x, 128 | 192 | 224 | 240)));
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
