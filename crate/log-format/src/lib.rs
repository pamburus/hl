pub mod ast;
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
    fn parse<B>(s: &[u8], target: B) -> Result<(bool, B), (Self::Error, B)>
    where
        B: ast::Build;
}

pub trait Lex: Iterator<Item = Result<Token, Self::Error>> {
    type Error;
}
