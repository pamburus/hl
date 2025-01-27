// std imports
use std::sync::Arc;

// third-party imports
use byte_strings::concat_bytes;
use chrono::{Offset, Utc};
use criterion::{criterion_group, criterion_main, Criterion};

// local imports
use hl::{
    format,
    formatting::v2::{RecordFormatter, RecordWithSourceFormatter},
    model::v2::{
        parse::{NewParser, Parser},
        record::{filter::CombinedFilter, RecordWithSource, Settings as ParserSettings},
    },
    processing::{RecordIgnorer, SegmentProcess, SegmentProcessor, SegmentProcessorOptions},
    settings,
    timezone::Tz,
    DateTimeFormatter, IncludeExcludeKeyFilter, LinuxDateFormat, Settings, Theme,
};

// ---

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse-format-v2");

    for (name, record) in [
        ("kibana-record-01-json-x1024", KIBANA_RECORD_01_JSON.repeat(1024)),
        ("kibana-record-01-logfmt", KIBANA_RECORD_01_LOGFMT.repeat(1024)),
    ] {
        for theme in ["universal"] {
            group.bench_function(format!("parse-and-format/{}/{}", name, theme), |b| {
                let settings = Settings::default();
                let parser = ParserSettings::new(&settings.fields.predefined);
                let formatter = RecordFormatter::new(
                    Arc::new(Theme::embedded(theme).unwrap()),
                    DateTimeFormatter::new(
                        LinuxDateFormat::new("%b %d %T.%3N").compile(),
                        Tz::FixedOffset(Utc.fix()),
                    ),
                    false,
                    Arc::new(IncludeExcludeKeyFilter::default()),
                    settings::Formatting {
                        flatten: Some(settings::FlattenOption::Always),
                        ..Default::default()
                    },
                )
                .with_flatten(true);
                let filter = CombinedFilter::default();
                let mut processor =
                    SegmentProcessor::new(&parser, &formatter, &filter, SegmentProcessorOptions::default());
                let mut buf = Vec::new();
                b.iter(|| {
                    processor.process(&record, &mut buf, "", None, &mut RecordIgnorer {});
                    buf.clear();
                });
            });
        }
    }

    for (name, record) in [
        ("kibana-record-01-json-x1024", KIBANA_RECORD_01_JSON.repeat(1024)),
        ("kibana-record-01-logfmt", KIBANA_RECORD_01_LOGFMT.repeat(1024)),
    ] {
        group.bench_function(format!("parse-only/{}", name), |b| {
            let settings = Settings::default();
            let parser = ParserSettings::new(&settings.fields.predefined);
            let filter = CombinedFilter::default();
            let mut processor =
                SegmentProcessor::new(&parser, NoFormatter, &filter, SegmentProcessorOptions::default());
            let mut buf = Vec::new();
            b.iter(|| {
                processor.process(&record, &mut buf, "", None, &mut RecordIgnorer {});
                buf.clear();
            });
        });
    }
}

struct NoFormatter;

impl RecordWithSourceFormatter for NoFormatter {
    #[inline]
    fn format_record(&self, _: &mut Vec<u8>, _: RecordWithSource) {}
}

// ---

const KIBANA_RECORD_01_JSON: &'static [u8] = concat_bytes!(br#"{"@timestamp":"2021-06-20T00:00:00.393Z","@version":"1","agent":{"ephemeral_id":"30ca3b53-1ef6-4699-8728-7754d1698a01","hostname":"as-rtrf-fileboat-ajjke","id":"1a9b51ef-ffbe-420e-a92c-4f653afff5aa","type":"fileboat","version":"7.8.3"},"koent-id":"1280e812-654f-4d04-a4f8-e6b84079920a","anchor":"oglsaash","caller":"example/demo.go:200","dc_name":"as-rtrf","ecs":{"version":"1.0.0"},"host":{"name":"as-rtrf-fileboat-ajjke"},"input":{"type":"docker"},"kubernetes":{"container":{"name":"some-segway"},"labels":{"app":"some-segway","component":"some-segway","pod-template-hash":"756d998476","release":"as-rtrf-some-segway","subcomponent":"some-segway"},"namespace":"as-rtrf","node":{"name":"as-rtrf-k8s-kube-node-vm01"},"pod":{"name":"as-rtrf-some-segway-platform-756d998476-jz4jm","uid":"9d445b65-fbf7-4d94-a7f4-4dbb7753d65c"},"replicaset":{"name":"as-rtrf-some-segway-platform-756d998476"}},"level":"info","localTime":"2021-06-19T23:59:58.450Z","log":{"file":{"path":"/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log"},"offset":34009140},"logger":"deep","msg":"io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1","server-uuid":"0a1bec7f-a252-4ff6-994a-1fbdca318d6d","slot":2,"stream":"stdout","task-id":"1a632cba-8480-4644-93f2-262bc0c13d04","tenant-id":"40ddb7cf-ce50-41e4-b994-408e393355c0","time":"2021-06-20T00:00:00.393Z","ts":"2021-06-19T23:59:58.449489225Z","type":"k8s_containers_logs","unit":"0"}"#, b"\n");

const KIBANA_RECORD_01_LOGFMT: &'static [u8] = concat_bytes!(br#"time=2021-06-20T00:00:00.393Z level=INFO msg="io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1" @version=1 agent.ephemeral_id=30ca3b53-1ef6-4699-8728-7754d1698a01 agent.hostname=as-rtrf-fileboat-ajjke agent.id=1a9b51ef-ffbe-420e-a92c-4f653afff5aa agent.type=fileboat agent.version=7.8.3 koent-id=1280e812-654f-4d04-a4f8-e6b84079920a anchor=oglsaash dc_name=as-rtrf ecs.version=1.0.0 host.name=as-rtrf-fileboat-ajjke input.type=docker kubernetes.container.name=some-segway kubernetes.labels.app=some-segway kubernetes.labels.component=some-segway kubernetes.labels.pod-template-hash=756d998476 kubernetes.labels.release=as-rtrf-some-segway kubernetes.labels.subcomponent=some-segway kubernetes.namespace=as-rtrf kubernetes.node.name=as-rtrf-k8s-kube-node-vm01 kubernetes.pod.name=as-rtrf-some-segway-platform-756d998476-jz4jm kubernetes.pod.uid=9d445b65-fbf7-4d94-a7f4-4dbb7753d65c kubernetes.replicaset.name=as-rtrf-some-segway-platform-756d998476 localTime=2021-06-19T23:59:58.450Z log.file.path=/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log log.offset=34009140 logger=deep server-uuid=0a1bec7f-a252-4ff6-994a-1fbdca318d6d slot=2 stream=stdout task-id=1a632cba-8480-4644-93f2-262bc0c13d04 tenant-id=40ddb7cf-ce50-41e4-b994-408e393355c0 type=k8s_containers_logs unit=0 source=example/demo.go:200"#, b"\n");

// ---

criterion_group!(benches, benchmark);
criterion_main!(benches);
