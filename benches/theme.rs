// std imports
use std::{alloc::System, collections::HashMap};

// third-party imports
use criterion::{criterion_group, criterion_main, Criterion};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

// local imports
use hl::{
    settings::{self, Color, Mode, Style, StylePack},
    theme::{Element, StylingPush, Theme},
    types::Level,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn criterion_benchmark(c: &mut Criterion) {
    let theme = Theme::load(&settings::Theme {
        default: StylePack {
            time: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            level: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(25)),
                background: None,
            },
            logger: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            caller: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            message: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(255)),
                background: None,
            },
            equal_sign: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            brace: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(246)),
                background: None,
            },
            quote: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(246)),
                background: None,
            },
            delimiter: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            comma: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            at_sign: Style {
                modes: vec![Mode::Italic],
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            ellipsis: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(8)),
                background: None,
            },
            field_key: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(117)),
                background: None,
            },
            null: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(136)),
                background: None,
            },
            boolean: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(178)),
                background: None,
            },
            number: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(41)),
                background: None,
            },
            string: Style {
                modes: Vec::default(),
                foreground: Some(Color::Palette(36)),
                background: None,
            },
            whitespace: Style {
                modes: Vec::default(),
                foreground: None,
                background: None,
            },
        },
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
                s.element(Element::Delimiter, |s| s.batch(|buf| buf.push(b'|')));
                s.element(Element::Level, |s| {
                    s.batch(|buf| buf.extend_from_slice(b"INF"))
                });
                s.element(Element::Delimiter, |s| s.batch(|buf| buf.push(b'|')));
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Logger, |s| {
                    s.batch(|buf| {
                        buf.extend_from_slice(b"logger");
                        buf.push(b':');
                    })
                });
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Message, |s| {
                    s.batch(|buf| {
                        buf.extend_from_slice(b"hello!");
                    })
                });
                for _ in 0..4 {
                    for (key, value) in &fields {
                        s.batch(|buf| buf.push(b' '));
                        s.element(Element::FieldKey, |s| {
                            s.batch(|buf| {
                                buf.extend_from_slice(&key[..]);
                            })
                        });
                        s.element(Element::EqualSign, |s| s.batch(|buf| buf.push(b'=')));
                        s.element(Element::String, |s| {
                            s.batch(|buf| {
                                buf.extend_from_slice(&value[..]);
                            })
                        });
                    }
                }
                s.element(Element::AtSign, |s| {
                    s.batch(|buf| buf.extend_from_slice(b" @ "))
                });
                s.element(Element::Caller, |s| {
                    s.batch(|buf| buf.extend_from_slice(b"caller"))
                });
            });
            n1 += 1;
        });
        c1 = Some(reg.change());
    });
    println!("allocations at 1 ({:?} iterations): {:#?}", n1, c1);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
