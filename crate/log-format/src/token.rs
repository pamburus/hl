// Token is a token in the log format.
// Each EntryBegin token must be followed by a sequence of tokens that ends with an EntryEnd token.
// If the corresponding EntryEnd token is missing, and a new EntryBegin token appears,
// the previous entry is considered to be invalid and should be discarded.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token<'s> {
    EntryBegin,
    EntryEnd,
    Scalar(Scalar<'s>),
    CompositeBegin(Composite<'s>),
    CompositeEnd,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Scalar<'s> {
    Null,
    Bool(bool),
    Number(&'s [u8]),
    String(String<'s>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum String<'s> {
    Plain(&'s [u8]),
    JsonEscaped(&'s [u8]),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Composite<'s> {
    Array,
    Object,
    Field(String<'s>),
}
