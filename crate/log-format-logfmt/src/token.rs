use logos::Logos;

use upstream::{
    token::{Scalar, String},
    Span,
};

use super::ErrorKind;

// Token is a token in the logfmt format.
// Key token must be followed by a Value token.
// If the corresponding Value token is missing, and a space token appears,
// the value is considered to be an empty string.
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Key(Span),
    Value(Scalar),
    Space,
    Eol,
}

impl Token {
    #[inline]
    pub fn lexer<'s>(s: &'s [u8]) -> Lexer<'s> {
        Lexer(Mode::M1(T1::lexer(s)))
    }
}

impl From<T1> for Token {
    #[inline]
    fn from(token: T1) -> Self {
        match token {
            T1::Key(span) => Token::Key(span),
            T1::Space => Token::Space,
            T1::Eol => Token::Eol,
        }
    }
}

impl From<T2> for Token {
    #[inline]
    fn from(token: T2) -> Self {
        match token {
            T2::Value(scalar) => Token::Value(scalar),
        }
    }
}

// ---

// Lexer allows to iterate over tokens in the input.
pub struct Lexer<'s>(Mode<'s>);

impl<'s> Lexer<'s> {
    #[inline]
    pub fn new(s: &'s [u8]) -> Self {
        Token::lexer(s)
    }

    #[inline]
    pub fn span(&self) -> Span {
        match &self.0 {
            Mode::M1(lexer) => lexer.span().into(),
            Mode::M2(lexer) => lexer.span().into(),
        }
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Result<Token, ErrorKind>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            Mode::M1(lexer) => match lexer.next() {
                Some(Ok(token)) => {
                    if let T1::Key(_) = token {
                        self.0 = Mode::M2(lexer.clone().morph());
                    }
                    Some(Ok(Token::from(token)))
                }
                Some(Err(e)) => Some(Err(e)),
                None => None,
            },
            Mode::M2(lexer) => {
                let result = match lexer.next() {
                    Some(Ok(token)) => Some(Ok(Token::from(token))),
                    Some(Err(e)) => Some(Err(e)),
                    None => None,
                };
                self.0 = Mode::M1(lexer.clone().morph());
                result
            }
        }
    }
}

// Mode defines the current lexer mode.
// The lexer starts in mode M1 and switches to mode M2 when it encounters a key.
// The lexer switches back to mode M1 when it reaches the end of the value.
enum Mode<'s> {
    M1(logos::Lexer<'s, T1>),
    M2(logos::Lexer<'s, T2>),
}

// ---

// T1 is a token for mode M1.
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(source = [u8])]
#[logos(error = ErrorKind)]
enum T1 {
    #[regex(r#"[^"\x00-\x20='(),;<>\[\]\\\^`{}|\x7F]+="#, |lex| Span::from(lex.span()).cut_right(1))]
    Key(Span),

    #[regex(r#"[\t ]+"#)]
    Space,

    #[regex(r"\r\n|\r|\n")]
    Eol,
}

// T2 is a token for mode M2.
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(source = [u8])]
#[logos(error = ErrorKind)]
enum T2 {
    #[token("null", |_| Scalar::Null)]
    #[token("false", |_| Scalar::Bool(false))]
    #[token("true", |_| Scalar::Bool(true))]
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| Scalar::Number(lex.span().into()), priority = 6)]
    #[regex(r#"[^"\x00-\x20=]+"#, |lex| Scalar::String(String::Plain(lex.span().into())), priority = 5)]
    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| Scalar::String(String::JsonEscaped(lex.span().into())), priority = 4)]
    Value(Scalar),
}
