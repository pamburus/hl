pub mod ast2;
pub mod origin;
pub mod span;
pub mod token;

pub use origin::Origin;
pub use span::Span;
pub use token::{Scalar, String, Token};

// ---

pub trait Format {
    type Error;
    type Lexer<'s>: Lex;

    fn lexer<'s>(s: &'s [u8]) -> Self::Lexer<'s>;
    fn parse<'s, B>(s: &'s [u8], target: B) -> Result<(bool, B), (B::Error, B)>
    where
        B: ast2::Build,
        B::Error: From<Self::Error>;
}

pub trait Lex: Iterator<Item = Result<Token, Self::Error>> {
    type Error;
}
