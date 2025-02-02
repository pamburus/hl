// third-party imports
use criterion::criterion_group;

// local imports
use super::{hash, samples, ND};

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

const GROUP: &str = "hl";
