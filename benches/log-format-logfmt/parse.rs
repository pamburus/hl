// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::*;
use flat_tree::FlatTree;
use log_ast::ast;
use log_format::{ast::Discarder, Format};
use log_format_logfmt::LogfmtFormat;

use super::KIBANA_REC_1;

criterion_group!(benches, parse);

fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("logfmt-parse");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));

    group.bench_function("discard", |b| {
        b.iter(|| black_box(LogfmtFormat::parse(KIBANA_REC_1, Discarder::new())).unwrap());
    });

    group.bench_function("ast", |b| {
        let mut tree = FlatTree::<ast::Value>::new();
        b.iter(|| {
            black_box(LogfmtFormat::parse(KIBANA_REC_1, ast::Builder::new(tree.metaroot())))
                .map_err(|x| x.0)
                .unwrap();
            tree.clear();
        });
    });

    group.finish();
}
