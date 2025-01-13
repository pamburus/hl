pub mod build;
pub mod origin;
pub mod span;
pub mod token;

pub use build::Build;
pub use origin::Origin;
pub use span::Span;
pub use token::{Scalar, String, Token};

// ---

pub trait Format {
    type Lexer<'s>: Lex;

    fn lexer(s: &[u8]) -> Self::Lexer<'_>;
    fn parse<B: Build>(s: &[u8], target: B) -> Result<B, B::Error>;
}

pub trait Lex: Iterator<Item = Result<Token, Self::Error>> {
    type Error;
}
