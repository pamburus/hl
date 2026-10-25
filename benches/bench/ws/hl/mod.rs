// third-party imports
use const_str::concat as strcat;
use criterion::criterion_group;

// local imports
use super::{BencherExt, ND, hash, samples};

mod combined;
mod delimiter;
mod follow;
mod theme;
mod timestamp;

criterion_group!(
    benches,
    combined::bench,
    delimiter::bench,
    follow::bench,
    theme::bench,
    timestamp::parsing::bench,
    timestamp::formatting::bench
);

const GROUP: &str = strcat!(super::GROUP, ND, "hl");
