pub use crate::error::{Error, Result};
use crate::model::v2::ast;

pub mod auto;
pub mod json;
pub mod logfmt;

pub trait Format {
    type Lexer<'a>: Clone;
    type Parser<'a>: Parse<'a, Lexer = Self::Lexer<'a>>;

    fn new_lexer<'a>(&self, input: &'a [u8]) -> Result<Self::Lexer<'a>>;
    fn new_parser_from_lexer<'a>(&self, lexer: Self::Lexer<'a>) -> Self::Parser<'a>;
    fn new_parser<'a>(&self, input: &'a [u8]) -> Result<Self::Parser<'a>> {
        Ok(self.new_parser_from_lexer(self.new_lexer(input)?))
    }
    fn parse_from_lexer<'a, T: ast::Build<'a>>(&self, lexer: &mut Self::Lexer<'a>, target: T) -> ParseResult<T> {
        let mut parser = self.new_parser_from_lexer(lexer.clone());
        let result = parser.parse(target);
        *lexer = parser.into_lexer();
        result
    }
    fn parse<'a, T: ast::Build<'a>>(&self, input: &'a [u8], target: T) -> ParseResult<T> {
        let mut lexer = self.new_lexer(input)?;
        self.parse_from_lexer(&mut lexer, target)
    }
}

pub trait Parse<'a> {
    type Lexer: Clone;

    fn parse<T: ast::Build<'a>>(&mut self, target: T) -> ParseResult<T>;
    fn into_lexer(self) -> Self::Lexer;
}

// ---

pub struct Auto;

impl Format for Auto {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> ParseResult<T> {
        if let Some(true) = Json.detect(input) {
            return Json.parse(input, target);
        }

        return Logfmt.parse(input, target);
    }
}

// ---

pub struct Json;

impl Format for Json {
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> ParseResult<T> {
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
    fn parse<'s, T: ast::Build<'s>>(&self, input: &'s [u8], target: T) -> ParseResult<T> {
        let mut lexer = logfmt::Lexer::new(std::str::from_utf8(input)?);
        logfmt::parse_line(&mut lexer, target)
            .map_err(|e| Error::FailedToParseLogfmtInput {
                message: e.0,
                start: e.1.start,
                end: e.1.end,
            })
            .map(|x| {
                x.map(|target| ParseOutput {
                    span: 0..lexer.span().end,
                    target,
                })
            })
    }
}

// ---

type ParseResult<T> = Option<std::result::Result<ParseOutput<T>, ParseError<T>>>;

pub struct ParseOutput<T> {
    pub span: std::ops::Range<usize>,
    pub target: T,
}

pub struct ParseError<T> {
    pub error: Error,
    pub span: std::ops::Range<usize>,
    pub target: T,
}

impl<T> From<ParseError<T>> for Error {
    fn from(e: ParseError<T>) -> Self {
        e.error
    }
}
