//! ANSI-styled bytes → structured [`Segment`]s for the JSON wire format.
//!
//! The hl pipeline emits ANSI-escaped text. The browser can't render ANSI directly, and
//! we don't want to ship a Vt100 parser to the client either, so we lift the styling out
//! here: each contiguous run of styled (or unstyled) bytes becomes a Segment with a
//! `class` string (themable CSS, when the style fits in a fixed palette) and an
//! optional `style` string (inline CSS, for truecolor that doesn't).
//!
//! The class scheme matches what the static theme stylesheets in `www/themes/` define:
//!   - `hl-fg-N` / `hl-bg-N`     for 16-color (0–15) and 256-color (16–255) palettes
//!   - `hl-bold`, `hl-italic`, `hl-underline`     for attributes
//!   - inline `color:#rgb;`     for 24-bit truecolor

use std::fmt::Write;

use serde::Serialize;

/// One styled span of text. `class` is empty for unstyled text; `style` is empty when
/// no inline CSS is needed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Segment {
    /// Space-separated CSS classes. Empty when the text is unstyled.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub class: String,
    /// Inline CSS for state classes can't express (truecolor). Empty when unused.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub style: String,
    pub text: String,
}

/// Walk a stream of bytes that may contain ANSI SGR escape sequences and produce a
/// vector of segments. Non-SGR escapes (cursor moves, OSC, etc.) are dropped silently.
pub fn ansi_to_segments(bytes: &[u8]) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut state = SgrState::default();
    let mut text_buf: Vec<u8> = Vec::new();

    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1B && bytes.get(i + 1) == Some(&b'[') {
            // CSI sequence: ESC [ <parameters> <final>
            // Parameter bytes are 0x30–0x3F; intermediate 0x20–0x2F; final 0x40–0x7E.
            let mut j = i + 2;
            while j < bytes.len() && bytes[j] >= 0x20 && bytes[j] < 0x40 {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'm' {
                // SGR — apply
                flush(&mut out, &state, &mut text_buf);
                state.apply_sgr(&bytes[i + 2..j]);
                i = j + 1;
                continue;
            }
            // Some other CSI: skip the whole sequence (including the final byte).
            i = if j < bytes.len() { j + 1 } else { bytes.len() };
            continue;
        }
        text_buf.push(bytes[i]);
        i += 1;
    }
    flush(&mut out, &state, &mut text_buf);
    out
}

fn flush(out: &mut Vec<Segment>, state: &SgrState, text_buf: &mut Vec<u8>) {
    if text_buf.is_empty() {
        return;
    }
    let text = String::from_utf8_lossy(text_buf).into_owned();
    text_buf.clear();
    let (class, style) = state.css();
    out.push(Segment { class, style, text });
}

#[derive(Default, Clone, Copy)]
struct SgrState {
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    bold: bool,
    italic: bool,
    underline: bool,
}

#[derive(Clone, Copy)]
enum AnsiColor {
    Palette(u8),
    Rgb(u8, u8, u8),
}

impl SgrState {
    fn apply_sgr(&mut self, params: &[u8]) {
        // SGR parameter list: numbers separated by ';'. Empty parameter (just ';' or
        // empty `[m`) is equivalent to 0 per the standard.
        let s = std::str::from_utf8(params).unwrap_or("");
        let mut nums = s.split(|c| c == ';' || c == ':').map(|p| {
            if p.is_empty() {
                Some(0u32)
            } else {
                p.parse::<u32>().ok()
            }
        });
        while let Some(item) = nums.next() {
            let Some(n) = item else { continue };
            match n {
                0 => *self = Self::default(),
                1 => self.bold = true,
                3 => self.italic = true,
                4 => self.underline = true,
                22 => self.bold = false,
                23 => self.italic = false,
                24 => self.underline = false,
                30..=37 => self.fg = Some(AnsiColor::Palette((n - 30) as u8)),
                38 => match nums.next().flatten() {
                    Some(5) => {
                        self.fg = nums.next().flatten().map(|x| AnsiColor::Palette(x as u8));
                    }
                    Some(2) => {
                        let r = nums.next().flatten().unwrap_or(0) as u8;
                        let g = nums.next().flatten().unwrap_or(0) as u8;
                        let b = nums.next().flatten().unwrap_or(0) as u8;
                        self.fg = Some(AnsiColor::Rgb(r, g, b));
                    }
                    _ => {}
                },
                39 => self.fg = None,
                40..=47 => self.bg = Some(AnsiColor::Palette((n - 40) as u8)),
                48 => match nums.next().flatten() {
                    Some(5) => {
                        self.bg = nums.next().flatten().map(|x| AnsiColor::Palette(x as u8));
                    }
                    Some(2) => {
                        let r = nums.next().flatten().unwrap_or(0) as u8;
                        let g = nums.next().flatten().unwrap_or(0) as u8;
                        let b = nums.next().flatten().unwrap_or(0) as u8;
                        self.bg = Some(AnsiColor::Rgb(r, g, b));
                    }
                    _ => {}
                },
                49 => self.bg = None,
                90..=97 => self.fg = Some(AnsiColor::Palette((n - 90 + 8) as u8)),
                100..=107 => self.bg = Some(AnsiColor::Palette((n - 100 + 8) as u8)),
                _ => {}
            }
        }
    }

    /// Render the state as `(class, style)` strings.
    fn css(&self) -> (String, String) {
        let mut class = String::new();
        let mut style = String::new();
        let push_class = |c: &mut String, name: &str| {
            if !c.is_empty() {
                c.push(' ');
            }
            c.push_str(name);
        };
        if let Some(c) = self.fg {
            match c {
                AnsiColor::Palette(n) => {
                    push_class(&mut class, &format!("hl-fg-{n}"));
                }
                AnsiColor::Rgb(r, g, b) => {
                    let _ = write!(style, "color:#{r:02x}{g:02x}{b:02x};");
                }
            }
        }
        if let Some(c) = self.bg {
            match c {
                AnsiColor::Palette(n) => {
                    push_class(&mut class, &format!("hl-bg-{n}"));
                }
                AnsiColor::Rgb(r, g, b) => {
                    let _ = write!(style, "background-color:#{r:02x}{g:02x}{b:02x};");
                }
            }
        }
        if self.bold {
            push_class(&mut class, "hl-bold");
        }
        if self.italic {
            push_class(&mut class, "hl-italic");
        }
        if self.underline {
            push_class(&mut class, "hl-underline");
        }
        (class, style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_one_segment() {
        let s = ansi_to_segments(b"hello world");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "hello world");
        assert_eq!(s[0].class, "");
        assert_eq!(s[0].style, "");
    }

    #[test]
    fn empty_input_emits_nothing() {
        assert!(ansi_to_segments(&[]).is_empty());
    }

    #[test]
    fn red_then_plain() {
        // ESC[31m red ESC[0m plain
        let s = ansi_to_segments(b"\x1b[31mred\x1b[0m plain");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].text, "red");
        assert_eq!(s[0].class, "hl-fg-1");
        assert_eq!(s[1].text, " plain");
        assert_eq!(s[1].class, "");
    }

    #[test]
    fn bold_underline_combine() {
        let s = ansi_to_segments(b"\x1b[1;4mhi\x1b[0m");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "hi");
        assert_eq!(s[0].class, "hl-bold hl-underline");
    }

    #[test]
    fn indexed_256_color() {
        let s = ansi_to_segments(b"\x1b[38;5;208morange\x1b[0m");
        assert_eq!(s[0].text, "orange");
        assert_eq!(s[0].class, "hl-fg-208");
    }

    #[test]
    fn truecolor_uses_inline_style() {
        let s = ansi_to_segments(b"\x1b[38;2;255;128;0mwarm\x1b[0m");
        assert_eq!(s[0].text, "warm");
        assert_eq!(s[0].class, "");
        assert_eq!(s[0].style, "color:#ff8000;");
    }

    #[test]
    fn bright_palette() {
        // 90 is bright black -> index 8 in the extended palette
        let s = ansi_to_segments(b"\x1b[90mdim\x1b[0m");
        assert_eq!(s[0].class, "hl-fg-8");
    }

    #[test]
    fn non_sgr_csi_is_dropped() {
        // ESC[H (cursor home) and ESC[2J (clear screen) — both non-SGR
        let s = ansi_to_segments(b"\x1b[H\x1b[2Jclear");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "clear");
    }

    #[test]
    fn reset_clears_state() {
        let s = ansi_to_segments(b"\x1b[1;31mred\x1b[0mclear");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].class, "hl-fg-1 hl-bold");
        assert_eq!(s[1].class, "");
    }

    #[test]
    fn background_color() {
        let s = ansi_to_segments(b"\x1b[42mgreenbg\x1b[0m");
        assert_eq!(s[0].class, "hl-bg-2");
    }
}
