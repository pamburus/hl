// std imports
use std::time::Duration;

// third-party imports
use const_str::concat as strcat;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

// local imports
use super::ND;
use hl::timestamp::Timestamp;

const GROUP: &str = strcat!(super::GROUP, ND, "timestamp");

pub(super) mod parsing {
    use super::*;

    const GROUP: &str = strcat!(super::GROUP, ND, "parsing");

    pub fn bench(c: &mut Criterion) {
        let mut c = c.benchmark_group(GROUP);
        c.warm_up_time(Duration::from_secs(1));
        c.measurement_time(Duration::from_secs(3));

        let unix = "1596742694";
        let unix_us = "1596742694123654";
        let rfc3339 = "2020-06-27T00:48:30.466249792+03:00";

        let variants = [("rfc3339", rfc3339)];

        for (name, input) in variants {
            c.throughput(Throughput::Bytes(input.len() as u64));

            c.bench_function(BenchmarkId::new("regex:match", name), |b| {
                use regex::Regex;
                let re = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-](\d{2}:\d{2}))?$").unwrap();
                let setup = || input;
                let perform = |input| re.is_match(input);
                assert!(perform(setup()));
                b.iter_batched(setup, perform, BatchSize::SmallInput);
            });

            c.bench_function(BenchmarkId::new("as-rfc3339", name), |b| {
                let setup = || Timestamp::new(input);
                let perform = |ts: Timestamp| ts.as_rfc3339().is_some();
                assert!(perform(setup()));
                b.iter_batched(setup, perform, BatchSize::SmallInput);
            });
        }

        let variants = [
            ("unix:seconds", unix),
            ("unix:microseconds", unix_us),
            ("rfc3339", rfc3339),
        ];

        for (name, input) in variants {
            c.throughput(Throughput::Bytes(input.len() as u64));

            c.bench_function(BenchmarkId::new("parse", name), |b| {
                let setup = || Timestamp::new(input);
                let perform = |ts: Timestamp| ts.parse();
                assert!(perform(setup()).is_some());
                b.iter_batched(setup, perform, BatchSize::SmallInput);
            });
        }
    }
}

pub mod formatting {
    use super::*;

    // std imports
    use std::io::Write;

    // third-party imports
    use chrono::{format::strftime::StrftimeItems, DateTime, Datelike, FixedOffset, NaiveDateTime, Timelike};

    // local imports
    use hl::datefmt::{DateTimeFormatter, LinuxDateFormat};
    use hl::{
        timestamp::{rfc3339, Timestamp},
        timezone::Tz,
    };

    const GROUP: &str = strcat!(super::GROUP, ND, "formatting");

    pub(crate) fn bench(c: &mut Criterion) {
        let mut c = c.benchmark_group(GROUP);
        c.warm_up_time(Duration::from_secs(1));
        c.measurement_time(Duration::from_secs(1));

        let tsr = || Timestamp::new("2020-06-27T00:48:30.466249792+00:00");
        let ts = || tsr().parse().unwrap();
        let tz = |secs| Tz::FixedOffset(FixedOffset::east_opt(secs).unwrap());

        c.bench_function("chrono:naive-local", |b| {
            let perform = |ts: DateTime<FixedOffset>| ts.naive_local();
            b.iter_batched(ts, perform, BatchSize::SmallInput);
        });

        c.bench_function("chrono:methods", |b| {
            let setup = || ts().naive_local();
            let perform = |tsn: NaiveDateTime| {
                tsn.year() as i64
                    + tsn.month() as i64
                    + tsn.day() as i64
                    + tsn.hour() as i64
                    + tsn.minute() as i64
                    + tsn.second() as i64
                    + tsn.nanosecond() as i64
            };
            assert!(perform(setup()) != 0);
            b.iter_batched(setup, perform, BatchSize::SmallInput);
        });

        let setup = || {
            let items = StrftimeItems::new("%y-%m-%d %T.%3f");
            let buf = Vec::<u8>::with_capacity(128);
            (items, buf, ts())
        };
        let perform = |(items, mut buf, ts): (StrftimeItems, Vec<u8>, DateTime<FixedOffset>)| {
            write!(&mut buf, "{}", ts.format_with_items(items)).map(|_| buf.len())
        };
        c.throughput(Throughput::Bytes(perform(setup()).unwrap() as u64));
        c.bench_function(BenchmarkId::new("chrono:format-with-items", "ymdT3f"), |b| {
            b.iter_batched(setup, perform, BatchSize::SmallInput);
        });

        let zones = &[("utc", 0), ("cet", 3600)];
        let formats = &[("ymdTN", "%y-%m-%d %T.%N"), ("bdTN", "%b %d %T.%N")];

        for (tzn, tzv) in zones {
            for (fmt_name, fmt) in formats {
                let param = format!("{}:{}", tzn, fmt_name);
                let setup = || {
                    let buf = Vec::<u8>::with_capacity(128);
                    let format = LinuxDateFormat::new(fmt).compile();
                    let formatter = DateTimeFormatter::new(format, tz(*tzv));
                    (formatter, buf, ts())
                };
                let perform = |(formatter, mut buf, ts): (DateTimeFormatter, Vec<u8>, DateTime<FixedOffset>)| {
                    formatter.format(&mut buf, ts);
                    buf.len()
                };
                c.throughput(Throughput::Bytes(perform(setup()) as u64));
                c.bench_function(BenchmarkId::new("format", param), |b| {
                    b.iter_batched(setup, perform, BatchSize::SmallInput);
                });
            }
        }

        let zones = &[("utc", 0), ("cet", 3600)];
        let formats = &[
            ("ymdTN", "%y-%m-%d %T.%N"),
            ("bdTN", "%b %d %T.%N"),
            ("YmdTNz", "%Y-%m-%d %T.%N %:z"),
        ];

        for (tzn, tzv) in zones {
            for (fmt_name, fmt) in formats {
                let param = format!("{}:{}", tzn, fmt_name);
                let tsr = tsr();
                let tsr = tsr.as_rfc3339().unwrap();
                let setup = || {
                    let buf = Vec::<u8>::with_capacity(128);
                    let format = LinuxDateFormat::new(fmt).compile();
                    let formatter = DateTimeFormatter::new(format, tz(*tzv));
                    (formatter, buf, tsr.clone())
                };
                let perform = |(formatter, mut buf, tsr): (DateTimeFormatter, Vec<u8>, rfc3339::Timestamp)| {
                    formatter.reformat_rfc3339(&mut buf, tsr);
                    buf.len()
                };
                c.throughput(Throughput::Bytes(perform(setup()) as u64));
                c.bench_function(BenchmarkId::new("reformat-rfc3339", param), |b| {
                    b.iter_batched(setup, perform, BatchSize::SmallInput);
                });
            }
        }
    }
}
