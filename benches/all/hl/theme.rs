// std imports
use std::{collections::HashMap, hint::black_box, time::Duration};

// third-party imports
use collection_macros::hashmap;
use const_str::concat as strcat;
use criterion::Criterion;

// local imports
use super::ND;
use hl::{
    theme::{Element, StylingPush, Theme},
    themecfg::{self, Color, Mode, Style},
    Level,
};

const GROUP: &str = strcat!(super::GROUP, ND, "theme");

pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);
    c.warm_up_time(Duration::from_secs(1));
    c.measurement_time(Duration::from_secs(3));

    let theme = theme();
    let fields = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
        (b"key5", b"value5"),
        (b"key6", b"value6"),
        (b"key7", b"value7"),
    ];

    c.bench_function("apply", |b| {
        let setup = || Vec::with_capacity(4096);
        b.iter_with_setup(setup, |mut buf: Vec<u8>| {
            black_box(&theme).apply(&mut buf, &Some(Level::Debug), |s| {
                s.element(Element::Time, |s| {
                    s.batch(|buf| buf.extend_from_slice(b"2020-01-01 00:00:00"))
                });
                s.batch(|buf| buf.push(b' '));
                s.element(Element::Level, |s| {
                    s.batch(|buf| buf.push(b'|'));
                    s.element(Element::LevelInner, |s| s.batch(|buf| buf.extend_from_slice(b"INF")));
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
                    for (key, value) in black_box(&fields) {
                        s.element(Element::Field, |s| {
                            s.batch(|buf| buf.push(b' '));
                            s.element(Element::Key, |s| s.batch(|buf| buf.extend_from_slice(&key[..])));
                            s.batch(|buf| buf.push(b'='));
                            s.element(Element::String, |s| s.batch(|buf| buf.extend_from_slice(&value[..])));
                        })
                    }
                }
                s.element(Element::Caller, |s| {
                    s.batch(|buf| buf.extend_from_slice(b" @ "));
                    s.element(Element::CallerInner, |s| {
                        s.batch(|buf| buf.extend_from_slice(b"caller"))
                    })
                })
            })
        })
    });
}

fn theme() -> Theme {
    Theme::from(&themecfg::Theme {
        elements: hashmap! {
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
        }
        .into(),
        levels: HashMap::new(),
        indicators: themecfg::IndicatorPack::default(),
    })
}
