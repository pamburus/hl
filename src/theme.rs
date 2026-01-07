// std imports
use std::{borrow::Borrow, collections::HashMap, sync::Arc, vec::Vec};

// third-party imports
use enum_map::EnumMap;
use unicode_width::UnicodeWidthStr;

// local imports
use crate::{
    appdirs::AppDirs,
    error::*,
    eseq::{Brightness, Color, ColorCode, Mode, Sequence, StyleCode},
    fmtx::Push,
    level,
    syntax::*,
    themecfg,
};

// test imports
#[cfg(test)]
use crate::testing::Sample;

// ---

pub use level::Level;
pub use themecfg::{Element, MergeFlag, MergeFlags, ThemeInfo, ThemeOrigin};

// ---

pub trait StylingPush<B: Push<u8>> {
    fn element<R, F: FnOnce(&mut Self) -> R>(&mut self, element: Element, f: F) -> R;
    fn batch<R, F: FnOnce(&mut B) -> R>(&mut self, f: F) -> R;
    fn space(&mut self);
    fn reset(&mut self);
}

#[derive(Default)]
struct LevelStyles {
    known: EnumMap<Level, StylePack>,
    unknown: StylePack,
}

// ---

#[derive(Default)]
pub struct Theme {
    levels: LevelStyles,
    pub indicators: IndicatorPack,
    pub expanded_value_prefix: ExpandedValuePrefix,
    pub expanded_value_suffix: ExpandedValueSuffix,
}

impl Theme {
    pub fn none() -> Self {
        Self::default()
    }

    fn new(cfg: impl Borrow<themecfg::Theme>) -> Self {
        let cfg = cfg.borrow();
        let mut levels = LevelStyles {
            unknown: StylePack::load(&cfg.elements),
            ..Default::default()
        };
        for (level, pack) in &cfg.levels {
            if let Some(level) = level {
                levels.known[*level] = StylePack::load(pack);
            } else {
                levels.unknown = StylePack::load(pack);
            }
        }
        Self {
            levels,
            indicators: IndicatorPack::new(&cfg.indicators),
            expanded_value_prefix: cfg
                .elements
                .get(&Element::ValueExpansion)
                .map(ExpandedValuePrefix::from)
                .unwrap_or_default(),
            expanded_value_suffix: cfg
                .elements
                .get(&Element::ValueExpansion)
                .map(ExpandedValueSuffix::from)
                .unwrap_or_default(),
        }
    }

    pub fn load(dirs: &AppDirs, name: &str) -> Result<Self> {
        Ok(themecfg::Theme::load(dirs, name)?.into())
    }

    pub fn load_with_overlays(dirs: &AppDirs, name: &str, overlays: &[impl AsRef<str>]) -> Result<Self> {
        Ok(themecfg::Theme::load_with_overlays(dirs, name, overlays)?.into())
    }

    pub fn embedded(name: &str) -> Result<Self> {
        Ok(themecfg::Theme::embedded(name)?.into())
    }

    pub fn list(dirs: &AppDirs) -> Result<HashMap<Arc<str>, ThemeInfo>> {
        Ok(themecfg::Theme::list(dirs)?)
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
                &Some(level) => &self.levels.known[level],
                None => &self.levels.unknown,
            },
            synced: None,
            current: None,
        };
        f(&mut styler);
        styler.reset()
    }
}

impl From<themecfg::Theme> for Theme {
    fn from(cfg: themecfg::Theme) -> Self {
        Self::from(&cfg)
    }
}

impl From<&themecfg::Theme> for Theme {
    fn from(cfg: &themecfg::Theme) -> Self {
        Self::new(cfg)
    }
}

#[cfg(test)]
impl Sample for Arc<Theme> {
    fn sample() -> Self {
        Theme::from(themecfg::testing::theme().unwrap()).into()
    }
}

// ---

#[derive(Clone, Eq, PartialEq, Debug, Default)]
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

impl<T: Into<Sequence>> From<T> for Style {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl From<&themecfg::Style> for Style {
    fn from(style: &themecfg::Style) -> Self {
        let mut codes = Vec::<StyleCode>::new();
        for mode in style.modes.adds {
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
            let color = Self::convert_color(color);
            if !matches!(color, ColorCode::Default) {
                codes.push(StyleCode::Background(color));
            }
        }
        if let Some(color) = &style.foreground {
            let color = Self::convert_color(color);
            if !matches!(color, ColorCode::Default) {
                codes.push(StyleCode::Foreground(color));
            }
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
        self.current = None;
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

impl<'a> Styler<'a, Vec<u8>> {
    #[inline]
    pub fn transact<R, E, F>(&mut self, f: F) -> std::result::Result<R, E>
    where
        F: FnOnce(&mut Self) -> std::result::Result<R, E>,
    {
        let current = self.current;
        let synced = self.synced;
        let n = self.buf.len();
        let result = f(self);
        if result.is_err() {
            self.buf.truncate(n);
            self.current = current;
            self.synced = synced;
        }
        result
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

#[derive(Default, Debug)]
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

        if !s.is_empty() {
            result.styles.push(Style::reset());
            result.reset = Some(0);
        }

        for (&element, style) in s.iter() {
            result.add(element, &Style::from(style));
        }

        result
    }
}

// ---

#[derive(Default)]
pub struct IndicatorPack {
    pub sync: SyncIndicatorPack,
}

impl IndicatorPack {
    fn new(indicator: &themecfg::IndicatorPack) -> Self {
        Self {
            sync: SyncIndicatorPack::new(&indicator.sync),
        }
    }
}

// ---

#[derive(Default)]
pub struct SyncIndicatorPack {
    pub synced: Indicator,
    pub failed: Indicator,
}

impl SyncIndicatorPack {
    fn new(indicator: &themecfg::SyncIndicatorPack) -> Self {
        Self {
            synced: Indicator::new(&indicator.synced),
            failed: Indicator::new(&indicator.failed),
        }
    }
}

// ---

#[derive(Default)]
pub struct Indicator {
    pub value: String,
    pub width: usize,
}

impl Indicator {
    fn new(indicator: &themecfg::Indicator) -> Self {
        let mut buf = Vec::new();
        let os = Style::from(&indicator.outer.style);
        let is = Style::from(&indicator.inner.style);
        os.with(&mut buf, |buf| {
            buf.extend(indicator.outer.prefix.as_bytes());
            is.with(buf, |buf| {
                buf.extend(indicator.inner.prefix.as_bytes());
                buf.extend(indicator.text.as_bytes());
                buf.extend(indicator.inner.suffix.as_bytes());
            });
            buf.extend(indicator.outer.suffix.as_bytes());
        });

        Self {
            value: String::from_utf8(buf).unwrap(),
            width: indicator.text.width()
                + indicator.outer.prefix.width()
                + indicator.outer.suffix.width()
                + indicator.inner.prefix.width()
                + indicator.inner.suffix.width(),
        }
    }
}

// ---

pub struct ExpandedValuePrefix {
    pub value: String,
}

impl From<&themecfg::Style> for ExpandedValuePrefix {
    fn from(style: &themecfg::Style) -> Self {
        Self {
            value: styled(style.into(), &Self::default().value),
        }
    }
}

impl Default for ExpandedValuePrefix {
    fn default() -> Self {
        Self {
            value: " ".repeat(EXPANDED_KEY_HEADER.len()) + EXPANDED_VALUE_INDENT,
        }
    }
}

// ---

pub struct ExpandedValueSuffix {
    pub value: String,
}

impl From<&themecfg::Style> for ExpandedValueSuffix {
    fn from(style: &themecfg::Style) -> Self {
        Self {
            value: styled(style.into(), &Self::default().value),
        }
    }
}

impl Default for ExpandedValueSuffix {
    fn default() -> Self {
        Self {
            value: EXPANDED_VALUE_HEADER.to_string(),
        }
    }
}

// ---

fn styled(style: Style, text: &str) -> String {
    if style == Style::reset() {
        return text.into();
    }

    let mut buf = Vec::new();
    style.with(&mut buf, |buf| {
        buf.extend(text.as_bytes());
    });
    String::from_utf8(buf).unwrap()
}

// ---

#[cfg(test)]
mod tests;
