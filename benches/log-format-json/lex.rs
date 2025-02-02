// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::*;
use logos::Logos;
use stats_alloc::{Region, Stats};

use super::{add_stat, GA, KIBANA_REC_1};

use log_format_json::{Lexer, Token};

criterion_group!(benches, lex);

fn lex(c: &mut Criterion) {
    let mut group = c.benchmark_group("json-lex");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));

    let mut iterations = 0;
    let mut allocs = Stats::default();
    let test = "drain-tokens/inner";

    group.bench_function(test, |b| {
        b.iter(|| {
            let reg = Region::new(&GA);

            let mut lexer = Token::lexer(KIBANA_REC_1);
            while let Some(_) = black_box(lexer.next()) {}

            add_stat(&mut allocs, &reg.change());
            iterations += 1;
        });
    });

    println!("{}: allocations per {:?} iterations: {:#?}", test, iterations, allocs);

    let mut iterations = 0;
    let mut allocs = Stats::default();
    let test = "drain-tokens/log-format";

    group.bench_function(test, |b| {
        b.iter(|| {
            let reg = Region::new(&GA);

            let mut lexer = Lexer::from_slice(KIBANA_REC_1);
            while let Some(_) = black_box(lexer.next()) {}

            add_stat(&mut allocs, &reg.change());
            iterations += 1;
        });
    });

    println!("{}: allocations per {:?} iterations: {:#?}", test, iterations, allocs);

    group.finish();
}
