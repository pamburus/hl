// std imports
use std::time::Duration;

// third-party imports
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};
use rand::random;

criterion_group!(benches, bench);

const GROUP: &str = "mem";

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
        let vi: Vec<u8> = (0..size).into_iter().map(|_| next()).collect();
        let ve: Vec<u8> = Vec::with_capacity(size);
        (vi, ve)
    };

    let variants = [
        (512, BatchSize::SmallInput),
        (4096, BatchSize::SmallInput),
        (65536, BatchSize::LargeInput),
        (1048576, BatchSize::LargeInput),
    ];

    for (n, batch) in variants {
        group.throughput(Throughput::Bytes(n as u64));

        group.bench_function(BenchmarkId::new("rotate:1", n), |b| {
            let setup = || bufs(n).0;
            b.iter_batched(setup, |mut vi| vi.rotate_right(1), batch);
        });

        group.bench_function(BenchmarkId::new("copy", n), |b| {
            let setup = || bufs(n);
            b.iter_batched(setup, |(vi, mut ve)| ve.extend_from_slice(vi.as_slice()), batch);
        });
    }

    let variants = [(4096, BatchSize::SmallInput)];

    for (n, batch) in variants {
        group.throughput(Throughput::Bytes(n as u64));

        let setup = || (0..n).into_iter().map(|x| (x * 256 / n) as u8).collect::<Vec<u8>>();
        let param = |x| format!("{}:{}", n, x);

        group.bench_function(BenchmarkId::new("position", param("single-value")), |b| {
            let needle = 128;
            b.iter_batched(setup, |vi| vi.iter().position(|&x| x == needle), batch);
        });

        group.bench_function(BenchmarkId::new("position", param("one-of-two-values")), |b| {
            b.iter_batched(setup, |vi| vi.iter().position(|&x| matches!(x, 128 | 192)), batch);
        });

        group.bench_function(BenchmarkId::new("position", param("one-of-four-values")), |b| {
            b.iter_batched(
                setup,
                |vi| vi.iter().position(|&x| matches!(x, 128 | 192 | 224 | 240)),
                batch,
            );
        });
    }

    group.finish();
}
