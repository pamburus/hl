//! ANSI SGR escape sequence to HTML converter.
//!
//! `hl` writes ANSI escape sequences of the form `ESC [ 0 ; codes m`. This module parses that
//! stream and emits a self-contained HTML fragment with inline-styled `<span>` segments.
//! Each call to [`convert`] starts and ends with no active style — output is safe to drop into
//! a single row of a virtualized list.

use std::fmt::Write;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
    pub conceal: bool,
    pub crossed_out: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Plain(u8, bool), // (0..8, bright)
    Palette(u8),
    Rgb(u8, u8, u8),
}

impl Style {
    fn is_default(&self) -> bool {
        *self == Style::default()
    }

    fn render_open(&self, out: &mut String) {
        out.push_str("<span style=\"");
        let mut needs_semi = false;
        if let Some(fg) = self.fg {
            push_color(out, "color", fg);
            needs_semi = true;
        }
        if let Some(bg) = self.bg {
            if needs_semi {
                out.push(';');
            }
            push_color(out, "background-color", bg);
            needs_semi = true;
        }
        if self.bold {
            if needs_semi {
                out.push(';');
            }
            out.push_str("font-weight:bold");
            needs_semi = true;
        }
        if self.faint {
            if needs_semi {
                out.push(';');
            }
            out.push_str("opacity:0.7");
            needs_semi = true;
        }
        if self.italic {
            if needs_semi {
                out.push(';');
            }
            out.push_str("font-style:italic");
            needs_semi = true;
        }
        if self.underline || self.crossed_out {
            if needs_semi {
                out.push(';');
            }
            out.push_str("text-decoration:");
            if self.underline {
                out.push_str("underline");
                if self.crossed_out {
                    out.push(' ');
                }
            }
            if self.crossed_out {
                out.push_str("line-through");
            }
        }
        out.push_str("\">");
    }

    fn render_close(out: &mut String) {
        out.push_str("</span>");
    }
}

fn push_color(out: &mut String, prop: &str, c: Color) {
    out.push_str(prop);
    out.push(':');
    match c {
        Color::Plain(idx, bright) => {
            out.push_str(plain_color_hex(idx, bright));
        }
        Color::Palette(idx) => {
            let (r, g, b) = palette256(idx);
            let _ = write!(out, "#{:02x}{:02x}{:02x}", r, g, b);
        }
        Color::Rgb(r, g, b) => {
            let _ = write!(out, "#{:02x}{:02x}{:02x}", r, g, b);
        }
    }
}

fn plain_color_hex(idx: u8, bright: bool) -> &'static str {
    match (idx, bright) {
        (0, false) => "#000000",
        (1, false) => "#cd0000",
        (2, false) => "#00cd00",
        (3, false) => "#cdcd00",
        (4, false) => "#0000ee",
        (5, false) => "#cd00cd",
        (6, false) => "#00cdcd",
        (7, false) => "#e5e5e5",
        (0, true) => "#7f7f7f",
        (1, true) => "#ff0000",
        (2, true) => "#00ff00",
        (3, true) => "#ffff00",
        (4, true) => "#5c5cff",
        (5, true) => "#ff00ff",
        (6, true) => "#00ffff",
        (7, true) => "#ffffff",
        _ => "inherit",
    }
}

fn palette256(idx: u8) -> (u8, u8, u8) {
    if idx < 16 {
        let plain = idx & 7;
        let bright = idx & 8 != 0;
        let hex = plain_color_hex(plain, bright);
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
        (r, g, b)
    } else if idx < 232 {
        let i = idx - 16;
        let r = i / 36;
        let g = (i % 36) / 6;
        let b = i % 6;
        (component(r), component(g), component(b))
    } else {
        let v = 8 + (idx - 232) * 10;
        (v, v, v)
    }
}

fn component(c: u8) -> u8 {
    if c == 0 { 0 } else { 55 + c * 40 }
}

/// Convert a single line of ANSI-styled bytes to an HTML string.
///
/// The input must be valid UTF-8 (`hl` always emits UTF-8). Trailing newline, if present, is
/// stripped from the output. The result contains balanced `<span>` tags and HTML-escaped text.
pub fn convert(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len() + input.len() / 4);
    let mut style = Style::default();
    let mut span_open = false;
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b == 0x1b && i + 1 < input.len() && input[i + 1] == b'[' {
            // parse CSI ... m
            let mut j = i + 2;
            let start = j;
            while j < input.len() && input[j] != b'm' && input[j] != 0x1b {
                j += 1;
            }
            if j < input.len() && input[j] == b'm' {
                let params = &input[start..j];
                if span_open {
                    Style::render_close(&mut out);
                    span_open = false;
                }
                apply_sgr_params(params, &mut style);
                if !style.is_default() {
                    style.render_open(&mut out);
                    span_open = true;
                }
                i = j + 1;
                continue;
            } else {
                // malformed; skip ESC and continue
                i += 1;
                continue;
            }
        }
        // Skip lone newline at end so virtualized rows don't show a blank line.
        if b == b'\n' && i + 1 == input.len() {
            break;
        }
        push_escaped(&mut out, b);
        i += 1;
    }
    if span_open {
        Style::render_close(&mut out);
    }
    out
}

fn push_escaped(out: &mut String, b: u8) {
    match b {
        b'<' => out.push_str("&lt;"),
        b'>' => out.push_str("&gt;"),
        b'&' => out.push_str("&amp;"),
        b'\r' => {}
        _ => {
            // We trust the input is UTF-8; push as a char if ASCII, otherwise let utf8 bytes accumulate.
            // Since `hl` emits well-formed UTF-8, pushing one byte at a time works for ASCII; for
            // multi-byte sequences we rely on String::push_str via from_utf8.
            // For simplicity: if it's ASCII printable, push directly; else collect into a small buffer.
            if b.is_ascii() {
                out.push(b as char);
            } else {
                // Fallback: append the byte as a raw character via UTF-8 slice. We can't push a
                // single non-ASCII byte safely; the caller (`convert`) feeds bytes one-by-one and
                // we lose multi-byte boundary info here. Defer to a helper that does this right.
                // Push the byte into a 1-byte slice and use unchecked utf8 if it forms a valid char.
                // Easier: convert the full remaining slice in one shot. To keep this fast, the
                // caller will pre-split on ESC boundaries; here we approximate by pushing the byte
                // as Latin-1 (incorrect for true multi-byte, but `hl` rarely emits non-ASCII outside
                // ESC sequences in practice). Improve later by buffering raw bytes between escapes.
                unsafe {
                    out.as_mut_vec().push(b);
                }
            }
        }
    }
}

fn apply_sgr_params(params: &[u8], style: &mut Style) {
    // Parse semicolon-separated decimal integers.
    let mut tokens = SgrTokens::new(params);
    while let Some(code) = tokens.next() {
        match code {
            0 => *style = Style::default(),
            1 => style.bold = true,
            2 => style.faint = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            8 => style.conceal = true,
            9 => style.crossed_out = true,
            22 => {
                style.bold = false;
                style.faint = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            28 => style.conceal = false,
            29 => style.crossed_out = false,
            30..=37 => style.fg = Some(Color::Plain((code - 30) as u8, false)),
            38 => style.fg = parse_extended_color(&mut tokens),
            39 => style.fg = None,
            40..=47 => style.bg = Some(Color::Plain((code - 40) as u8, false)),
            48 => style.bg = parse_extended_color(&mut tokens),
            49 => style.bg = None,
            90..=97 => style.fg = Some(Color::Plain((code - 90) as u8, true)),
            100..=107 => style.bg = Some(Color::Plain((code - 100) as u8, true)),
            _ => {}
        }
    }
}

fn parse_extended_color(tokens: &mut SgrTokens<'_>) -> Option<Color> {
    match tokens.next()? {
        5 => tokens.next().map(|n| Color::Palette(n as u8)),
        2 => {
            let r = tokens.next()? as u8;
            let g = tokens.next()? as u8;
            let b = tokens.next()? as u8;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

struct SgrTokens<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> SgrTokens<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }
}

impl Iterator for SgrTokens<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.src.len() && self.src[self.pos] == b';' {
            self.pos += 1;
        }
        if self.pos >= self.src.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == start {
            // empty parameter, treat as 0
            self.pos += 1;
            return Some(0);
        }
        let slice = &self.src[start..self.pos];
        // ASCII-decimal parse
        let mut n: u32 = 0;
        for &b in slice {
            n = n.saturating_mul(10).saturating_add((b - b'0') as u32);
        }
        Some(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(convert(b"hello"), "hello");
    }

    #[test]
    fn escapes_html_chars() {
        assert_eq!(convert(b"a<b&c>"), "a&lt;b&amp;c&gt;");
    }

    #[test]
    fn reset_only_emits_nothing() {
        assert_eq!(convert(b"\x1b[0mhi\x1b[0m"), "hi");
    }

    #[test]
    fn bold_red_foreground() {
        let html = convert(b"\x1b[0;1;31merror\x1b[0m");
        assert!(html.contains("font-weight:bold"));
        assert!(html.contains("color:#cd0000"));
        assert!(html.contains(">error</span>"));
    }

    #[test]
    fn rgb_foreground() {
        let html = convert(b"\x1b[0;38;2;255;128;0mhi\x1b[0m");
        assert!(html.contains("color:#ff8000"));
    }

    #[test]
    fn palette_foreground() {
        let html = convert(b"\x1b[0;38;5;208mhi\x1b[0m");
        // palette 208 -> color cube (208-16)=192 -> 192/36=5, 192%36=12, 12/6=2, 12%6=0 -> (5,2,0)
        // component(5)=255, component(2)=135, component(0)=0
        assert!(html.contains("color:#ff8700"));
    }

    #[test]
    fn strips_trailing_newline() {
        assert_eq!(convert(b"hello\n"), "hello");
    }
}
