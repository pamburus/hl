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
    type Lexer<'s>: Lex<'s>;

    fn lexer<'s>(&self, s: &'s Source) -> Self::Lexer<'s>;
    fn parse<'s, B>(&mut self, s: &'s Source, target: B) -> Result<(bool, B), (Self::Error, B)>
    where
        B: ast::Build<'s>;
}

pub trait Lex<'s>: Iterator<Item = Result<Token<'s>, Self::Error>> {
    type Error;

    fn span(&self) -> Span;
    fn bump(&mut self, n: usize);
}
