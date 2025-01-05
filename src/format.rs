use crate::{error::Error, error::Result, model::v2::ast};

pub mod json;
pub mod logfmt;

pub trait Format {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> Result<Option<ParseOutput>>;

    fn detect<'s>(&self, _input: &'s [u8]) -> Option<bool> {
        None
    }
}

pub struct Auto;

impl Format for Auto {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> Result<Option<ParseOutput>> {
        if let Some(true) = Json.detect(input) {
            return Json.parse(input, target);
        }

        return Logfmt.parse(input, target);
    }
}

// ---

pub struct Json;

impl Format for Json {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> Result<Option<ParseOutput>> {
        let mut lexer = json::Lexer::new(std::str::from_utf8(input)?);
        json::parse_value(&mut lexer, target)
            .map_err(|e| Error::FailedToParseJsonInput {
                message: e.0,
                start: e.1.start,
                end: e.1.end,
            })
            .map(|x| {
                x.map(|_| ParseOutput {
                    span: 0..lexer.span().end,
                })
            })
    }

    fn detect<'s>(&self, input: &'s [u8]) -> Option<bool> {
        Some(input.starts_with(b"{"))
    }
}

// ---

pub struct Logfmt;

impl Format for Logfmt {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> Result<Option<ParseOutput>> {
        let mut lexer = logfmt::Lexer::new(std::str::from_utf8(input)?);
        logfmt::parse_line(&mut lexer, target)
            .map_err(|e| Error::FailedToParseLogfmtInput {
                message: e.0,
                start: e.1.start,
                end: e.1.end,
            })
            .map(|x| {
                x.map(|_| ParseOutput {
                    span: 0..lexer.span().end,
                })
            })
    }
}

// ---

pub struct ParseOutput {
    pub span: std::ops::Range<usize>,
}
