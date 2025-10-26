// std imports
use std::sync::Arc;

// third-party imports
use byte_strings::concat_bytes;
use chrono::{Offset, Utc};

// local imports
use crate::{
    DateTimeFormatter, IncludeExcludeKeyFilter, LinuxDateFormat, Settings, Theme,
    formatting::v2::{RecordFormatter, RecordFormatterBuilder},
    model::v2::record::{Settings as ParserSettings, filter::CombinedFilter},
    processing::{RecordIgnorer, SegmentProcess, SegmentProcessor, SegmentProcessorOptions},
    settings,
    timezone::Tz,
};

// ---

#[test]
fn test_parse_and_format() {
    let settings = Settings::default();
    let parser = ParserSettings::new(&settings.fields.predefined);
    let formatter = RecordFormatterBuilder::new()
        .with_theme(Arc::new(Theme::embedded("universal").unwrap()))
        .with_timestamp_formatter(DateTimeFormatter::new(
            LinuxDateFormat::new("%b %d %T.%3N").compile(),
            Tz::FixedOffset(Utc.fix()),
        ))
        .with_flatten(true)
        .build();
    let filter = CombinedFilter::default();
    let mut processor = SegmentProcessor::new(&parser, &formatter, &filter, SegmentProcessorOptions::default());
    let mut buf = Vec::new();
    processor.process(KIBANA_RECORD_01_JSON, &mut buf, "", None, &mut RecordIgnorer {});
    assert_ne!(buf.len(), 0);
    assert_eq!(
        std::str::from_utf8(&buf).unwrap(),
        "\u{1b}[0;2mJun 19 23:59:58.449 \u{1b}[0;2;39m|\u{1b}[0;36mINF\u{1b}[0;2;39m|\u{1b}[0;2m deep:\u{1b}[0m \u{1b}[0;1;39mio#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1 \u{1b}[0;2m:: \u{1b}[0;32m@version\u{1b}[0;2m=\u{1b}[0;39m\"1\" \u{1b}[0;32magent.ephemeral-id\u{1b}[0;2m=\u{1b}[0;39m30ca3b53-1ef6-4699-8728-7754d1698a01 \u{1b}[0;32magent.hostname\u{1b}[0;2m=\u{1b}[0;39mas-rtrf-fileboat-ajjke \u{1b}[0;32magent.id\u{1b}[0;2m=\u{1b}[0;39m1a9b51ef-ffbe-420e-a92c-4f653afff5aa \u{1b}[0;32magent.type\u{1b}[0;2m=\u{1b}[0;39mfileboat \u{1b}[0;32magent.version\u{1b}[0;2m=\u{1b}[0;39m7.8.3 \u{1b}[0;32mkoent-id\u{1b}[0;2m=\u{1b}[0;39m1280e812-654f-4d04-a4f8-e6b84079920a \u{1b}[0;32manchor\u{1b}[0;2m=\u{1b}[0;39moglsaash \u{1b}[0;32mdc-name\u{1b}[0;2m=\u{1b}[0;39mas-rtrf \u{1b}[0;32mecs.version\u{1b}[0;2m=\u{1b}[0;39m1.0.0 \u{1b}[0;32mhost.name\u{1b}[0;2m=\u{1b}[0;39mas-rtrf-fileboat-ajjke \u{1b}[0;32minput.type\u{1b}[0;2m=\u{1b}[0;39mdocker \u{1b}[0;32mkubernetes.container.name\u{1b}[0;2m=\u{1b}[0;39msome-segway \u{1b}[0;32mkubernetes.labels.app\u{1b}[0;2m=\u{1b}[0;39msome-segway \u{1b}[0;32mkubernetes.labels.component\u{1b}[0;2m=\u{1b}[0;39msome-segway \u{1b}[0;32mkubernetes.labels.pod-template-hash\u{1b}[0;2m=\u{1b}[0;39m756d998476 \u{1b}[0;32mkubernetes.labels.release\u{1b}[0;2m=\u{1b}[0;39mas-rtrf-some-segway \u{1b}[0;32mkubernetes.labels.subcomponent\u{1b}[0;2m=\u{1b}[0;39msome-segway \u{1b}[0;32mkubernetes.namespace\u{1b}[0;2m=\u{1b}[0;39mas-rtrf \u{1b}[0;32mkubernetes.node.name\u{1b}[0;2m=\u{1b}[0;39mas-rtrf-k8s-kube-node-vm01 \u{1b}[0;32mkubernetes.pod.name\u{1b}[0;2m=\u{1b}[0;39mas-rtrf-some-segway-platform-756d998476-jz4jm \u{1b}[0;32mkubernetes.pod.uid\u{1b}[0;2m=\u{1b}[0;39m9d445b65-fbf7-4d94-a7f4-4dbb7753d65c \u{1b}[0;32mkubernetes.replicaset.name\u{1b}[0;2m=\u{1b}[0;39mas-rtrf-some-segway-platform-756d998476 \u{1b}[0;32mlocalTime\u{1b}[0;2m=\u{1b}[0;39m2021-06-19T23:59:58.450Z \u{1b}[0;32mlog.file.path\u{1b}[0;2m=\u{1b}[0;39m/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log \u{1b}[0;32mlog.offset\u{1b}[0;2m=\u{1b}[0;94m34009140 \u{1b}[0;32mserver-uuid\u{1b}[0;2m=\u{1b}[0;39m0a1bec7f-a252-4ff6-994a-1fbdca318d6d \u{1b}[0;32mslot\u{1b}[0;2m=\u{1b}[0;94m2 \u{1b}[0;32mstream\u{1b}[0;2m=\u{1b}[0;39mstdout \u{1b}[0;32mtask-id\u{1b}[0;2m=\u{1b}[0;39m1a632cba-8480-4644-93f2-262bc0c13d04 \u{1b}[0;32mtenant-id\u{1b}[0;2m=\u{1b}[0;39m40ddb7cf-ce50-41e4-b994-408e393355c0 \u{1b}[0;32mtype\u{1b}[0;2m=\u{1b}[0;39mk8s_containers_logs \u{1b}[0;32munit\u{1b}[0;2m=\u{1b}[0;39m\"0\"\u{1b}[0;2;3m @ example/demo.go:200\u{1b}[0m\n"
    );
}

// ---

const KIBANA_RECORD_01_JSON: &'static [u8] = concat_bytes!(br#"{"@timestamp":"2021-06-20T00:00:00.393Z","@version":"1","agent":{"ephemeral_id":"30ca3b53-1ef6-4699-8728-7754d1698a01","hostname":"as-rtrf-fileboat-ajjke","id":"1a9b51ef-ffbe-420e-a92c-4f653afff5aa","type":"fileboat","version":"7.8.3"},"koent-id":"1280e812-654f-4d04-a4f8-e6b84079920a","anchor":"oglsaash","caller":"example/demo.go:200","dc_name":"as-rtrf","ecs":{"version":"1.0.0"},"host":{"name":"as-rtrf-fileboat-ajjke"},"input":{"type":"docker"},"kubernetes":{"container":{"name":"some-segway"},"labels":{"app":"some-segway","component":"some-segway","pod-template-hash":"756d998476","release":"as-rtrf-some-segway","subcomponent":"some-segway"},"namespace":"as-rtrf","node":{"name":"as-rtrf-k8s-kube-node-vm01"},"pod":{"name":"as-rtrf-some-segway-platform-756d998476-jz4jm","uid":"9d445b65-fbf7-4d94-a7f4-4dbb7753d65c"},"replicaset":{"name":"as-rtrf-some-segway-platform-756d998476"}},"level":"info","localTime":"2021-06-19T23:59:58.450Z","log":{"file":{"path":"/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log"},"offset":34009140},"logger":"deep","msg":"io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1","server-uuid":"0a1bec7f-a252-4ff6-994a-1fbdca318d6d","slot":2,"stream":"stdout","task-id":"1a632cba-8480-4644-93f2-262bc0c13d04","tenant-id":"40ddb7cf-ce50-41e4-b994-408e393355c0","time":"2021-06-20T00:00:00.393Z","ts":"2021-06-19T23:59:58.449489225Z","type":"k8s_containers_logs","unit":"0"}"#, b"\n");

const KIBANA_RECORD_01_LOGFMT: &'static [u8] = concat_bytes!(br#"time=2021-06-20T00:00:00.393Z level=INFO msg="io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1" @version=1 agent.ephemeral_id=30ca3b53-1ef6-4699-8728-7754d1698a01 agent.hostname=as-rtrf-fileboat-ajjke agent.id=1a9b51ef-ffbe-420e-a92c-4f653afff5aa agent.type=fileboat agent.version=7.8.3 koent-id=1280e812-654f-4d04-a4f8-e6b84079920a anchor=oglsaash dc_name=as-rtrf ecs.version=1.0.0 host.name=as-rtrf-fileboat-ajjke input.type=docker kubernetes.container.name=some-segway kubernetes.labels.app=some-segway kubernetes.labels.component=some-segway kubernetes.labels.pod-template-hash=756d998476 kubernetes.labels.release=as-rtrf-some-segway kubernetes.labels.subcomponent=some-segway kubernetes.namespace=as-rtrf kubernetes.node.name=as-rtrf-k8s-kube-node-vm01 kubernetes.pod.name=as-rtrf-some-segway-platform-756d998476-jz4jm kubernetes.pod.uid=9d445b65-fbf7-4d94-a7f4-4dbb7753d65c kubernetes.replicaset.name=as-rtrf-some-segway-platform-756d998476 localTime=2021-06-19T23:59:58.450Z log.file.path=/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log log.offset=34009140 logger=deep server-uuid=0a1bec7f-a252-4ff6-994a-1fbdca318d6d slot=2 stream=stdout task-id=1a632cba-8480-4644-93f2-262bc0c13d04 tenant-id=40ddb7cf-ce50-41e4-b994-408e393355c0 type=k8s_containers_logs unit=0 source=example/demo.go:200"#, b"\n");

// ---
