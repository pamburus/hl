// std imports
use std::{
    io::{Cursor, Write},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

// third-party imports
use chrono::{Offset, Utc};
use const_str::concat as strcat;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

// local imports
use super::{BencherExt, ND, hash};
use hl::{
    App, Delimiter, FieldOptions, LinuxDateFormat, Options, Theme,
    app::OutputFormat,
    input::{InputHolder, InputReference},
    settings::{AsciiMode, ExpansionMode, Formatting},
    timezone::Tz,
};

const GROUP: &str = strcat!(super::GROUP, ND, "follow");

const THEME: &str = "universal";
const INPUT_SIZE: usize = 4 << 20;
const INPUTS: [usize; 2] = [1, 4];

/// Runs the whole follow mode pipeline, from reading the inputs to writing the merged output,
/// on inputs that end, so that each run ends when they are read.
pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);
    c.warm_up_time(Duration::from_secs(1));
    c.measurement_time(Duration::from_secs(5));

    let app = App::new(options());

    for inputs in INPUTS {
        // The timestamps of the inputs interleave, so their records have to be sorted when merged.
        let data: Vec<&'static [u8]> = (0..inputs).map(|i| generate(i, inputs)).collect();
        let param = format!("json:{}:x{}", hash(data[0]), inputs);
        c.throughput(Throughput::Bytes(data.iter().map(|x| x.len() as u64).sum()));

        c.bench_function(BenchmarkId::new("run", &param), |b| {
            let setup = || {
                data.iter()
                    .map(|&data| InputHolder::new(InputReference::Stdin, Some(Box::new(Cursor::new(data)))))
                    .collect::<Vec<_>>()
            };

            b.iter_batched_fixed(
                setup,
                |inputs| app.run(inputs, &mut std::io::sink()).unwrap(),
                BatchSize::LargeInput,
            );
        });
    }
}

/// Generates the data of input `i` of `n`: JSON records with timestamps that grow by a millisecond,
/// shifted for each input, so that the records of all inputs alternate.
fn generate(i: usize, n: usize) -> &'static [u8] {
    let mut data = Vec::with_capacity(INPUT_SIZE + 256);
    let mut j = 0;
    while data.len() < INPUT_SIZE {
        let us = j * 1000 + i * 1000 / n;
        writeln!(
            data,
            r#"{{"ts":"2026-07-06T10:{:02}:{:02}.{:06}Z","level":"info","logger":"bench","msg":"request {j} of input {i} handled","method":"GET","status":200,"duration":0.{:03}}}"#,
            us / 60_000_000 % 60,
            us / 1_000_000 % 60,
            us % 1_000_000,
            j % 1000,
        )
        .unwrap();
        j += 1;
    }
    data.leak()
}

fn options() -> Options {
    Options {
        theme: Arc::new(Theme::embedded(THEME).unwrap()),
        time_format: LinuxDateFormat::new("%b %d %T.%3N").compile(),
        raw: false,
        raw_fields: false,
        allow_prefix: false,
        buffer_size: NonZeroUsize::new(256 << 10).unwrap(),
        max_message_size: NonZeroUsize::new(64 << 20).unwrap(),
        concurrency: 4,
        filter: Default::default(),
        fields: FieldOptions::default(),
        formatting: Formatting::default(),
        time_zone: Tz::FixedOffset(Utc.fix()),
        hide_empty_fields: false,
        sort: false,
        follow: true,
        sync_interval: Duration::from_millis(100),
        input_info: Default::default(),
        input_format: None,
        dump_index: false,
        app_dirs: None,
        tail: 0,
        delimiter: Delimiter::default(),
        unix_ts_unit: None,
        flatten: false,
        ascii: AsciiMode::Off,
        expand: ExpansionMode::default(),
        output_delimiter: "\n".to_string(),
        output_format: OutputFormat::Pretty,
    }
}
