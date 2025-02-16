use bytes::Bytes;

// Token is a token in the log format.
// Each EntryBegin token must be followed by a sequence of tokens that ends with an EntryEnd token.
// If the corresponding EntryEnd token is missing, and a new EntryBegin token appears,
// the previous entry is considered to be invalid and should be discarded.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token {
    EntryBegin,
    EntryEnd,
    Scalar(Scalar),
    CompositeBegin(Composite),
    CompositeEnd,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Scalar {
    Null,
    Bool(bool),
    Number(Bytes),
    String(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum String {
    Plain(Bytes),
    JsonEscaped(Bytes),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Composite {
    Array,
    Object,
    Field(String),
}
