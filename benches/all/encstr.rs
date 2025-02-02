// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::*;
use serde_json::de::{Read, StrRead};

// local imports
use super::hash;
use encstr::{json::JsonEncodedString, AnyEncodedString, Builder, Handler, Ignorer};

criterion_group!(benches, bench);

const GROUP: &str = "encstr";

fn bench(c: &mut Criterion) {
    for input in [JSON_MEDIUM] {
        bench_with(c, "json", JsonEncodedString::new(input));
    }
    for input in [RAW_MEDIUM] {
        bench_with(c, "raw", JsonEncodedString::new(input));
    }
}

fn bench_with<'a, I: AnyEncodedString<'a>>(c: &mut Criterion, title: &str, input: I) {
    let param = format!("{}:{}:{}", title, input.source().len(), hash(input.source()));
    let mut group = c.benchmark_group(GROUP);
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));
    group.throughput(Throughput::Bytes(input.source().len() as u64));
    group.bench_function(BenchmarkId::new("serde:parse_str_raw", &param), |b| {
        let setup = || (Vec::with_capacity(4096), StrRead::new(black_box(&input.source()[1..])));
        b.iter_with_setup(setup, |(mut buf, mut reader)| {
            black_box(reader.parse_str_raw(black_box(&mut buf)).unwrap());
        });
    });
    group.bench_function(BenchmarkId::new("serde:ignore_str", &param), |b| {
        let setup = || StrRead::new(black_box(&input.source()[1..]));
        b.iter_with_setup(setup, |mut reader| {
            black_box(reader.ignore_str().unwrap());
        });
    });
    group.bench_function(BenchmarkId::new("decode:ignore", &param), |b| {
        let mut result = Ignorer;
        b.iter(|| {
            black_box(&input).decode(&mut result).unwrap();
        });
    });
    group.bench_function(BenchmarkId::new("decode:build", &param), |b| {
        let setup = || Builder::with_capacity(4096);
        b.iter_with_setup(setup, |mut buf| {
            black_box(&input).decode(&mut buf).unwrap();
        });
    });
    group.bench_function(BenchmarkId::new("tokens:ignore", &param), |b| {
        b.iter(|| {
            for token in black_box(&input).tokens() {
                token.unwrap();
            }
        });
    });
    group.bench_function(BenchmarkId::new("tokens:build", &param), |b| {
        let setup = || Builder::with_capacity(4096);
        b.iter_with_setup(setup, |mut buf| {
            for token in black_box(&input).tokens() {
                buf.handle(token.unwrap()).unwrap();
            }
        });
    });
    group.finish();
}

const JSON_MEDIUM: &str = r#""UPDATE \"apple\" SET \"seed\"='8c858361-5b73-442e-b84c-78482ed60ce1',\"planted_at\"=now() + timeout,\"importer\"='00d1cce2-c32e-4bb7-88da-474083fc2a1a',\"start_at\"=now() + repeat_interval,\"planted_at\"=now(),\"state\"='running',\"updated_at\"='2023-12-04 10:01:29.399' WHERE id IN (SELECT id FROM \"apple\" WHERE breed in ('red-delicious') AND distributor in ('magic-fruits','grand-provider') AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'b66134a4-c5c5-4adc-8c33-c8b7f780853b' AND importer != 'f86eb35d-33cd-499b-85cd-da175188e459'))) OR (now() >= planted_at)) ORDER BY \"updated_at\" LIMIT 4) AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'a3ecc839-0a32-4722-b4db-90c2ce8296a5' AND importer != '73a1fe4e-f4d1-4d09-99cb-9b07f2e32a96'))) OR (now() >= planted_at)) RETURNING *""#;

const RAW_MEDIUM: &str = r#"UPDATE "apple" SET "seed"='8c858361-5b73-442e-b84c-78482ed60ce1',"planted_at"=now() + timeout,"importer"='00d1cce2-c32e-4bb7-88da-474083fc2a1a',"start_at"=now() + repeat_interval,"planted_at"=now(),"state"='running',"updated_at"='2023-12-04 10:01:29.399' WHERE id IN (SELECT id FROM "apple" WHERE breed in ('red-delicious') AND distributor in ('magic-fruits','grand-provider') AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'b66134a4-c5c5-4adc-8c33-c8b7f780853b' AND importer != 'f86eb35d-33cd-499b-85cd-da175188e459'))) OR (now() >= planted_at)) ORDER BY "updated_at" LIMIT 4) AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'a3ecc839-0a32-4722-b4db-90c2ce8296a5' AND importer != '73a1fe4e-f4d1-4d09-99cb-9b07f2e32a96'))) OR (now() >= planted_at)) RETURNING *"#;
