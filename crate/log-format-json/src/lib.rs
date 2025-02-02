use logos::Logos;
use upstream::{ast2, Format};

pub mod error;
pub mod lexer;
mod parse;
mod token;

#[cfg(test)]
mod tests;

pub use error::{Error, ErrorKind};
pub use lexer::Lexer;
pub use token::InnerToken;

// ---

pub struct JsonFormat;

impl Format for JsonFormat {
    type Error = Error;
    type Lexer<'s> = Lexer<'s>;

    fn lexer<'s>(s: &'s [u8]) -> Self::Lexer<'s> {
        Lexer::from_slice(s)
    }

    fn parse<'s, B>(s: &'s [u8], target: B) -> Result<(bool, B), (Self::Error, B)>
    where
        B: ast2::Build,
    {
        let mut lexer = InnerToken::lexer(s);
        parse::parse_object(&mut lexer, target)
    }
}
