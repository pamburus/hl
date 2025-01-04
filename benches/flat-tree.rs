// std imports
use std::{alloc::System, hint::black_box, time::Duration};

// third-party imports
use criterion::*;
use logos::Logos;
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

// workspace imports
use hl::{
    format::json::{parse_all_into, Token},
    model::v2::ast,
};

criterion_group!(benches, container);

fn add_stat(lhs: &mut Stats, rhs: &Stats) {
    lhs.allocations += rhs.allocations;
    lhs.deallocations += rhs.deallocations;
    lhs.reallocations += rhs.reallocations;
    lhs.bytes_allocated += rhs.bytes_allocated;
    lhs.bytes_deallocated += rhs.bytes_deallocated;
    lhs.bytes_reallocated += rhs.bytes_reallocated;
}

fn container(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat-tree");
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(2));

    let mut iterations = 0;
    let mut allocs = Stats::default();
    let test = "parse-to-container/kibana";

    group.bench_function(test, |b| {
        let mut container = ast::Container::new();
        container.reserve(512);

        b.iter(|| {
            let reg = Region::new(&GLOBAL);

            container.clear();
            let mut lexer = Token::lexer(KIBANA_REC_1);

            black_box(parse_all_into(&mut lexer, container.metaroot())).unwrap();

            add_stat(&mut allocs, &reg.change());
            iterations += 1;
        });
    });

    println!("{}: allocations per {:?} iterations: {:#?}", test, iterations, allocs);

    let mut iterations = 0;
    let mut allocs = Stats::default();
    let test = "drain-tokens/kibana";

    group.bench_function(test, |b| {
        b.iter(|| {
            let reg = Region::new(&GLOBAL);

            let mut lexer = Token::lexer(KIBANA_REC_1);
            while let Some(_) = black_box(lexer.next()) {}

            add_stat(&mut allocs, &reg.change());
            iterations += 1;
        });
    });

    println!("{}: allocations per {:?} iterations: {:#?}", test, iterations, allocs);

    group.finish();
}

const KIBANA_REC_1: &str = r#"{"@timestamp":"2021-06-20T00:00:00.393Z","@version":"1","agent":{"ephemeral_id":"30ca3b53-1ef6-4699-8728-7754d1698a01","hostname":"as-rtrf-fileboat-ajjke","id":"1a9b51ef-ffbe-420e-a92c-4f653afff5aa","type":"fileboat","version":"7.8.3"},"koent-id":"1280e812-654f-4d04-a4f8-e6b84079920a","anchor":"oglsaash","caller":"example/demo.go:200","dc_name":"as-rtrf","ecs":{"version":"1.0.0"},"host":{"name":"as-rtrf-fileboat-ajjke"},"input":{"type":"docker"},"kubernetes":{"container":{"name":"some-segway"},"labels":{"app":"some-segway","component":"some-segway","pod-template-hash":"756d998476","release":"as-rtrf-some-segway","subcomponent":"some-segway"},"namespace":"as-rtrf","node":{"name":"as-rtrf-k8s-kube-node-vm01"},"pod":{"name":"as-rtrf-some-segway-platform-756d998476-jz4jm","uid":"9d445b65-fbf7-4d94-a7f4-4dbb7753d65c"},"replicaset":{"name":"as-rtrf-some-segway-platform-756d998476"}},"level":"info","localTime":"2021-06-19T23:59:58.450Z","log":{"file":{"path":"/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log"},"offset":34009140},"logger":"deep","msg":"io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1","server-uuid":"0a1bec7f-a252-4ff6-994a-1fbdca318d6d","slot":2,"stream":"stdout","task-id":"1a632cba-8480-4644-93f2-262bc0c13d04","tenant-id":"40ddb7cf-ce50-41e4-b994-408e393355c0","time":"2021-06-20T00:00:00.393Z","ts":"2021-06-19T23:59:58.449489225Z","type":"k8s_containers_logs","unit":"0"}
"#;

criterion_main!(benches);
