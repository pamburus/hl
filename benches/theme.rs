// std imports
use std::{alloc::System, collections::HashMap};

// third-party imports
use criterion::{criterion_group, criterion_main, Criterion};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

// local imports
use hl::{
    theme::{Element, StylingPush, Theme},
    themecfg::{self, Color, Mode, Style},
    types::Level,
};

// ---

macro_rules! collection {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        Iterator::collect(IntoIterator::into_iter([$(($k, $v),)*]))
    }};
    // set-like
    ($($v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        Iterator::collect(IntoIterator::into_iter([$($v,)*]))
    }};
}

// ---

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

// ---

fn benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("theme");
    let theme = Theme::from(&themecfg::Theme {
        default: HashMap::from(collection! {
            Element::Time => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            Element::Level => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(25)),
                background: None,
            },
            Element::Logger => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            Element::Caller => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            Element::Message => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(255)),
                background: None,
            },
            Element::Field => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            Element::Object => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(246)),
                background: None,
            },
            Element::Array => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(246)),
                background: None,
            },
            Element::Ellipsis => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            Element::Key => Style {
                modes: vec![Mode::Underline],
                foreground: Some(Color::Palette(117)),
                background: None,
            },
            Element::Null => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(136)),
                background: None,
            },
            Element::Boolean => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(178)),
                background: None,
            },
            Element::Number => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(41)),
                background: None,
            },
            Element::String => Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(36)),
                background: None,
            },
        })
        .into(),
        levels: HashMap::new(),
    });
    let fields = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
        (b"key5", b"value5"),
        (b"key6", b"value6"),
        (b"key7", b"value7"),
    ];
    let mut buf = Vec::with_capacity(8192);

    let mut c1 = None;
    let mut n1 = 0;
    c.bench_function("theme", |b| {
        let reg = Region::new(&GLOBAL);
        b.iter(|| {
            buf.clear();
            theme.apply(&mut buf, &Some(Level::Debug), |s| {
                s.element(Element::Time, |s| {
                    s.batch(|buf| buf.extend_from_slice(b"2020-01-01 00:00:00"))
                });
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Level, |s| {
                    s.batch(|buf| buf.push(b'|'));
                    s.element(Element::LevelInner, |s| {
                        s.batch(|buf| buf.extend_from_slice(b"INF"))
                    });
                    s.batch(|buf| buf.push(b'|'))
                });
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Logger, |s| {
                    s.element(Element::LoggerInner, |s| {
                        s.batch(|buf| buf.extend_from_slice(b"logger"))
                    });
                    s.batch(|buf| buf.push(b':'))
                });
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Message, |s| {
                    s.batch(|buf| {
                        buf.extend_from_slice(b"hello!");
                    })
                });
                for _ in 0..4 {
                    for (key, value) in &fields {
                        s.element(Element::Field, |s| {
                            s.batch(|buf| buf.push(b' '));
                            s.element(Element::Key, |s| {
                                s.batch(|buf| buf.extend_from_slice(&key[..]))
                            });
                            s.batch(|buf| buf.push(b'='));
                            s.element(Element::String, |s| {
                                s.batch(|buf| buf.extend_from_slice(&value[..]))
                            });
                        });
                    }
                }
                s.element(Element::Caller, |s| {
                    s.batch(|buf| buf.extend_from_slice(b" @ "));
                    s.element(Element::CallerInner, |s| {
                        s.batch(|buf| buf.extend_from_slice(b"caller"))
                    });
                });
            });
            n1 += 1;
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);
}

// ---

criterion_group!(benches, benchmark);
criterion_main!(benches);
