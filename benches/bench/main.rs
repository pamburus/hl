// std imports
use std::{
    alloc::System,
    hash::{Hash, Hasher},
};

// third-party imports
use base32::Alphabet;
use criterion::criterion_main;
use fnv::FnvHasher;
use stats_alloc::{StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GA: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const ND: &str = ":"; // name delimiter

mod encstr;
mod hl;
mod mem;
mod samples;
mod wildcard;

criterion_main!(encstr::benches, mem::benches, hl::benches, wildcard::benches);

fn hash<T: Hash>(value: T) -> String {
    let mut hasher = FnvHasher::default();
    value.hash(&mut hasher);
    let hash = hasher.finish().to_be_bytes();
    base32::encode(Alphabet::Rfc4648Lower { padding: false }, &hash[..])
}
