use logos::Span;

pub type Error = (&'static str, Span);

pub type Result<T> = std::result::Result<T, Error>;
