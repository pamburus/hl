use super::Span;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token {
    EntryBegin,
    EntryEnd,
    Scalar(Scalar),
    CompositeBegin(Composite),
    CompositeEnd,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Scalar {
    Null,
    Bool(bool),
    Number(Span),
    String(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum String {
    Plain(Span),
    JsonEscaped(Span),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Composite {
    Array,
    Object,
    Field(String),
}
