// third-party imports
use criterion::{criterion_group, Criterion};
use stats_alloc::Region;

// local imports
use super::GA;
use hl::timestamp::Timestamp;

criterion_group!(benches, benchmark);

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("ts-parse");
    let unix = "1596742694";
    let unix_us = "1596742694123654";
    let rfc3339 = "2020-06-27T00:48:30.466249792+03:00";
    c.bench_function("regex rfc3339", |b| {
        use regex::Regex;
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-](\d{2}:\d{2}))?$").unwrap();
        b.iter(|| assert!(re.is_match(rfc3339)));
    });
    c.bench_function("as_rfc3339", |b| {
        let ts = Timestamp::new(rfc3339);
        b.iter(|| assert!(ts.as_rfc3339().is_some()));
    });
    c.bench_function("parse unix", |b| {
        let ts = Timestamp::new(unix);
        b.iter(|| assert!(ts.parse().is_some()))
    });
    c.bench_function("parse unix microseconds", |b| {
        let ts = Timestamp::new(unix_us);
        b.iter(|| assert!(ts.parse().is_some()))
    });

    let mut c1 = None;
    let mut n1 = 0;
    c.bench_function("parse rfc3339", |b| {
        let ts = Timestamp::new(rfc3339);
        let reg = Region::new(&GA);
        b.iter(|| {
            n1 += 1;
            assert!(ts.parse().is_some())
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);
}
