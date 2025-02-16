// std imports
use std::{hint::black_box, ops::Range, time::Duration, vec::Vec};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};

// workspace imports
use log_ast::ast::{Composite, Container, Node, Scalar, String, Value};
use log_format::Format;
use log_format_json::JsonFormat;

// local imports
use super::{hash, samples, ND};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "log-ast");

const SAMPLES: &[&[u8]] = &[samples::log::elk01::JSON];

pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);
    c.warm_up_time(Duration::from_secs(1));
    c.measurement_time(Duration::from_secs(3));

    for &sample in SAMPLES {
        let param = format!("{}:{}", sample.len(), hash(sample));

        let setup = || {
            let sample = Vec::from(sample);
            let mut container = Container::with_capacity(160);
            JsonFormat
                .parse(&sample, container.metaroot())
                .map_err(|x| x.0)
                .unwrap();
            (sample, container)
        };

        let container = setup().1;

        c.throughput(Throughput::Elements(container.roots().len() as u64));
        c.bench_function(BenchmarkId::new("roots", &param), |b| {
            b.iter_batched_ref(
                setup,
                |(_, container)| {
                    for root in container.roots() {
                        let _ = black_box(root);
                    }
                },
                BatchSize::SmallInput,
            );
        });

        c.throughput(Throughput::Elements(container.nodes().len() as u64));
        c.bench_function(BenchmarkId::new("traverse:fast", &param), |b| {
            b.iter_batched_ref(
                setup,
                |(sample, container)| {
                    for root in container.roots() {
                        traverse_fast(sample, root)
                    }
                },
                BatchSize::SmallInput,
            );
        });

        c.throughput(Throughput::Elements(container.nodes().len() as u64));
        c.bench_function(BenchmarkId::new("traverse:match:drain", &param), |b| {
            b.iter_batched_ref(
                setup,
                |(sample, container)| {
                    for root in container.roots() {
                        traverse_match_drain(sample, root)
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    c.finish();
}

fn traverse_fast(sample: &[u8], node: Node) {
    for child in node.children() {
        traverse_fast(sample, child);
    }
}

fn traverse_match_drain(sample: &[u8], node: Node) {
    match node.value() {
        Value::Scalar(scalar) => match scalar {
            Scalar::Null => {
                let _ = black_box(());
            }
            Scalar::Bool(value) => {
                let _ = black_box(value);
            }
            Scalar::Number(span) => {
                let _ = black_box(&sample[Range::from(*span)]);
            }
            Scalar::String(String::Plain(span)) => {
                let _ = black_box(&sample[Range::from(*span)]);
            }
            Scalar::String(String::JsonEscaped(span)) => {
                let _ = black_box(&sample[Range::from(*span)]);
            }
        },
        Value::Composite(composite) => match composite {
            Composite::Array => {
                black_box(b'[');
                for field in node.children() {
                    traverse_match_drain(sample, field);
                }
                black_box(b']');
            }
            Composite::Object => {
                black_box(b'{');
                for field in node.children() {
                    traverse_match_drain(sample, field);
                }
                black_box(b'}');
            }
            Composite::Field(key) => {
                let _ = black_box(key);
                for field in node.children() {
                    traverse_match_drain(sample, field);
                }
            }
        },
    }
}
