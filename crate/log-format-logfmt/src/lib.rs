use logos::Logos;
use upstream::{ast, Format};

pub mod error;
pub mod lexer;
mod parse;
mod token;

#[cfg(test)]
mod tests;

pub use error::{Error, ErrorKind};
pub use lexer::Lexer;
pub use token::Token;

// ---

pub struct LogfmtFormat;

impl Format for LogfmtFormat {
    type Error = Error;
    type Lexer<'s> = Lexer<'s>;

    #[inline]
    fn lexer<'s>(s: &'s [u8]) -> Self::Lexer<'s> {
        Lexer::from_slice(s)
    }

    #[inline]
    fn parse<'s, B>(s: &'s [u8], target: B) -> Result<(bool, B), (Self::Error, B)>
    where
        B: ast::Build,
    {
        let mut lexer = Token::lexer(s);
        parse::parse_line(&mut lexer, target)
    }
}
