// std imports
use std::{cmp::min, ops::Deref, sync::Arc};

// third-party imports
use enumset::{EnumSet, EnumSetType, enum_set as mask};
use memchr::memmem;
use thiserror::Error;

// workspace imports
use encstr::{AnyEncodedString, EncodedString, JsonAppender};
use enumset_ext::EnumSetExt;
use mline::prefix_lines_within;

// local imports
use crate::{
    formatting::{AutoTrimFlag, AutoTrimFlags, WithAutoTrim},
    model::{MAX_NUMBER_LEN, looks_like_number},
    settings::MessageFormat,
};

// ---

/// Error is an error which may occur in the application.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    #[error(transparent)]
    ParseError(#[from] encstr::Error),
}

// ---

pub type Result<T, E = Error> = std::result::Result<T, E>;

// ---

pub trait Format {
    fn format<'a>(
        &self,
        input: EncodedString<'a>,
        buf: &mut Vec<u8>,
        options: FormatOptions<'a>,
    ) -> Result<FormatResult>;

    #[inline(always)]
    fn rtrim(self, n: usize) -> FormatRightTrimmed<Self>
    where
        Self: Sized,
    {
        FormatRightTrimmed::new(n, self)
    }
}

pub type DynFormat = Arc<dyn Format + Send + Sync>;

#[derive(Clone, Copy)]
pub struct FormatOptions<'a> {
    xsa: ExtendedSpaceAction<'a>,
}

impl<'a> From<ExtendedSpaceAction<'a>> for FormatOptions<'a> {
    #[inline(always)]
    fn from(xsa: ExtendedSpaceAction<'a>) -> Self {
        Self { xsa }
    }
}

#[derive(Clone)]
pub struct DynMessageFormat {
    format: DynFormat,
    pub delimited: bool,
}

impl DynMessageFormat {
    pub fn new(formatting: &super::Formatting, ascii: super::AsciiMode) -> Self {
        new_message_format(formatting.message.format, || {
            formatting.punctuation.message_delimiter.resolve(ascii)
        })
    }
}

impl Deref for DynMessageFormat {
    type Target = DynFormat;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.format
    }
}

pub fn new_message_format<D: DisplayResolve>(setting: MessageFormat, delimiter: D) -> DynMessageFormat {
    let (format, delimited): (DynFormat, _) = match setting {
        MessageFormat::AutoQuoted => (Arc::new(MessageFormatAutoQuoted), false),
        MessageFormat::AlwaysQuoted => (Arc::new(MessageFormatAlwaysQuoted), false),
        MessageFormat::AlwaysDoubleQuoted => (Arc::new(MessageFormatDoubleQuoted), false),
        MessageFormat::Delimited => {
            let delimiter = format!(" {} ", delimiter.resolve());
            let n = delimiter.len();
            (Arc::new(MessageFormatDelimited::new(delimiter).rtrim(n)), true)
        }
        MessageFormat::Raw => (Arc::new(MessageFormatRaw), false),
    };
    DynMessageFormat { format, delimited }
}

// ---

pub trait Analyze {
    fn analyze(&self) -> Analysis;
}

impl Analyze for [u8] {
    #[inline(always)]
    fn analyze(&self) -> Analysis {
        let mut chars = Mask::empty();
        self.iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
            chars |= group;
        });
        Analysis { chars }
    }
}

// ---

#[derive(Clone, Copy)]
pub enum ExtendedSpaceAction<'a> {
    Inline,
    Expand(&'a dyn Fn(&mut Vec<u8>) -> usize),
    Escape,
    Abort,
}

#[must_use]
pub enum FormatResult {
    Ok(Option<Analysis>),
    Aborted,
}

impl FormatResult {
    #[cfg(test)]
    #[inline(always)]
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
}

// ---

pub struct Analysis {
    pub chars: Mask,
}

impl Analysis {
    #[inline(always)]
    pub fn empty() -> Self {
        Self { chars: Mask::empty() }
    }
}

// ---

// ---

pub trait DisplayResolve {
    type Output: std::fmt::Display;

    fn resolve(self) -> Self::Output;
}

impl<'a> DisplayResolve for &'a str {
    type Output = Self;

    #[inline(always)]
    fn resolve(self) -> &'a str {
        self
    }
}

impl<'a, F> DisplayResolve for F
where
    F: FnOnce() -> &'a str,
{
    type Output = &'a str;

    #[inline(always)]
    fn resolve(self) -> &'a str {
        self()
    }
}

// ---

/// Characters that make an unquoted key or value ambiguous in `key=value` output.
const NON_PLAIN: Mask = mask!(
    Flag::Control
        | Flag::DoubleQuote
        | Flag::SingleQuote
        | Flag::Backslash
        | Flag::Backtick
        | Flag::Space
        | Flag::Tab
        | Flag::Newline
        | Flag::EqualSign
);

// ---

/// How a field key is quoted in the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyQuoting {
    /// Written as is.
    Plain,
    /// Wrapped in the quote character, without escaping.
    Quoted(Quote),
    /// Written as a double-quoted JSON string with escape sequences.
    JsonEscaped,
}

/// A quote character that can wrap a string without escaping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quote {
    Double,
    Single,
    Backtick,
}

impl Quote {
    #[inline(always)]
    pub const fn byte(self) -> u8 {
        match self {
            Self::Double => b'"',
            Self::Single => b'\'',
            Self::Backtick => b'`',
        }
    }
}

impl KeyQuoting {
    /// Picks the quoting for a key using the same quote-character cascade as string values.
    ///
    /// Unlike values, keys never look like numbers or literals in key position,
    /// so only the characters in [`NON_PLAIN`] trigger quoting, and keys always stay on one line.
    /// Key prettification only replaces `_` with `-`, neither of which affects the result,
    /// so the original key can be analyzed.
    #[inline(always)]
    pub fn of(key: &[u8]) -> Self {
        if key.is_empty() {
            return Self::Quoted(Quote::Double);
        }

        let mask = key.analyze().chars;
        if !mask.intersects(NON_PLAIN) {
            return Self::Plain;
        }

        const BREAKING: Mask = mask!(Flag::Control | Flag::Tab | Flag::Newline);

        if !mask.intersects(BREAKING | Flag::Backslash | Flag::DoubleQuote) {
            Self::Quoted(Quote::Double)
        } else if !mask.intersects(BREAKING | Flag::Backslash | Flag::SingleQuote) {
            Self::Quoted(Quote::Single)
        } else if !mask.intersects(BREAKING | Flag::Backtick) {
            Self::Quoted(Quote::Backtick)
        } else {
            Self::JsonEscaped
        }
    }
}

/// Appends `input` to `buf` as a double-quoted JSON string, escaping special characters.
#[inline]
pub fn format_json_str(input: &str, buf: &mut Vec<u8>) {
    // Decoding a raw string never fails.
    _ = EncodedString::raw(input).format_json(buf);
}

// ---

#[derive(Default)]
pub struct ValueFormatAuto;

impl Format for ValueFormatAuto {
    #[inline(always)]
    fn format<'a>(
        &self,
        input: EncodedString<'a>,
        buf: &mut Vec<u8>,
        options: FormatOptions<'a>,
    ) -> Result<FormatResult> {
        if input.is_empty() {
            buf.extend(r#""""#.as_bytes());
            return Ok(FormatResult::Ok(None));
        }

        let begin = buf.len();
        _ = buf.with_auto_trim(
            |buf| ValueFormatRaw.format(input, buf, options),
            AutoTrimFlag::PreserveWhiteSpaceOnly,
        )?;

        let analysis = buf[begin..].analyze();
        let mask = analysis.chars;

        let confusing = || matches!(&buf[begin..], [b'{', ..] | [b'[', ..] | b"true" | b"false" | b"null");

        let like_number =
            || (mask == mask!(Flag::Digit) && buf[begin..].len() <= MAX_NUMBER_LEN) || looks_like_number(&buf[begin..]);

        if !mask.intersects(NON_PLAIN) && !confusing() && !like_number() {
            return Ok(FormatResult::Ok(None));
        }

        if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf.push(b'"');
            buf.push(b'"');
            buf[begin..].rotate_right(1);
            return Ok(FormatResult::Ok(None));
        }

        if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf.push(b'\'');
            buf.push(b'\'');
            buf[begin..].rotate_right(1);
            return Ok(FormatResult::Ok(None));
        }

        const WS: Mask = mask!(Flag::Newline | Flag::Tab | Flag::Space);

        let has_control = mask.contains(Flag::Control);
        let has_backtick = mask.contains(Flag::Backtick);
        let has_extended_space = mask.intersects(Flag::Newline | Flag::Tab);
        let has_non_whitespace = mask.intersects(!WS);

        if !has_control && has_non_whitespace {
            if !has_backtick && (!has_extended_space || matches!(options.xsa, ExtendedSpaceAction::Inline)) {
                buf.push(b'`');
                buf.push(b'`');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(None));
            }

            match options.xsa {
                ExtendedSpaceAction::Expand(prefix) => {
                    let l0 = buf.len();
                    let pl = prefix(buf);
                    let n = buf.len() - l0;
                    buf[begin..].rotate_right(n);
                    prefix_lines_within(buf, begin + n.., 1.., (begin + n - pl)..(begin + n));
                    return Ok(FormatResult::Ok(None));
                }
                ExtendedSpaceAction::Abort => {
                    buf.truncate(begin);
                    return Ok(FormatResult::Aborted);
                }
                _ => {}
            }
        }

        buf.truncate(begin);
        ValueFormatDoubleQuoted.format(input, buf, options)
    }
}

// ---

pub struct ValueFormatRaw;

impl Format for ValueFormatRaw {
    #[inline(always)]
    fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>, _: FormatOptions<'a>) -> Result<FormatResult> {
        input.decode(buf)?;
        Ok(FormatResult::Ok(None))
    }
}

// ---

pub struct ValueFormatDoubleQuoted;

impl Format for ValueFormatDoubleQuoted {
    #[inline(always)]
    fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>, _: FormatOptions<'a>) -> Result<FormatResult> {
        input.format_json(buf)?;
        Ok(FormatResult::Ok(None))
    }
}

// ---

pub struct MessageFormatAutoQuoted;

impl Format for MessageFormatAutoQuoted {
    #[inline(always)]
    fn format<'a>(
        &self,
        input: EncodedString<'a>,
        buf: &mut Vec<u8>,
        options: FormatOptions<'a>,
    ) -> Result<FormatResult> {
        if input.is_empty() {
            buf.extend(r#""""#.as_bytes());
            return Ok(FormatResult::Ok(Some(Analysis::empty())));
        }

        let begin = buf.len();
        _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, options), AutoTrimFlags::new())?;

        let analysis = buf[begin..].analyze();
        let mask = analysis.chars;

        const NOT_PLAIN: Mask = mask!(Flag::EqualSign | Flag::Control | Flag::Newline | Flag::Backslash);

        if !mask.intersects(NOT_PLAIN) && !matches!(buf.get(begin), Some(b'"' | b'\'' | b'`')) && &buf[begin..] != b"~"
        {
            return Ok(FormatResult::Ok(Some(analysis)));
        }

        if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf.push(b'"');
            buf.push(b'"');
            buf[begin..].rotate_right(1);
            return Ok(FormatResult::Ok(Some(analysis)));
        }

        if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf.push(b'\'');
            buf.push(b'\'');
            buf[begin..].rotate_right(1);
            return Ok(FormatResult::Ok(Some(analysis)));
        }

        const XS: Mask = mask!(Flag::Newline);
        let has_control = mask.contains(Flag::Control);
        let has_backtick = mask.contains(Flag::Backtick);
        let has_newline = mask.intersects(XS);

        if !has_control {
            if !has_backtick && (!has_newline || matches!(options.xsa, ExtendedSpaceAction::Inline)) {
                buf.push(b'`');
                buf.push(b'`');
                buf[begin..].rotate_right(1);
                return Ok(FormatResult::Ok(Some(analysis)));
            }

            if matches!(options.xsa, ExtendedSpaceAction::Abort) {
                buf.truncate(begin);
                return Ok(FormatResult::Aborted);
            }
        }

        buf.truncate(begin);
        MessageFormatDoubleQuoted.format(input, buf, options)
    }
}

// ---

pub struct MessageFormatAlwaysQuoted;

impl Format for MessageFormatAlwaysQuoted {
    #[inline(always)]
    fn format<'a>(
        &self,
        input: EncodedString<'a>,
        buf: &mut Vec<u8>,
        options: FormatOptions<'a>,
    ) -> Result<FormatResult> {
        if input.is_empty() {
            return Ok(FormatResult::Ok(Some(Analysis::empty())));
        }

        let begin = buf.len();
        buf.push(b'"');
        _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, options), AutoTrimFlags::new())?;

        let analysis = buf[begin + 1..].analyze();
        let mask = analysis.chars;

        const XS: Mask = mask!(Flag::Newline);
        let has_newline = mask.intersects(XS);

        if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf.push(b'"');
            return Ok(FormatResult::Ok(None));
        }

        if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf[begin] = b'\'';
            buf.push(b'\'');
            return Ok(FormatResult::Ok(None));
        }

        if !mask.intersects(Flag::Backtick | Flag::Control) {
            // Backticks can contain newlines, but only in inline mode
            if !has_newline || matches!(options.xsa, ExtendedSpaceAction::Inline) {
                buf[begin] = b'`';
                buf.push(b'`');
                return Ok(FormatResult::Ok(None));
            }

            // For newlines in non-inline modes, fall through to double-quoted or abort
            if matches!(options.xsa, ExtendedSpaceAction::Abort) {
                buf.truncate(begin);
                return Ok(FormatResult::Aborted);
            }
        }

        buf.truncate(begin);
        MessageFormatDoubleQuoted.format(input, buf, options)
    }
}

// ---

pub struct MessageFormatDelimited(String);

impl MessageFormatDelimited {
    pub fn new(suffix: String) -> Self {
        Self(suffix)
    }
}

impl Format for MessageFormatDelimited {
    #[inline(always)]
    fn format<'a>(
        &self,
        input: EncodedString<'a>,
        buf: &mut Vec<u8>,
        options: FormatOptions<'a>,
    ) -> Result<FormatResult> {
        if input.is_empty() {
            return Ok(FormatResult::Ok(None));
        }

        let begin = buf.len();
        _ = buf.with_auto_trim(|buf| MessageFormatRaw.format(input, buf, options), AutoTrimFlags::new())?;

        let analysis = buf[begin..].analyze();
        let mask = analysis.chars;

        const XS: Mask = mask!(Flag::Newline);
        if mask.intersects(XS) && matches!(options.xsa, ExtendedSpaceAction::Abort) {
            buf.truncate(begin);
            return Ok(FormatResult::Aborted);
        }

        let can_inline = !mask.intersects(XS) || matches!(options.xsa, ExtendedSpaceAction::Inline);

        if !mask.contains(Flag::Control)
            && !matches!(buf.get(begin), Some(b'"' | b'\'' | b'`'))
            && &buf[begin..] != b"~"
            && memmem::find(&buf[begin..], self.0.as_bytes()).is_none()
            && can_inline
        {
            buf.extend(self.0.as_bytes());
            return Ok(FormatResult::Ok(Some(analysis)));
        }

        if !mask.intersects(Flag::DoubleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf.push(b'"');
            buf.push(b'"');
            buf[begin..].rotate_right(1);
            buf.extend(self.0.as_bytes());
            return Ok(FormatResult::Ok(Some(analysis)));
        }

        if !mask.intersects(Flag::SingleQuote | Flag::Control | Flag::Tab | Flag::Newline | Flag::Backslash) {
            buf.push(b'\'');
            buf.push(b'\'');
            buf[begin..].rotate_right(1);
            buf.extend(self.0.as_bytes());
            return Ok(FormatResult::Ok(Some(analysis)));
        }

        if !mask.intersects(Flag::Backtick | Flag::Control) && can_inline {
            buf.push(b'`');
            buf.push(b'`');
            buf[begin..].rotate_right(1);
            buf.extend(self.0.as_bytes());
            return Ok(FormatResult::Ok(Some(analysis)));
        }

        buf.truncate(begin);
        let result = MessageFormatDoubleQuoted.format(input, buf, options)?;
        buf.extend(self.0.as_bytes());
        Ok(result)
    }
}

// ---

pub struct MessageFormatRaw;

impl Format for MessageFormatRaw {
    #[inline(always)]
    fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>, _: FormatOptions<'a>) -> Result<FormatResult> {
        input.decode(buf)?;
        Ok(FormatResult::Ok(None))
    }
}

// ---

pub struct MessageFormatDoubleQuoted;

impl Format for MessageFormatDoubleQuoted {
    #[inline(always)]
    fn format<'a>(&self, input: EncodedString<'a>, buf: &mut Vec<u8>, _: FormatOptions<'a>) -> Result<FormatResult> {
        input.format_json(buf)?;
        Ok(FormatResult::Ok(None))
    }
}

// ---

pub struct FormatRightTrimmed<F> {
    n: usize,
    inner: F,
}

impl<F> FormatRightTrimmed<F> {
    #[inline(always)]
    fn new(n: usize, inner: F) -> Self {
        Self { n, inner }
    }
}

impl<F: Format> Format for FormatRightTrimmed<F> {
    #[inline(always)]
    fn format<'a>(
        &self,
        input: EncodedString<'a>,
        buf: &mut Vec<u8>,
        options: FormatOptions<'a>,
    ) -> Result<FormatResult> {
        let begin = buf.len();
        let result = self.inner.format(input, buf, options)?;
        buf.truncate(buf.len() - min(buf.len() - begin, self.n));
        Ok(result)
    }
}

// ---

trait EncodedStringExt {
    fn format_json(&self, buf: &mut Vec<u8>) -> Result<()>;
}

impl<'a, S> EncodedStringExt for S
where
    S: AnyEncodedString<'a>,
{
    #[inline(always)]
    fn format_json(&self, buf: &mut Vec<u8>) -> Result<()> {
        buf.push(b'"');
        self.decode(JsonAppender::new(buf))?;
        buf.push(b'"');
        Ok(())
    }
}

// ---

static CHAR_GROUPS: [Mask; 256] = {
    const CT: Mask = mask!(Flag::Control); // 0x00..0x1F except 0x09, 0x0A, 0x0D
    const DQ: Mask = mask!(Flag::DoubleQuote); // 0x22
    const SQ: Mask = mask!(Flag::SingleQuote); // 0x27
    const BS: Mask = mask!(Flag::Backslash); // 0x5C
    const BT: Mask = mask!(Flag::Backtick); // 0x60
    const SP: Mask = mask!(Flag::Space); // 0x20
    const TB: Mask = mask!(Flag::Tab); // 0x09
    const NL: Mask = mask!(Flag::Newline); // 0x0A, 0x0D
    const EQ: Mask = mask!(Flag::EqualSign); // 0x3D
    const HY: Mask = mask!(Flag::Minus); // Hyphen, 0x2D
    const DO: Mask = mask!(Flag::Dot); // Dot, 0x2E
    const DD: Mask = mask!(Flag::Digit); // Decimal digit, 0x30..0x39
    const CL: Mask = mask!(Flag::Colon); // Colon, 0x3A
    const TL: Mask = mask!(Flag::Tilde); // Tilde, 0x7E
    const PA: Mask = mask!(Flag::Parantheses); // 0x28, 0x29
    const BK: Mask = mask!(Flag::Brackets); // 0x5B, 0x5D
    const BR: Mask = mask!(Flag::Braces); // 0x7B, 0x7D
    const AB: Mask = mask!(Flag::AngleBrackets); // 0x3C, 0x3E
    const __: Mask = mask!(Flag::Other);
    [
        //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
        CT, CT, CT, CT, CT, CT, CT, CT, CT, TB, NL, CT, CT, NL, CT, CT, // 0
        CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
        SP, __, DQ, __, __, __, __, SQ, __, __, __, __, __, HY, DO, __, // 2
        DD, DD, DD, DD, DD, DD, DD, DD, DD, DD, CL, __, AB, EQ, AB, __, // 3
        __, __, __, __, __, __, __, __, PA, PA, __, __, __, __, __, __, // 4
        __, __, __, __, __, __, __, __, __, __, __, BK, BS, BK, __, __, // 5
        BT, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
        __, __, __, __, __, __, __, __, __, __, __, BR, __, BR, TL, __, // 7
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
    ]
};

#[derive(EnumSetType)]
pub enum Flag {
    Control,
    DoubleQuote,
    SingleQuote,
    Backslash,
    Backtick,
    Space,
    Tab,
    Newline,
    EqualSign,
    Digit,
    Minus,
    Dot,
    Colon,
    Tilde,
    Parantheses,
    Brackets,
    Braces,
    AngleBrackets,
    Other,
}

pub type Mask = EnumSet<Flag>;

#[cfg(test)]
mod tests;
