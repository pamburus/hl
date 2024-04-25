use criterion::criterion_main;

mod json;

criterion_main!(json::benches);
