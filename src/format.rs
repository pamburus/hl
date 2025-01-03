use crate::{error::Error, error::Result, model::v2::ast};

pub mod json;

pub trait Format {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> Result<()>;

    fn detect<'s>(&self, _input: &'s [u8]) -> Option<bool> {
        None
    }
}

pub struct Json;

impl Format for Json {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> Result<()> {
        let mut lexer = json::Lexer::new(std::str::from_utf8(input)?);
        json::parse_into(&mut lexer, target).map_err(|e| Error::FailedToParseJsonInput {
            message: e.0,
            start: e.1.start,
            end: e.1.end,
        })
    }

    fn detect<'s>(&self, input: &'s [u8]) -> Option<bool> {
        Some(input.starts_with(b"{"))
    }
}
