// third-party imports
use const_str::concat as strcat;
use criterion::criterion_group;

// local imports
use super::{hash, samples, BencherExt, ND};

mod combined;
mod theme;
mod timestamp;

criterion_group!(
    benches,
    combined::bench,
    theme::bench,
    timestamp::parsing::bench,
    timestamp::formatting::bench
);

const GROUP: &str = strcat!(super::GROUP, ND, "hl");
