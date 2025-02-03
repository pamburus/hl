// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};

criterion_group!(benches, bench);

const GROUP: &str = "mem";

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));

    let bufs = |size| {
        let vi: Vec<u8> = (0..size).into_iter().map(|x| x as u8).collect();
        let ve: Vec<u8> = Vec::with_capacity(size);
        (vi, ve)
    };

    for n in [512, 4096] {
        group.throughput(Throughput::Bytes(n as u64));

        group.bench_function(BenchmarkId::new("rotate", n), |b| {
            let (mut vi, _) = bufs(n);
            b.iter(|| {
                black_box(&mut vi).rotate_right(1);
            });
        });

        group.bench_function(BenchmarkId::new("copy", n), |b| {
            let (vi, mut ve) = bufs(n);
            b.iter(|| {
                ve.clear();
                black_box(&mut ve).extend_from_slice(black_box(&vi.as_slice()));
            });
        });
    }

    for n in [4096] {
        group.throughput(Throughput::Bytes(n as u64));

        let setup = || (0..n).into_iter().map(|x| (x * 256 / n) as u8).collect::<Vec<u8>>();

        group.bench_function(BenchmarkId::new("find-single-value", n), |b| {
            let needle = 128;
            b.iter_with_setup(setup, |vi| {
                black_box(vi.iter().position(|&x| x == needle));
            });
        });

        group.bench_function(BenchmarkId::new("find-one-of-two-values", n), |b| {
            b.iter_with_setup(setup, |vi| {
                black_box(vi.iter().position(|&x| matches!(x, 128 | 192)));
            });
        });

        group.bench_function(BenchmarkId::new("find-one-of-four-values", n), |b| {
            b.iter_with_setup(setup, |vi| {
                black_box(vi.iter().position(|&x| matches!(x, 128 | 192 | 224 | 240)));
            });
        });
    }

    group.finish();
}
