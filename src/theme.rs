use std::collections::HashMap;
use std::vec::Vec;

use crate::eseq;
use crate::types;

use eseq::{
    eseq0, eseq1, eseq2, eseq3, Color, ColorCode::RGB, Style::Faint, Style::Reverse, StyleCode,
};
pub use types::Level;

#[repr(u8)]
pub enum Element {
    Time,
    Level,
    Logger,
    Caller,
    Message,
    EqualSign,
    Brace,
    Quote,
    Delimiter,
    Punctuation,
    FieldKey,
    LiteralNull,
    LiteralBoolean,
    LiteralNumber,
    LiteralString,
}

pub type Buf = Vec<u8>;

pub struct Styler<'a> {
    pack: &'a StylePack,
    current: Option<usize>,
}

pub struct Theme {
    packs: HashMap<Level, StylePack>,
    default: StylePack,
}

#[derive(Clone, Eq, PartialEq)]
struct Style(Buf);

impl Style {
    pub fn apply(&self, buf: &mut Buf) {
        buf.extend_from_slice(self.0.as_slice())
    }
}

impl From<Vec<u8>> for Style {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<StyleCode> for Style {
    fn from(value: StyleCode) -> Self {
        Self(eseq1(value))
    }
}

impl From<(StyleCode, StyleCode)> for Style {
    fn from(v: (StyleCode, StyleCode)) -> Self {
        Self(eseq2(v.0, v.1))
    }
}

impl From<(StyleCode, StyleCode, StyleCode)> for Style {
    fn from(v: (StyleCode, StyleCode, StyleCode)) -> Self {
        Self(eseq3(v.0, v.1, v.2))
    }
}

impl<'a> Styler<'a> {
    pub fn set(&mut self, buf: &mut Buf, e: Element) {
        self.set_style(buf, self.pack.elements[e as usize])
    }

    fn reset(&mut self, buf: &mut Buf) {
        self.set_style(buf, None)
    }

    fn set_style(&mut self, buf: &mut Buf, style: Option<usize>) {
        let style = match style {
            Some(style) => Some(style),
            None => self.pack.reset,
        };
        if let Some(style) = style {
            if self.current != Some(style) {
                self.current = Some(style);
                let style = &self.pack.styles[style];
                style.apply(buf);
            }
        }
    }
}

impl Theme {
    pub fn apply<'a, F: FnOnce(&mut Buf, &mut Styler<'a>)>(
        &'a self,
        buf: &mut Buf,
        level: &Option<Level>,
        f: F,
    ) {
        let mut styler = Styler {
            pack: match level {
                Some(level) => match self.packs.get(level) {
                    Some(pack) => pack,
                    None => &self.default,
                },
                None => &self.default,
            },
            current: None,
        };
        f(buf, &mut styler);
        styler.reset(buf)
    }
}

struct StylePack {
    elements: Vec<Option<usize>>,
    reset: Option<usize>,
    styles: Vec<Style>,
}

impl StylePack {
    fn new() -> Self {
        Self {
            styles: vec![Style(eseq0())],
            reset: Some(0),
            elements: vec![None; 255],
        }
    }

    fn none() -> Self {
        Self {
            elements: vec![None; 255],
            reset: None,
            styles: Vec::new(),
        }
    }

    fn add(&mut self, element: Element, style: &Style) {
        let pos = match self.styles.iter().position(|x| x == style) {
            Some(pos) => pos,
            None => {
                self.styles.push(style.clone());
                self.styles.len() - 1
            }
        };
        self.elements[element as usize] = Some(pos);
    }
}

impl Theme {
    pub fn none() -> Self {
        Self {
            packs: HashMap::new(),
            default: StylePack::none(),
        }
    }

    pub fn dark24() -> Self {
        let pack = |level| {
            let mut result = StylePack::new();
            let dark = RGB(92, 96, 100).fg().into();
            let medium = RGB(162, 185, 194).fg().into();
            let bright = RGB(255, 255, 255).fg().into();
            let orange = RGB(209, 154, 102).fg().into();
            let green = RGB(0, 175, 135).fg().into();
            let gray = RGB(153, 153, 153).fg().into();
            let yellow = RGB(255, 255, 175).fg().into();
            let blue = RGB(93, 175, 239).fg().into();
            result.add(Element::Time, &dark);
            result.add(Element::Level, &level);
            result.add(Element::Logger, &dark);
            result.add(Element::Caller, &dark);
            result.add(Element::Message, &bright);
            result.add(Element::FieldKey, &orange);
            result.add(Element::EqualSign, &dark);
            result.add(Element::Brace, &gray);
            result.add(Element::Quote, &green);
            result.add(Element::Delimiter, &gray);
            result.add(Element::Punctuation, &dark);
            result.add(Element::LiteralNull, &yellow);
            result.add(Element::LiteralBoolean, &yellow);
            result.add(Element::LiteralNumber, &blue);
            result.add(Element::LiteralString, &medium);
            result
        };
        let mut packs = HashMap::new();
        packs.insert(Level::Debug, pack(RGB(56, 119, 128).fg().into()));
        packs.insert(Level::Info, pack(RGB(86, 182, 194).fg().into()));
        packs.insert(Level::Warning, pack(RGB(255, 224, 128).fg().into()));
        packs.insert(Level::Error, pack(RGB(255, 128, 128).fg().into()));
        Self {
            packs,
            default: pack(RGB(56, 119, 128).fg().into()),
        }
    }

    pub fn dark() -> Self {
        let pack = |level| {
            let mut result = StylePack::new();
            let dark = Color::Black.bright().fg().into();
            let medium = eseq0().into();
            let bright = Color::White.bright().fg().into();
            let gray = (Faint.into(), Color::White.bright().fg()).into();
            let green = Color::Green.fg().into();
            let yellow = Color::Yellow.fg().into();
            let cyan = Color::Cyan.fg().into();
            result.add(Element::Time, &dark);
            result.add(Element::Level, &level);
            result.add(Element::Logger, &dark);
            result.add(Element::Caller, &dark);
            result.add(Element::Message, &bright);
            result.add(Element::FieldKey, &green);
            result.add(Element::EqualSign, &dark);
            result.add(Element::Brace, &gray);
            result.add(Element::Quote, &gray);
            result.add(Element::Delimiter, &gray);
            result.add(Element::Punctuation, &gray);
            result.add(Element::LiteralNull, &yellow);
            result.add(Element::LiteralBoolean, &yellow);
            result.add(Element::LiteralNumber, &cyan);
            result.add(Element::LiteralString, &medium);
            result
        };
        let mut packs = HashMap::new();
        packs.insert(Level::Debug, pack(Color::Magenta.fg().into()));
        packs.insert(Level::Info, pack(Color::Cyan.fg().into()));
        packs.insert(
            Level::Warning,
            pack((Reverse.into(), Color::Yellow.bright().fg()).into()),
        );
        packs.insert(
            Level::Error,
            pack((Reverse.into(), Color::Red.bright().fg()).into()),
        );
        Self {
            packs,
            default: pack(Color::Magenta.fg().into()),
        }
    }

    pub fn light() -> Self {
        let pack = |level| {
            let mut result = StylePack::new();
            let dark = Color::Black.bright().fg().into();
            let medium = eseq0().into();
            let bright = Color::Black.fg().into();
            let gray = Color::Black.bright().fg().into();
            let green = Color::Green.fg().into();
            let yellow = Color::Yellow.fg().into();
            let cyan = Color::Cyan.fg().into();
            result.add(Element::Time, &dark);
            result.add(Element::Level, &level);
            result.add(Element::Logger, &dark);
            result.add(Element::Caller, &dark);
            result.add(Element::Message, &bright);
            result.add(Element::FieldKey, &green);
            result.add(Element::EqualSign, &dark);
            result.add(Element::Brace, &gray);
            result.add(Element::Quote, &gray);
            result.add(Element::Delimiter, &gray);
            result.add(Element::Punctuation, &gray);
            result.add(Element::LiteralNull, &yellow);
            result.add(Element::LiteralBoolean, &yellow);
            result.add(Element::LiteralNumber, &cyan);
            result.add(Element::LiteralString, &medium);
            result
        };
        let mut packs = HashMap::new();
        packs.insert(Level::Debug, pack(Color::Magenta.fg().into()));
        packs.insert(Level::Info, pack(Color::Cyan.fg().into()));
        packs.insert(
            Level::Warning,
            pack(
                (
                    Reverse.into(),
                    Color::Yellow.bright().fg(),
                    Color::Black.bg(),
                )
                    .into(),
            ),
        );
        packs.insert(
            Level::Error,
            pack((Reverse.into(), Color::Red.bright().fg(), Color::Black.bg()).into()),
        );
        Self {
            packs,
            default: pack(Color::Magenta.fg().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme() {
        let theme = Theme::dark();
        let mut buf = Vec::new();
        theme.apply(&mut buf, &Some(Level::Debug), |buf, styler| {
            styler.set(buf, Element::Message);
        });
    }
}
