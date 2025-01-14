// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::*;
use log_format::{ast::Discarder, Format};
use stats_alloc::{Region, Stats};

use log_format_json::{Error, JsonFormat};

use super::{add_stat, GA, KIBANA_REC_1};

criterion_group!(benches, parse);

fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));

    let mut iterations = 0;
    let mut allocs = Stats::default();
    let test = "discard";

    group.bench_function(test, |b| {
        b.iter(|| {
            let reg = Region::new(&GA);

            black_box(JsonFormat::parse(KIBANA_REC_1, Discarder::<Error>::new())).unwrap();

            add_stat(&mut allocs, &reg.change());
            iterations += 1;
        });
    });

    println!("{}: allocations per {:?} iterations: {:#?}", test, iterations, allocs);

    group.finish();
}
