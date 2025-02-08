// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};

// local imports
use super::{hash, BencherExt, ND};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "wildcard");

fn bench(c: &mut Criterion) {
    bench_with::<wildmatch::WildMatch>(c, "wildmatch");
    bench_with::<wildflower::Pattern<&str>>(c, "wildflower");
}

fn bench_with<Pattern: Wildcard>(c: &mut Criterion, title: &str) {
    let mut c = c.benchmark_group(GROUP);
    c.warm_up_time(Duration::from_secs(1));
    c.measurement_time(Duration::from_secs(3));

    const P1X: (&str, &str) = ("1x", "_*");
    const P27X: (&str, &str) = ("27x", "SOME_VERY_VERY_LONG_PREFIX_*");

    let variants = [
        ("short", "_TEST", P1X, true),
        ("short", "TEST", P1X, false),
        ("long", "_TEST_SOME_VERY_VERY_LONG_NAME", P1X, true),
        ("long", "SOME_VERY_VERY_LONG_PREFIX_AND_SOMEWHAT", P27X, true),
        ("long", "TEST_SOME_VERY_VERY_LONG_NAME", P27X, false),
    ];

    for (name, input, (pname, pattern), expected) in &variants {
        let function = format!("{}:{}", title, "matches");
        let param = format!(
            "{}:{}:{}:{}:{}",
            name,
            pname,
            if *expected { "pos" } else { "neg" },
            input.len(),
            hash((pattern, input))
        );
        let pattern = Pattern::new(pattern);
        let setup = || String::from(*input);
        let routine = |input: String| black_box(&pattern).matches(&input);

        assert_eq!(routine(setup()), *expected);

        c.throughput(Throughput::Bytes(input.len() as u64));
        c.bench_function(BenchmarkId::new(function, param), |b| {
            b.iter_batched_fixed(setup, routine, BatchSize::NumIterations(16384));
        });
    }
}

// ---

trait Wildcard {
    fn new(pattern: &'static str) -> Self;
    fn matches(&self, what: &str) -> bool;
}

impl Wildcard for wildmatch::WildMatch {
    #[inline(always)]
    fn new(pattern: &str) -> Self {
        Self::new(pattern)
    }

    #[inline(always)]
    fn matches(&self, what: &str) -> bool {
        self.matches(what)
    }
}

impl Wildcard for wildflower::Pattern<&'static str> {
    #[inline(always)]
    fn new(pattern: &'static str) -> Self {
        Self::new(pattern)
    }

    #[inline(always)]
    fn matches(&self, what: &str) -> bool {
        self.matches(what)
    }
}
