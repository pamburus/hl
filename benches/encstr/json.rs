// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::*;
use serde_json::de::{Read, StrRead};

// local imports
use encstr::{json::JsonEncodedString, AnyEncodedString, Builder, Handler, Ignorer};

criterion_group!(benches, serde, decode, tokens);

fn serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("encstr/json/serde");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("medium", |b| {
        let mut buf = Vec::with_capacity(4096);
        b.iter(|| {
            buf.clear();
            let mut reader = StrRead::new(&MEDIUM[1..]);
            black_box(reader.parse_str_raw(black_box(&mut buf))).unwrap();
        });
    });
    group.finish();
}

fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encstr/json/decode");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("ignore/medium", |b| {
        let _ = serde_json::from_str::<String>(MEDIUM).unwrap();
        let string = JsonEncodedString::new(MEDIUM);
        let mut result = Ignorer;
        b.iter(|| {
            string.decode(black_box(&mut result)).unwrap();
        });
    });
    group.bench_function("build/medium", |b| {
        let string = JsonEncodedString::new(MEDIUM);
        let mut result = Builder::with_capacity(4096);
        b.iter(|| {
            result.clear();
            string.decode(black_box(&mut result)).unwrap();
        });
        assert_eq!(result.as_str(), serde_json::from_str::<String>(MEDIUM).unwrap());
    });
    group.finish();
}

fn tokens(c: &mut Criterion) {
    let mut group = c.benchmark_group("encstr/json/tokens");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("ignore/medium", |b| {
        let string = JsonEncodedString::new(MEDIUM);
        b.iter(|| {
            for token in string.tokens() {
                black_box(token).unwrap();
            }
        });
    });
    group.bench_function("build/medium", |b| {
        let string = JsonEncodedString::new(MEDIUM);
        let mut result = Builder::with_capacity(4096);
        b.iter(|| {
            result.clear();
            for token in black_box(string.tokens()) {
                black_box(result.handle(black_box(token).unwrap())).unwrap();
            }
        });
        assert_eq!(result.as_str(), serde_json::from_str::<String>(MEDIUM).unwrap());
    });
    group.finish();
}

const MEDIUM: &str = r#""UPDATE \"apple\" SET \"seed\"='8c858361-5b73-442e-b84c-78482ed60ce1',\"planted_at\"=now() + timeout,\"importer\"='00d1cce2-c32e-4bb7-88da-474083fc2a1a',\"start_at\"=now() + repeat_interval,\"planted_at\"=now(),\"state\"='running',\"updated_at\"='2023-12-04 10:01:29.399' WHERE id IN (SELECT id FROM \"apple\" WHERE breed in ('red-delicious') AND distributor in ('magic-fruits','grand-provider') AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'b66134a4-c5c5-4adc-8c33-c8b7f780853b' AND importer != 'f86eb35d-33cd-499b-85cd-da175188e459'))) OR (now() >= planted_at)) ORDER BY \"updated_at\" LIMIT 4) AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'a3ecc839-0a32-4722-b4db-90c2ce8296a5' AND importer != '73a1fe4e-f4d1-4d09-99cb-9b07f2e32a96'))) OR (now() >= planted_at)) RETURNING *""#;
