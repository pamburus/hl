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
    themecfg::{self, StyleInventory},
};

// test imports
#[cfg(test)]
use crate::testing::Sample;

// ---

pub use level::Level;
pub use themecfg::{Element, MergeFlag, MergeFlags, MergedWith, ThemeInfo, ThemeOrigin};

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
        let flags = s.merge_flags();
        let inventory = s.styles.resolve(flags);
        let default = StylePack::load(&s.elements, &inventory, flags);
        // log::trace!("loaded default style pack: {:#?}", &default);
        let mut packs = EnumMap::default();
        for (level, pack) in &s.levels {
            let level = match level {
                InfallibleLevel::Valid(level) => *level,
                InfallibleLevel::Invalid(s) => {
                    log::warn!("unknown level: {:?}", s);
                    continue;
                }
            };
            let flags = flags - MergeFlag::ReplaceGroups;
            packs[level] = StylePack::load(&s.elements.clone().merged_with(pack, flags), &inventory, flags);
            // log::trace!("loaded style pack for level {:?}: {:#?}", level, &packs[level]);
        }
        Self {
            default,
            packs,
            indicators: IndicatorPack::new(&s.indicators, &inventory, flags),
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

#[derive(Clone, Eq, PartialEq, Debug)]
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

impl From<&themecfg::ResolvedStyle> for Style {
    fn from(style: &themecfg::ResolvedStyle) -> Self {
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

    fn load(s: &themecfg::StylePack, inventory: &themecfg::StyleInventory, flags: MergeFlags) -> Self {
        let mut result = Self::default();

        let items = s.items();
        if !items.is_empty() {
            result.styles.push(Style::reset());
            result.reset = Some(0);
        }

        // Build a map of inner elements to their parents for quick lookup
        let inner_pairs = [
            (Element::Level, Element::LevelInner),
            (Element::Logger, Element::LoggerInner),
            (Element::Caller, Element::CallerInner),
            (Element::InputNumber, Element::InputNumberInner),
            (Element::InputName, Element::InputNameInner),
        ];

        // Process all elements, applying parent→inner inheritance where needed
        for (&element, style) in s.items() {
            // Check if this element is an inner element that should inherit from its parent
            let mut final_style = style.clone();
            for (parent, inner) in inner_pairs {
                if element == inner {
                    // This is an inner element
                    // Only inherit from parent if the inner doesn't have a base style reference
                    if style.base.is_none() {
                        if let Some(parent_style) = s.items().get(&parent) {
                            final_style = parent_style.clone().merged(&final_style, flags);
                        }
                    }
                    break;
                }
            }
            result.add(element, &Style::from(&final_style.resolve(inventory, flags)));
        }

        // Add inherited inner elements that weren't explicitly defined
        for (parent, inner) in inner_pairs {
            if let Some(parent_style) = s.items().get(&parent) {
                if s.items().get(&inner).is_none() {
                    result.add(inner, &Style::from(&parent_style.resolve(inventory, flags)));
                }
            }
        }

        // Handle boolean variants inheriting from base boolean
        if let Some(base) = s.items().get(&Element::Boolean) {
            for variant in [Element::BooleanTrue, Element::BooleanFalse] {
                let mut style = base.clone();
                if let Some(patch) = s.items().get(&variant) {
                    style = style.merged(patch, flags)
                }
                result.add(variant, &Style::from(&style.resolve(inventory, flags)));
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

impl IndicatorPack {
    fn new(indicator: &themecfg::IndicatorPack, inventory: &StyleInventory, flags: MergeFlags) -> Self {
        Self {
            sync: SyncIndicatorPack::new(&indicator.sync, inventory, flags),
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
    fn new(indicator: &themecfg::SyncIndicatorPack, inventory: &StyleInventory, flags: MergeFlags) -> Self {
        Self {
            synced: Indicator::new(&indicator.synced, inventory, flags),
            failed: Indicator::new(&indicator.failed, inventory, flags),
        }
    }
}

// ---

#[derive(Default)]
pub struct Indicator {
    pub value: String,
}

impl Indicator {
    fn new(indicator: &themecfg::Indicator, inventory: &StyleInventory, flags: MergeFlags) -> Self {
        let mut buf = Vec::new();
        let os = Style::from(&indicator.outer.style.resolve(inventory, flags));
        let is = Style::from(&indicator.inner.style.resolve(inventory, flags));
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
