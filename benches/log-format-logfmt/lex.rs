// std imports
use std::{hint::black_box, time::Duration};

// third-party imports
use criterion::*;

use super::KIBANA_REC_1;

use log_format_logfmt::{Lexer, Token};

criterion_group!(benches, lex);

fn lex(c: &mut Criterion) {
    let mut group = c.benchmark_group("logfmt-lex");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));

    let test = "drain-tokens/inner";

    group.bench_function(test, |b| {
        b.iter(|| {
            let mut lexer = Token::lexer(black_box(KIBANA_REC_1));
            while let Some(_) = lexer.next() {}
            black_box(lexer);
        });
    });

    let test = "drain-tokens/log-format";

    group.bench_function(test, |b| {
        b.iter(|| {
            let mut lexer = Lexer::from_slice(black_box(KIBANA_REC_1));
            while let Some(_) = lexer.next() {}
            black_box(lexer);
        });
    });

    group.finish();
}
