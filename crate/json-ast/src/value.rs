// std imports
use std::collections::HashMap;

// external imports
use logos::Lexer;

// lopcal imports
use crate::{error::Result, token::Token};

// ---

/// Represent any valid JSON value.
#[derive(Debug)]
pub enum Value<'s> {
    Null,
    Bool(bool),
    Number(&'s str),
    String(String<'s>),
    Array(Vec<Value<'s>>),
    Object(HashMap<String<'s>, Value<'s>>),
}

impl<'s> From<String<'s>> for Value<'s> {
    #[inline]
    fn from(s: String<'s>) -> Self {
        Value::String(s)
    }
}

impl<'s> From<bool> for Value<'s> {
    #[inline]
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

// ---

#[derive(PartialEq, Eq, Hash)]
pub enum String<'s> {
    Decoded(&'s str),
    Encoded(&'s str),
}

impl<'s> std::fmt::Debug for String<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decoded(s) => write!(f, "{:?}", s),
            Self::Encoded(s) => write!(f, "{:?}", s),
        }
    }
}

impl<'s> String<'s> {
    #[inline]
    pub fn from_plain(s: &'s str) -> Self {
        Self::Decoded(&s[1..s.len() - 1])
    }

    #[inline]
    pub fn from_escaped(s: &'s str) -> Self {
        Self::Encoded(s)
    }
}

// ---

/// Parse a token stream into a JSON value.
pub fn parse_value<'s>(lexer: &mut Lexer<'s, Token<'s>>) -> Result<Option<Value<'s>>> {
    if let Some(token) = lexer.next() {
        match token {
            Ok(Token::Bool(b)) => Ok(Value::Bool(b)),
            Ok(Token::BraceOpen) => parse_object(lexer),
            Ok(Token::BracketOpen) => parse_array(lexer),
            Ok(Token::Null) => Ok(Value::Null),
            Ok(Token::Number(n)) => Ok(Value::Number(n)),
            Ok(Token::PlainString(s)) => Ok(Value::String(String::from_plain(s))),
            Ok(Token::EscapedString(s)) => Ok(Value::String(String::from_escaped(s))),
            _ => Err(("unexpected token here (context: value)", lexer.span())),
        }
        .map(Some)
    } else {
        Ok(None)
    }
}

/// Parse a token stream into an array and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '[' was consumed.
fn parse_array<'s>(lexer: &mut Lexer<'s, Token<'s>>) -> Result<Value<'s>> {
    let mut array = Vec::new();
    let span = lexer.span();
    let mut awaits_comma = false;
    let mut awaits_value = false;

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::Bool(b)) if !awaits_comma => {
                array.push(Value::Bool(b));
                awaits_value = false;
            }
            Ok(Token::BraceOpen) if !awaits_comma => {
                let object = parse_object(lexer)?;
                array.push(object);
                awaits_value = false;
            }
            Ok(Token::BracketOpen) if !awaits_comma => {
                let sub_array = parse_array(lexer)?;
                array.push(sub_array);
                awaits_value = false;
            }
            Ok(Token::BracketClose) if !awaits_value => return Ok(Value::Array(array)),
            Ok(Token::Comma) if awaits_comma => awaits_value = true,
            Ok(Token::Null) if !awaits_comma => {
                array.push(Value::Null);
                awaits_value = false
            }
            Ok(Token::Number(n)) if !awaits_comma => {
                array.push(Value::Number(n));
                awaits_value = false;
            }
            Ok(Token::PlainString(s)) if !awaits_comma => {
                array.push(Value::String(String::from_plain(s)));
                awaits_value = false;
            }
            Ok(Token::EscapedString(s)) if !awaits_comma => {
                array.push(Value::String(String::from_escaped(s)));
                awaits_value = false;
            }
            _ => return Err(("unexpected token here (context: array)", lexer.span())),
        }
        awaits_comma = !awaits_value;
    }
    Err(("unmatched opening bracket defined here", span))
}

/// Parse a token stream into an object and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '{' was consumed.
fn parse_object<'s>(lexer: &mut Lexer<'s, Token<'s>>) -> Result<Value<'s>> {
    let mut map = HashMap::new();
    let span = lexer.span();
    let mut awaits_comma = false;
    let mut awaits_key = false;

    let mut insert = |lexer: &mut Lexer<'s, Token<'s>>, key: String<'s>| {
        match lexer.next() {
            Some(Ok(Token::Colon)) => (),
            _ => return Err(("unexpected token here, expecting ':'", lexer.span())),
        }
        if let Some(value) = parse_value(lexer)? {
            map.insert(key, value);
        } else {
            return Err(("unexpected end of stream while expecting value", lexer.span()));
        }
        Ok(())
    };

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::BraceClose) if !awaits_key => {
                return Ok(Value::Object(map));
            }
            Ok(Token::Comma) if awaits_comma => {
                awaits_key = true;
            }
            Ok(Token::PlainString(key)) if !awaits_comma => {
                insert(lexer, String::from_plain(key))?;
                awaits_key = false;
            }
            Ok(Token::EscapedString(key)) if !awaits_comma => {
                insert(lexer, String::from_escaped(key))?;
                awaits_key = false;
            }
            _ => {
                return Err(("unexpected token here (context: object)", lexer.span()));
            }
        }
        awaits_comma = !awaits_key;
    }

    Err(("unmatched opening brace defined here", span))
}
