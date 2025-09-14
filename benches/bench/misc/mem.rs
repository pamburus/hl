// std imports
use std::time::Duration;

// third-party imports
use const_str::concat as strcat;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group};
use memchr::{memchr, memchr2, memchr3};
use rand::random;

// local imports
use super::{BencherExt, ND};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "mem");

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    let seq = || {
        move || {
            let x: u8 = random();
            x
        }
    };

    let bufs = |size| {
        let next = seq();
        let vi: Vec<u8> = (0..size).map(|_| next()).collect();
        let ve: Vec<u8> = Vec::with_capacity(size);
        (vi, ve)
    };

    let variants = [
        (8, BatchSize::NumIterations(8192)),
        (512, BatchSize::NumIterations(8192)),
        (4096, BatchSize::NumIterations(8192)),
    ];

    for (n, batch) in variants {
        group.throughput(Throughput::Bytes(n as u64));

        group.bench_function(BenchmarkId::new("rotate:1", n), |b| {
            let setup = || bufs(n).0;
            b.iter_batched_ref_fixed(setup, |vi| vi.rotate_right(1), batch);
        });

        group.bench_function(BenchmarkId::new("copy", n), |b| {
            let setup = || bufs(n);
            b.iter_batched_ref_fixed(setup, |(vi, ve)| ve.extend_from_slice(vi.as_slice()), batch);
        });
    }

    let variants = [(4096, BatchSize::NumIterations(8192))];

    for (n, batch) in variants {
        group.throughput(Throughput::Bytes(n as u64));

        let setup = || (0..n).map(|x| (x * 256 / n) as u8).collect::<Vec<u8>>();
        let param = |x| format!("{}:{}", n, x);

        group.bench_function(BenchmarkId::new("position", param("single-value")), |b| {
            let needle = 128;
            b.iter_batched_ref_fixed(setup, |vi| vi.iter().position(|&x| x == needle), batch);
        });

        group.bench_function(BenchmarkId::new("position", param("one-of-two-values")), |b| {
            b.iter_batched_ref_fixed(setup, |vi| vi.iter().position(|&x| matches!(x, 128 | 192)), batch);
        });

        group.bench_function(BenchmarkId::new("position", param("one-of-three-values")), |b| {
            b.iter_batched_ref_fixed(setup, |vi| vi.iter().position(|&x| matches!(x, 128 | 192 | 224)), batch);
        });

        group.bench_function(BenchmarkId::new("memchr", param("single-value")), |b| {
            b.iter_batched_ref_fixed(setup, |vi| memchr(128, vi), batch);
        });

        group.bench_function(BenchmarkId::new("memchr", param("one-of-two-values")), |b| {
            b.iter_batched_ref_fixed(setup, |vi| memchr2(128, 192, vi), batch);
        });

        group.bench_function(BenchmarkId::new("memchr", param("one-of-three-values")), |b| {
            b.iter_batched_ref_fixed(setup, |vi| memchr3(128, 192, 224, vi), batch);
        });
    }

    group.finish();
}
