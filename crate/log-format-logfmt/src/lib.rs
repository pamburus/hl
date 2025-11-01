use upstream::{Format, Span, ast};

pub mod error;
pub mod lexer;
mod parse;
mod token;

pub use error::{Error, ErrorKind};
pub use lexer::Lexer;
pub use token::Token;

// ---

pub struct LogfmtFormat;

impl Format for LogfmtFormat {
    type Error = Error;
    type Lexer<'s> = Lexer<'s>;

    #[inline]
    fn lexer<'s>(&self, s: &'s [u8]) -> Self::Lexer<'s> {
        Lexer::from_slice(s)
    }

    #[inline]
    fn parse<B>(&mut self, s: &[u8], target: B) -> Result<(Option<Span>, B), (Self::Error, B)>
    where
        B: ast::Build,
    {
        let mut lexer = Token::lexer(s);
        parse::parse_line(&mut lexer, target)
    }
}
