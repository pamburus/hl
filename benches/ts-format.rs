use std::io::Write;

use chrono::{format::strftime::StrftimeItems, Datelike, FixedOffset, Timelike};
use criterion::{criterion_group, criterion_main, Criterion};

use hl::datefmt::{DateTimeFormatter, LinuxDateFormat};
use hl::timestamp::Timestamp;

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("ts-format");
    let tsr = Timestamp::new("2020-06-27T00:48:30.466249792+00:00", None);
    let ts = tsr.parse().unwrap();
    let tsn = ts.naive_local();
    c.bench_function("chrono conversion to naive local", |b| {
        b.iter(|| {
            ts.naive_local();
        });
    });
    c.bench_function("datefmt format utc [%y-%m-%d %T.%N]", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        let format = LinuxDateFormat::new("%y-%m-%d %T.%N").compile();
        let formatter = DateTimeFormatter::new(format, FixedOffset::east(0));
        b.iter(|| {
            formatter.format(&mut buf, ts);
            buf.clear();
        });
    });
    c.bench_function("datefmt format msk [%y-%m-%d %T.%N]", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        let format = LinuxDateFormat::new("%y-%m-%d %T.%N").compile();
        let formatter = DateTimeFormatter::new(format, FixedOffset::east(3 * 3600));
        b.iter(|| {
            formatter.format(&mut buf, ts);
            buf.clear();
        });
    });
    c.bench_function("datefmt format utc [%b %d %T.%N]", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        let format = LinuxDateFormat::new("%b %d %T.%N").compile();
        let formatter = DateTimeFormatter::new(format, FixedOffset::east(0));
        b.iter(|| {
            formatter.format(&mut buf, ts);
            buf.clear();
        });
    });
    c.bench_function("datefmt format msk [%b %d %T.%N]", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        let format = LinuxDateFormat::new("%b %d %T.%N").compile();
        let formatter = DateTimeFormatter::new(format, FixedOffset::east(3 * 3600));
        b.iter(|| {
            formatter.format(&mut buf, ts);
            buf.clear();
        });
    });
    c.bench_function("datefmt re-format utc [%y-%m-%d %T.%N]", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        let format = LinuxDateFormat::new("%y-%m-%d %T.%N").compile();
        let formatter = DateTimeFormatter::new(format, FixedOffset::east(0));
        let tsr = tsr.as_rfc3339().unwrap();
        b.iter(|| {
            formatter.reformat_rfc3339(&mut buf, tsr.clone());
            buf.clear();
        });
    });
    c.bench_function("datefmt re-format utc [%b %d %T.%N]", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        let format = LinuxDateFormat::new("%b %d %T.%N").compile();
        let formatter = DateTimeFormatter::new(format, FixedOffset::east(0));
        let tsr = tsr.as_rfc3339().unwrap();
        b.iter(|| {
            formatter.reformat_rfc3339(&mut buf, tsr.clone());
            buf.clear();
        });
    });
    c.bench_function("datefmt re-format utc [%Y-%m-%d %T.%N %:z]", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        let format = LinuxDateFormat::new("%Y-%m-%d %T.%N %:z").compile();
        let formatter = DateTimeFormatter::new(format, FixedOffset::east(0));
        let tsr = tsr.as_rfc3339().unwrap();
        b.iter(|| {
            formatter.reformat_rfc3339(&mut buf, tsr.clone());
            buf.clear();
        });
    });
    c.bench_function("calling chrono date-time methods", |b| {
        b.iter(|| {
            assert!(
                tsn.year() as i64
                    + tsn.month() as i64
                    + tsn.day() as i64
                    + tsn.hour() as i64
                    + tsn.minute() as i64
                    + tsn.second() as i64
                    + tsn.nanosecond() as i64
                    != 0
            );
        });
    });
    let items = StrftimeItems::new("%y-%m-%d %T.%3f");
    c.bench_function("chrono format", |b| {
        let mut buf = Vec::<u8>::with_capacity(4096);
        b.iter(|| {
            assert!(write!(&mut buf, "{}", ts.format_with_items(items.clone())).is_ok());
            buf.clear();
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
