// std imports
use std::{borrow::Borrow, collections::HashMap, sync::Arc, vec::Vec};

// third-party imports
use enum_map::EnumMap;

// local imports
use crate::{
    appdirs::AppDirs,
    error::*,
    eseq::{Brightness, Color, ColorCode, Mode, Sequence, StyleCode},
    fmtx::Push,
    level::{self, InfallibleLevel},
    themecfg,
};

// test imports
#[cfg(test)]
use crate::testing::Sample;

// ---

pub use level::Level;
pub use themecfg::{Element, ThemeInfo, ThemeOrigin};

// ---

pub trait StylingPush<B: Push<u8>> {
    fn element<R, F: FnOnce(&mut Self) -> R>(&mut self, element: Element, f: F) -> R;
    fn batch<R, F: FnOnce(&mut B) -> R>(&mut self, f: F) -> R;
    fn space(&mut self);
    fn reset(&mut self);
}

// ---

#[derive(Default)]
pub struct Theme {
    packs: EnumMap<Level, StylePack>,
    default: StylePack,
    pub indicators: IndicatorPack,
}

impl Theme {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn load(app_dirs: &AppDirs, name: &str) -> Result<Self> {
        Ok(themecfg::Theme::load(app_dirs, name)?.into())
    }

    pub fn embedded(name: &str) -> Result<Self> {
        Ok(themecfg::Theme::embedded(name)?.into())
    }

    pub fn list(app_dirs: &AppDirs) -> Result<HashMap<Arc<str>, ThemeInfo>> {
        Ok(themecfg::Theme::list(app_dirs)?)
    }

    pub fn apply<'a, B: Push<u8>, F: FnOnce(&mut Styler<'a, B>)>(
        &'a self,
        buf: &'a mut B,
        level: &Option<Level>,
        f: F,
    ) {
        let mut styler = Styler {
            buf,
            pack: match level {
                Some(level) => &self.packs[*level],
                None => &self.default,
            },
            synced: None,
            current: None,
        };
        f(&mut styler);
        styler.reset()
    }
}

impl<S: Borrow<themecfg::Theme>> From<S> for Theme {
    fn from(s: S) -> Self {
        let s = s.borrow();
        let default = StylePack::load(&s.elements);
        let mut packs = EnumMap::default();
        for (level, pack) in &s.levels {
            let level = match level {
                InfallibleLevel::Valid(level) => level,
                InfallibleLevel::Invalid(s) => {
                    log::warn!("unknown level: {:?}", s);
                    continue;
                }
            };
            packs[*level] = StylePack::load(&s.elements.clone().merged(pack.clone()));
        }
        Self {
            default,
            packs,
            indicators: IndicatorPack::from(&s.indicators),
        }
    }
}

#[cfg(test)]
impl Sample for Arc<Theme> {
    fn sample() -> Self {
        Theme::from(themecfg::testing::theme().unwrap()).into()
    }
}

// ---

#[derive(Clone, Eq, PartialEq)]
struct Style(Sequence);

impl Style {
    #[inline(always)]
    pub fn apply<B: Push<u8>>(&self, buf: &mut B) {
        buf.extend_from_slice(self.0.data())
    }

    #[inline(always)]
    pub fn with<B: Push<u8>, F: FnOnce(&mut B)>(&self, buf: &mut B, f: F) {
        if self.0.data().is_empty() {
            f(buf)
        } else {
            buf.extend_from_slice(self.0.data());
            f(buf);
            buf.extend_from_slice(Self::reset().0.data());
        }
    }

    pub fn reset() -> Self {
        Sequence::reset().into()
    }

    fn convert_color(color: &themecfg::Color) -> ColorCode {
        match color {
            themecfg::Color::Plain(color) => match color {
                themecfg::PlainColor::Default => ColorCode::Default,
                themecfg::PlainColor::Black => ColorCode::Plain(Color::Black, Brightness::Normal),
                themecfg::PlainColor::Blue => ColorCode::Plain(Color::Blue, Brightness::Normal),
                themecfg::PlainColor::Cyan => ColorCode::Plain(Color::Cyan, Brightness::Normal),
                themecfg::PlainColor::Green => ColorCode::Plain(Color::Green, Brightness::Normal),
                themecfg::PlainColor::Magenta => ColorCode::Plain(Color::Magenta, Brightness::Normal),
                themecfg::PlainColor::Red => ColorCode::Plain(Color::Red, Brightness::Normal),
                themecfg::PlainColor::White => ColorCode::Plain(Color::White, Brightness::Normal),
                themecfg::PlainColor::Yellow => ColorCode::Plain(Color::Yellow, Brightness::Normal),
                themecfg::PlainColor::BrightBlack => ColorCode::Plain(Color::Black, Brightness::Bright),
                themecfg::PlainColor::BrightBlue => ColorCode::Plain(Color::Blue, Brightness::Bright),
                themecfg::PlainColor::BrightCyan => ColorCode::Plain(Color::Cyan, Brightness::Bright),
                themecfg::PlainColor::BrightGreen => ColorCode::Plain(Color::Green, Brightness::Bright),
                themecfg::PlainColor::BrightMagenta => ColorCode::Plain(Color::Magenta, Brightness::Bright),
                themecfg::PlainColor::BrightRed => ColorCode::Plain(Color::Red, Brightness::Bright),
                themecfg::PlainColor::BrightWhite => ColorCode::Plain(Color::White, Brightness::Bright),
                themecfg::PlainColor::BrightYellow => ColorCode::Plain(Color::Yellow, Brightness::Bright),
            },
            themecfg::Color::Palette(code) => ColorCode::Palette(*code),
            themecfg::Color::RGB(themecfg::RGB(r, g, b)) => ColorCode::Rgb(*r, *g, *b),
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::reset()
    }
}

impl<T: Into<Sequence>> From<T> for Style {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl From<&themecfg::Style> for Style {
    fn from(style: &themecfg::Style) -> Self {
        let mut codes = Vec::<StyleCode>::new();
        for mode in &style.modes {
            codes.push(
                match mode {
                    themecfg::Mode::Bold => Mode::Bold,
                    themecfg::Mode::Conceal => Mode::Conceal,
                    themecfg::Mode::CrossedOut => Mode::CrossedOut,
                    themecfg::Mode::Faint => Mode::Faint,
                    themecfg::Mode::Italic => Mode::Italic,
                    themecfg::Mode::RapidBlink => Mode::RapidBlink,
                    themecfg::Mode::Reverse => Mode::Reverse,
                    themecfg::Mode::SlowBlink => Mode::SlowBlink,
                    themecfg::Mode::Underline => Mode::Underline,
                }
                .into(),
            );
        }
        if let Some(color) = &style.background {
            codes.push(StyleCode::Background(Self::convert_color(color)));
        }
        if let Some(color) = &style.foreground {
            codes.push(StyleCode::Foreground(Self::convert_color(color)));
        }
        Self(codes.into())
    }
}

// ---

pub struct Styler<'a, B: Push<u8>> {
    buf: &'a mut B,
    pack: &'a StylePack,
    synced: Option<usize>,
    current: Option<usize>,
}

impl<'a, B: Push<u8>> Styler<'a, B> {
    #[inline(always)]
    pub fn reset(&mut self) {
        if let Some(style) = self.pack.reset {
            self.pack.styles[style].apply(self.buf)
        }
        self.synced = None;
    }

    #[inline(always)]
    fn set(&mut self, e: Element) -> Option<usize> {
        self.set_style(self.pack.elements[e])
    }

    #[inline(always)]
    fn set_style(&mut self, style: Option<usize>) -> Option<usize> {
        self.current.replace(style?)
    }

    #[inline(always)]
    fn sync(&mut self) {
        if self.synced != self.current {
            if let Some(style) = self.current.or(self.pack.reset) {
                self.pack.styles[style].apply(self.buf);
            }
            self.synced = self.current;
        }
    }
}

impl<'a, B: Push<u8>> StylingPush<B> for Styler<'a, B> {
    #[inline]
    fn element<R, F: FnOnce(&mut Self) -> R>(&mut self, element: Element, f: F) -> R {
        let style = self.set(element);
        let result = f(self);
        self.set_style(style);
        result
    }

    #[inline]
    fn space(&mut self) {
        self.buf.push(b' ');
    }

    #[inline]
    fn reset(&mut self) {
        self.reset()
    }

    #[inline]
    fn batch<R, F: FnOnce(&mut B) -> R>(&mut self, f: F) -> R {
        self.sync();
        f(self.buf)
    }
}

// ---

#[derive(Default)]
struct StylePack {
    elements: EnumMap<Element, Option<usize>>,
    reset: Option<usize>,
    styles: Vec<Style>,
}

impl StylePack {
    fn add(&mut self, element: Element, style: &Style) {
        let pos = match self.styles.iter().position(|x| x == style) {
            Some(pos) => pos,
            None => {
                self.styles.push(style.clone());
                self.styles.len() - 1
            }
        };
        self.elements[element] = Some(pos);
    }

    fn load(s: &themecfg::StylePack) -> Self {
        let mut result = Self::default();

        let items = s.items();
        if !items.is_empty() {
            result.styles.push(Style::reset());
            result.reset = Some(0);
        }

        for (&element, style) in s.items() {
            result.add(element, &Style::from(style))
        }

        if let Some(base) = s.items().get(&Element::Boolean) {
            for variant in [Element::BooleanTrue, Element::BooleanFalse] {
                let mut style = base.clone();
                if let Some(patch) = s.items().get(&variant) {
                    style = style.merged(patch)
                }
                result.add(variant, &Style::from(&style));
            }
        }

        result
    }
}

// ---

#[derive(Default)]
pub struct IndicatorPack {
    pub sync: SyncIndicatorPack,
}

impl From<&themecfg::IndicatorPack> for IndicatorPack {
    fn from(indicator: &themecfg::IndicatorPack) -> Self {
        Self {
            sync: SyncIndicatorPack::from(&indicator.sync),
        }
    }
}

// ---

#[derive(Default)]
pub struct SyncIndicatorPack {
    pub synced: Indicator,
    pub failed: Indicator,
}

impl From<&themecfg::SyncIndicatorPack> for SyncIndicatorPack {
    fn from(indicator: &themecfg::SyncIndicatorPack) -> Self {
        Self {
            synced: Indicator::from(&indicator.synced),
            failed: Indicator::from(&indicator.failed),
        }
    }
}

// ---

#[derive(Default)]
pub struct Indicator {
    pub value: String,
}

impl From<&themecfg::Indicator> for Indicator {
    fn from(indicator: &themecfg::Indicator) -> Self {
        let mut buf = Vec::new();
        let os = Style::from(&indicator.outer.style);
        let is = Style::from(&indicator.inner.style);
        os.apply(&mut buf);
        os.with(&mut buf, |buf| {
            buf.extend(indicator.outer.prefix.as_bytes());
            is.with(buf, |buf| {
                buf.extend(indicator.inner.prefix.as_bytes());
                buf.extend(indicator.text.as_bytes());
                buf.extend(indicator.outer.prefix.as_bytes());
            });
            buf.extend(indicator.outer.suffix.as_bytes());
        });

        Self {
            value: String::from_utf8(buf).unwrap(),
        }
    }
}

// ---

#[cfg(test)]
mod tests;
