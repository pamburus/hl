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

mod encstr;
mod mem;
mod parse_and_format;
mod regex;
mod string;
mod theme;
mod ts_format;
mod ts_parse;
mod wildflower;
mod wildmatch;

criterion_main!(
    encstr::benches,
    mem::benches,
    parse_and_format::benches,
    regex::benches,
    string::benches,
    theme::benches,
    ts_format::benches,
    ts_parse::benches,
    wildflower::benches,
    wildmatch::benches
);

fn hash<T: Hash>(value: T) -> String {
    let mut hasher = FnvHasher::default();
    value.hash(&mut hasher);
    let hash = hasher.finish().to_be_bytes();
    base32::encode(Alphabet::Rfc4648Lower { padding: false }, &hash[..])
}
