use upstream::{Build, Format};

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
    type Lexer<'s> = Lexer<'s>;

    fn lexer(s: &[u8]) -> Self::Lexer<'_> {
        Lexer::from_slice(s)
    }

    fn parse<B: Build>(s: &[u8], target: B) -> Result<B, B::Error> {
        let lexer = Self::lexer(s);
        result.map(|(_, target)| target)
    }
}
