// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::*;
use flat_tree::FlatTree;
use log_ast::ast;
use log_format::{ast2::Discarder, Format};
use log_format_json::{Error, JsonFormat};

use super::KIBANA_REC_1;

criterion_group!(benches, parse);

fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));

    group.bench_function("discard", |b| {
        b.iter(|| black_box(JsonFormat::parse(KIBANA_REC_1, Discarder::<Error>::new())).unwrap());
    });

    group.bench_function("ast", |b| {
        let mut tree = FlatTree::new();
        b.iter(|| black_box(JsonFormat::parse(KIBANA_REC_1, ast::Builder::new(tree.metaroot()))).unwrap());
    });

    group.finish();
}
