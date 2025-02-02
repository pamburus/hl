// std imports
use std::alloc::System;

// third-party imports
use criterion::criterion_main;
use stats_alloc::{StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GA: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

mod encstr;
mod json;
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
    json::benches,
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
