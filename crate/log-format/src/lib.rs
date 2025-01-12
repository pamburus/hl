pub mod origin;
pub mod span;
pub mod token;

pub use origin::Origin;
pub use span::Span;
pub use token::{Scalar, String, Token};

// ---

pub trait Format {
    type Lexer<'s>: Lex;

    fn lexer(s: &[u8]) -> Self::Lexer<'_>;
}

pub trait Lex: Iterator<Item = Result<Token, Self::Error>> {
    type Error;
}
