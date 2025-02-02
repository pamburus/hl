// third-party imports
use criterion::criterion_main;
use stats_alloc::{Stats, StatsAlloc, INSTRUMENTED_SYSTEM};

mod lex;
mod parse;

#[global_allocator]
static GA: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn add_stat(lhs: &mut Stats, rhs: &Stats) {
    lhs.allocations += rhs.allocations;
    lhs.deallocations += rhs.deallocations;
    lhs.reallocations += rhs.reallocations;
    lhs.bytes_allocated += rhs.bytes_allocated;
    lhs.bytes_deallocated += rhs.bytes_deallocated;
    lhs.bytes_reallocated += rhs.bytes_reallocated;
}

const KIBANA_REC_1: &[u8] = br#"time=2021-06-20T00:00:00.393Z level=INFO msg="io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1" @version=1 agent.ephemeral_id=30ca3b53-1ef6-4699-8728-7754d1698a01 agent.hostname=as-rtrf-fileboat-ajjke agent.id=1a9b51ef-ffbe-420e-a92c-4f653afff5aa agent.type=fileboat agent.version=7.8.3 koent-id=1280e812-654f-4d04-a4f8-e6b84079920a anchor=oglsaash dc_name=as-rtrf ecs.version=1.0.0 host.name=as-rtrf-fileboat-ajjke input.type=docker kubernetes.container.name=some-segway kubernetes.labels.app=some-segway kubernetes.labels.component=some-segway kubernetes.labels.pod-template-hash=756d998476 kubernetes.labels.release=as-rtrf-some-segway kubernetes.labels.subcomponent=some-segway kubernetes.namespace=as-rtrf kubernetes.node.name=as-rtrf-k8s-kube-node-vm01 kubernetes.pod.name=as-rtrf-some-segway-platform-756d998476-jz4jm kubernetes.pod.uid=9d445b65-fbf7-4d94-a7f4-4dbb7753d65c kubernetes.replicaset.name=as-rtrf-some-segway-platform-756d998476 localTime=2021-06-19T23:59:58.450Z log.file.path=/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log log.offset=34009140 logger=deep server-uuid=0a1bec7f-a252-4ff6-994a-1fbdca318d6d slot=2 stream=stdout task-id=1a632cba-8480-4644-93f2-262bc0c13d04 tenant-id=40ddb7cf-ce50-41e4-b994-408e393355c0 type=k8s_containers_logs unit=0 source=example/demo.go:200"#;

criterion_main!(lex::benches, parse::benches);
