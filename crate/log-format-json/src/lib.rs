use logos::Logos;
use upstream::{ast, Format, Source};

pub mod error;
pub mod lexer;
mod parse;
mod token;

pub use error::{Error, ErrorKind};
pub use lexer::Lexer;
pub use token::Token;

// ---

pub struct JsonFormat;

impl Format for JsonFormat {
    type Error = Error;
    type Lexer<'s> = Lexer<'s>;

    #[inline]
    fn lexer<'s>(&self, s: &'s Source) -> Self::Lexer<'s> {
        Lexer::from_source(s)
    }

    #[inline]
    fn parse<'s, B>(&mut self, s: &'s Source, target: B) -> Result<(bool, B), (Self::Error, B)>
    where
        B: ast::Build<'s>,
    {
        let mut lexer = Token::lexer(s);
        parse::parse_object(&mut lexer, target)
    }
}
