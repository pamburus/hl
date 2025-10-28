use crate::format::json;
use logos::Logos;

use super::*;

#[test]
fn test_builder() {
    let settings = Settings::default();
    let mut container = ast::Container::new();
    let mut pc = PriorityController::default();
    let mut record = Record::default();
    let b = Builder::new(&settings, &mut pc, &mut record, container.metaroot());
    b.add_scalar(Scalar::Bool(true))
        .add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Bool(false)), Ok(())))
        .1
        .unwrap();
    assert_eq!(container.nodes().len(), 3);
}

#[test]
fn test_builder_json() {
    let mut container = ast::Container::new();
    let mut pc = PriorityController::default();
    let settings = Settings::new(&PredefinedFields::default()).with_ignore(["kubernetes", "agent"]);
    let mut record = Record::default();
    json::parse_all_into(
        &mut json::Token::lexer(KIBANA_REC_1),
        Builder::new(&settings, &mut pc, &mut record, container.metaroot()),
    )
    .1
    .unwrap();

    assert_eq!(container.roots().len(), 1);
    assert_eq!(container.roots().iter().next().unwrap().children().len(), 22);
    assert_eq!(container.nodes().len(), 57);

    println!("{:?}", container);

    record.ast = container;
    assert_eq!(record.predefined.len(), 5);
    assert_eq!(record.fields_for_search().into_iter().count(), 22);
    assert_eq!(record.fields().into_iter().count(), 17);
}

const KIBANA_REC_1: &str = r#"{"@timestamp":"2021-06-20T00:00:00.393Z","@version":"1","agent":{"ephemeral_id":"30ca3b53-1ef6-4699-8728-7754d1698a01","hostname":"as-rtrf-fileboat-ajjke","id":"1a9b51ef-ffbe-420e-a92c-4f653afff5aa","type":"fileboat","version":"7.8.3"},"koent-id":"1280e812-654f-4d04-a4f8-e6b84079920a","anchor":"oglsaash","caller":"example/demo.go:200","dc_name":"as-rtrf","ecs":{"version":"1.0.0"},"host":{"name":"as-rtrf-fileboat-ajjke"},"input":{"type":"docker"},"kubernetes":{"container":{"name":"some-segway"},"labels":{"app":"some-segway","component":"some-segway","pod-template-hash":"756d998476","release":"as-rtrf-some-segway","subcomponent":"some-segway"},"namespace":"as-rtrf","node":{"name":"as-rtrf-k8s-kube-node-vm01"},"pod":{"name":"as-rtrf-some-segway-platform-756d998476-jz4jm","uid":"9d445b65-fbf7-4d94-a7f4-4dbb7753d65c"},"replicaset":{"name":"as-rtrf-some-segway-platform-756d998476"}},"level":"info","localTime":"2021-06-19T23:59:58.450Z","log":{"file":{"path":"/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log"},"offset":34009140},"logger":"deep","msg":"io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1","server-uuid":"0a1bec7f-a252-4ff6-994a-1fbdca318d6d","slot":2,"stream":"stdout","task-id":"1a632cba-8480-4644-93f2-262bc0c13d04","tenant-id":"40ddb7cf-ce50-41e4-b994-408e393355c0","time":"2021-06-20T00:00:00.393Z","ts":"2021-06-19T23:59:58.449489225Z","type":"k8s_containers_logs","unit":"0"}"#;
