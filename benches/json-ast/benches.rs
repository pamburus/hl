use criterion::criterion_main;

mod container;

criterion_main!(container::benches);
