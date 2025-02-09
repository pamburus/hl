use logos::Logos;
use upstream::{ast, Format};

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

    fn lexer<'s>(&self, s: &'s [u8]) -> Self::Lexer<'s> {
        Lexer::from_slice(s)
    }

    fn parse<'s, B>(&mut self, s: &'s [u8], target: B) -> Result<(bool, B), (Self::Error, B)>
    where
        B: ast::Build,
    {
        let mut lexer = Token::lexer(s);
        parse::parse_object(&mut lexer, target)
    }
}
