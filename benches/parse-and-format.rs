// std imports
use std::sync::Arc;

// third-party imports
use byte_strings::concat_bytes;
use chrono::FixedOffset;
use criterion::{criterion_group, criterion_main, Criterion};

// local imports
use hl::{
    DateTimeFormatter, Filter, IncludeExcludeKeyFilter, LinuxDateFormat, Parser, ParserSettings,
    RecordFormatter, SegmentProcesor, Settings, Theme,
};

// ---

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("parse-and-format");
    c.bench_function("kibana-record-01", |b| {
        let settings = Settings::default();
        let parser = Parser::new(ParserSettings::new(&settings.fields, false));
        let mut formatter = RecordFormatter::new(
            Arc::new(Theme::embedded("classic").unwrap()),
            DateTimeFormatter::new(
                LinuxDateFormat::new("%b %d %T.%3N").compile(),
                FixedOffset::east(0),
            ),
            false,
            Arc::new(IncludeExcludeKeyFilter::default()),
        );
        let filter = Filter::default();
        let mut processor = SegmentProcesor::new(&parser, &mut formatter, &filter);
        let mut buf = Vec::new();
        b.iter(|| {
            processor.run(KIBANA_RECORD_01, &mut buf);
            buf.clear();
        });
    });
}

// ---

const KIBANA_RECORD_01: &'static [u8] = concat_bytes!(br#"{"@timestamp":"2021-06-20T00:00:00.393Z","@version":"1","agent":{"ephemeral_id":"30ca3b53-1ef6-4699-8728-7754d1698a01","hostname":"as-rtrf-fileboat-ajjke","id":"1a9b51ef-ffbe-420e-a92c-4f653afff5aa","type":"fileboat","version":"7.8.3"},"koent-id":"1280e812-654f-4d04-a4f8-e6b84079920a","anchor":"oglsaash","caller":"example/demo.go:200","dc_name":"as-rtrf","ecs":{"version":"1.0.0"},"host":{"name":"as-rtrf-fileboat-ajjke"},"input":{"type":"docker"},"kubernetes":{"container":{"name":"some-segway"},"labels":{"app":"some-segway","component":"some-segway","pod-template-hash":"756d998476","release":"as-rtrf-some-segway","subcomponent":"some-segway"},"namespace":"as-rtrf","node":{"name":"as-rtrf-k8s-kube-node-vm01"},"pod":{"name":"as-rtrf-some-segway-platform-756d998476-jz4jm","uid":"9d445b65-fbf7-4d94-a7f4-4dbb7753d65c"},"replicaset":{"name":"as-rtrf-some-segway-platform-756d998476"}},"level":"info","localTime":"2021-06-19T23:59:58.450Z","log":{"file":{"path":"/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log"},"offset":34009140},"logger":"deep","msg":"io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1","server-uuid":"0a1bec7f-a252-4ff6-994a-1fbdca318d6d","slot":2,"stream":"stdout","task-id":"1a632cba-8480-4644-93f2-262bc0c13d04","tenant-id":"40ddb7cf-ce50-41e4-b994-408e393355c0","time":"2021-06-20T00:00:00.393Z","ts":"2021-06-19T23:59:58.449489225Z","type":"k8s_containers_logs","unit":"0"}"#, b"\n");

// ---

criterion_group!(benches, benchmark);
criterion_main!(benches);
