// std imports
use std::{collections::HashMap, hint::black_box, time::Duration};

// third-party imports
use collection_macros::hashmap;
use const_str::concat as strcat;
use criterion::{BatchSize, Criterion};

// local imports
use super::{BencherExt, ND};
use hl::{
    Level,
    theme::{Element, StylingPush, Theme},
    themecfg::{self, Color, Mode, RawStyle, Role, ThemeVersion},
};

const GROUP: &str = strcat!(super::GROUP, ND, "theme");

pub(super) fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);
    c.warm_up_time(Duration::from_secs(1));
    c.measurement_time(Duration::from_secs(3));

    let theme = theme();
    let fields = || {
        vec![
            (b"key1", b"value1"),
            (b"key2", b"value2"),
            (b"key3", b"value3"),
            (b"key4", b"value4"),
            (b"key5", b"value5"),
            (b"key6", b"value6"),
            (b"key7", b"value7"),
        ]
    };

    c.bench_function("apply", |b| {
        let setup = || (Vec::with_capacity(1024), fields());
        b.iter_batched_ref_fixed(
            setup,
            |(buf, fields)| {
                black_box(&theme).apply(buf, &Some(Level::Debug), |s| {
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
                        for (key, value) in fields.iter() {
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
            },
            BatchSize::SmallInput,
        )
    });
}

fn theme() -> Theme {
    let cfg = themecfg::RawTheme {
        version: ThemeVersion::CURRENT,
        tags: Default::default(),
        styles: themecfg::v1::StylePack(hashmap! {
            Role::Primary => RawStyle::new().foreground(Some(Color::Palette(36))),
            Role::Secondary => RawStyle::new().foreground(Some(Color::Palette(8))),
            Role::Strong => RawStyle::new().foreground(Some(Color::Palette(255))),
            Role::Syntax => RawStyle::new().foreground(Some(Color::Palette(246))),
            Role::Accent => RawStyle::new().modes(Mode::Underline.into()).foreground(Some(Color::Palette(8))),
        }),
        elements: themecfg::v1::StylePack(hashmap! {
            Element::Time => Role::Secondary.into(),
            Element::Level => RawStyle::new().foreground(Some(Color::Palette(25))),
            Element::Logger => Role::Secondary.into(),
            Element::Caller => Role::Secondary.into(),
            Element::Message => Role::Strong.into(),
            Element::Field => Role::Secondary.into(),
            Element::Object => Role::Syntax.into(),
            Element::Array => Role::Syntax.into(),
            Element::Ellipsis => Role::Secondary.into(),
            Element::Key => Role::Accent.into(),
            Element::Null => RawStyle::new().foreground(Some(Color::Palette(136))),
            Element::Boolean => RawStyle::new().foreground(Some(Color::Palette(178))),
            Element::Number => RawStyle::new().foreground(Some(Color::Palette(41))),
            Element::String => Role::Primary.into(),
        }),
        levels: HashMap::new(),
        indicators: themecfg::v1::IndicatorPack::default(),
    };
    Theme::from(cfg.resolve().unwrap())
}
