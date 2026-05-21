//! Wire `hl`'s parser + formatter together for use from WebAssembly.
//!
//! Mirrors the construction path in `hl::main` so the WASM output matches what `hl` would write to
//! the terminal: same theme (loaded from the embedded `config.toml`), same time format, same
//! formatting options, same punctuation.

use std::sync::Arc;

use hl::formatting::Expansion;
use hl::settings::{AsciiMode, FieldShowOption};
use hl::timezone::Tz;
use hl::{
    DateTimeFormatter, LinuxDateFormat, Parser, ParserSettings, RawRecordParser, RecordFormatter,
    RecordFormatterBuilder, Settings, Theme,
};

use crate::ansi_html;

pub struct Renderer {
    formatter: RecordFormatter,
    parser: Parser,
    raw_parser: RawRecordParser,
}

impl Renderer {
    /// Construct a renderer using the embedded default `config.toml` shipped with `hl`.
    pub fn new() -> Result<Self, String> {
        let settings = Settings::default();
        let theme = Theme::embedded_with_overlays(&settings.theme, &settings.theme_overlays)
            .map_err(|e| format!("failed to load embedded theme: {e}"))?;
        // Browsers can display Unicode unconditionally; pick the non-ASCII (Unicode) glyph set.
        let ascii_mode = AsciiMode::Off;
        let punctuation = Arc::new(settings.formatting.punctuation.resolve(ascii_mode));
        let tz = Tz::IANA(settings.time_zone);
        let time_format = LinuxDateFormat::new(&settings.time_format).compile();

        let formatter = RecordFormatterBuilder::new()
            .with_theme(Arc::new(theme))
            .with_timestamp_formatter(DateTimeFormatter::new(time_format, tz))
            .with_options(settings.formatting.clone())
            .with_punctuation(punctuation)
            .with_ascii(ascii_mode)
            .with_expansion(Expansion::from(settings.formatting.expansion.clone()))
            .with_always_show_time(settings.fields.predefined.time.show == FieldShowOption::Always)
            .with_always_show_level(settings.fields.predefined.level.show == FieldShowOption::Always)
            .build();

        let parser = Parser::new(ParserSettings::new(
            &settings.fields.predefined,
            &settings.fields.ignore,
            None,
        ));
        let raw_parser = RawRecordParser::new();

        Ok(Self {
            formatter,
            parser,
            raw_parser,
        })
    }

    /// Render a single log line (or a small batch of records concatenated by newlines) to HTML.
    ///
    /// Returns an empty string if the input does not parse as a log record. The output is a
    /// concatenation of `<span>`-wrapped fragments, suitable for `Element::innerHTML`.
    pub fn format(&self, line: &[u8]) -> String {
        let trimmed = trim_newline(line);
        if trimmed.is_empty() {
            return String::new();
        }

        let mut stream = self.raw_parser.parse(trimmed);
        let mut buf: Vec<u8> = Vec::with_capacity(trimmed.len() + 128);
        let mut wrote_any = false;
        while let Some(item) = stream.next() {
            let Ok(annotated) = item else {
                // Parse error: fall back to the raw text so the user still sees something useful.
                if wrote_any {
                    buf.push(b'\n');
                }
                buf.extend_from_slice(trimmed);
                wrote_any = true;
                break;
            };
            if wrote_any {
                buf.push(b'\n');
            }
            let record = self.parser.parse(&annotated.record);
            self.formatter.format_record(&mut buf, 0..0, &record);
            wrote_any = true;
        }

        if !wrote_any {
            return ansi_html::convert(trimmed);
        }

        ansi_html::convert(&buf)
    }
}

fn trim_newline(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    while end > 0 && (line[end - 1] == b'\n' || line[end - 1] == b'\r') {
        end -= 1;
    }
    &line[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_json_record() {
        let r = Renderer::new().expect("renderer init");
        let html = r.format(br#"{"time":"2024-01-01T00:00:00Z","level":"info","msg":"hello"}"#);
        assert!(!html.is_empty(), "expected non-empty HTML output");
        assert!(html.contains("hello"), "expected the message text to appear in output: {html}");
        // The default theme paints — there should be at least one color span.
        assert!(html.contains("<span style="), "expected styled spans; got: {html}");
    }

    #[test]
    fn formats_logfmt_record() {
        let r = Renderer::new().expect("renderer init");
        let html = r.format(b"time=2024-01-01T00:00:00Z level=info msg=hello");
        assert!(!html.is_empty());
        assert!(html.contains("hello"), "got: {html}");
    }

    #[test]
    fn empty_input_returns_empty() {
        let r = Renderer::new().expect("renderer init");
        assert_eq!(r.format(b""), "");
    }
}
