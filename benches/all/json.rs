// third-party imports
use criterion::{criterion_group, Criterion};
use serde_json as json;
use stats_alloc::Region;

// local imports
use super::GA;

criterion_group!(benches, benchmark);

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("json");

    let mut c1 = None;
    let mut n1 = 0;
    c.bench_function("parse-to-str", |b| {
        let sample = r#""test-message""#;
        let reg = Region::new(&GA);
        b.iter(|| {
            assert_eq!(json::from_str::<&str>(sample).unwrap(), "test-message");
            n1 += 1;
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);

    let mut c2 = None;
    let mut n2 = 0;
    c.bench_function("parse-to-string", |b| {
        let sample = r#""test-\"message\"""#;
        let reg = Region::new(&GA);
        b.iter(|| {
            assert_eq!(json::from_str::<String>(sample).unwrap(), r#"test-"message""#);
            n2 += 1;
        });
        c2 = Some(reg.change());
    });
    println!("allocations at 2 ({:?} iterations): {:#?}", n2, c2);
}
