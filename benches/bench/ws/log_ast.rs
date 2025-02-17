// std imports
use std::{hint::black_box, ops::Range, sync::Arc, time::Duration, vec::Vec};

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};

// workspace imports
use log_ast::{
    ast::{Composite, Container, Node, Scalar, String, Value},
    model::{self, FormatExt, Segment},
};
use log_format::Format;
use log_format_json::JsonFormat;

// local imports
use super::{hash, samples, ND};
use crate::utf8;

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

        let setup = || {
            let sample = Arc::<str>::from(utf8!(sample));
            let mut segment = Segment::with_capacity(160);
            JsonFormat.parse_into(sample, &mut segment).1.unwrap();
            segment
        };

        c.throughput(Throughput::Elements(
            segment_node_count::entries(setup().entries()) as u64
        ));
        c.bench_function(BenchmarkId::new("segment:traverse:match:drain", &param), |b| {
            b.iter_batched_ref(
                setup,
                |segment| segment_node_count::entries(segment.entries()),
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

mod segment_node_count {
    use super::*;

    pub(super) fn entries<'s>(entries: model::Entries<'s>) -> usize {
        entries.into_iter().map(object).sum()
    }

    pub(super) fn array<'s>(array: model::Array<'s>) -> usize {
        array.into_iter().map(value).sum()
    }

    pub(super) fn object<'s>(object: model::Object<'s>) -> usize {
        object.into_iter().map(field).sum()
    }

    pub(super) fn field<'s>(field: (model::String<'s>, model::Value<'s>)) -> usize {
        black_box(field.0.text());
        1 + value(field.1)
    }

    pub(super) fn value<'s>(value: model::Value<'s>) -> usize {
        match value {
            model::Value::Null => 1,
            model::Value::Bool(v) => {
                black_box(v);
                1
            }
            model::Value::Number(n) => {
                black_box(n.text());
                1
            }
            model::Value::String(s) => {
                black_box(s.text());
                1
            }
            model::Value::Array(a) => array(a),
            model::Value::Object(o) => object(o),
        }
    }
}
