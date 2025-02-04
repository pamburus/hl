// third-party imports
use criterion::criterion_group;

mod combined;
mod theme;

criterion_group!(benches, combined::bench, theme::bench);

const GROUP: &str = "hl";
