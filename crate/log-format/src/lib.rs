pub mod ast;
pub mod origin;
pub mod source;
pub mod span;
pub mod token;

pub use origin::Origin;
pub use source::Source;
pub use span::Span;
pub use token::{Scalar, String, Token};

// ---

pub trait Format {
    type Error;
    type Lexer<'s>: Lex;

    fn lexer<'s>(&self, s: &'s Source) -> Self::Lexer<'s>;
    fn parse<B>(&mut self, s: &Source, target: B) -> Result<(bool, B), (Self::Error, B)>
    where
        B: ast::Build;
}

pub trait Lex: Iterator<Item = Result<Token, Self::Error>> {
    type Error;

    fn span(&self) -> Span;
    fn bump(&mut self, n: usize);
}
