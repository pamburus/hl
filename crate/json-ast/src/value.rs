// std imports
use std::collections::HashMap;

// external imports
use logos::{Lexer, Span};

// lopcal imports
use crate::token::Token;

// ---

type Error = (&'static str, Span);

type Result<T> = std::result::Result<T, Error>;

/// Represent any valid JSON value.
#[derive(Debug)]
pub enum Value<'s> {
    Null,
    Bool(bool),
    Number(&'s str),
    String(&'s str),
    Array(Vec<Value<'s>>),
    Object(HashMap<&'s str, Value<'s>>),
}

/// Parse a token stream into a JSON value.
pub fn parse_value<'s>(lexer: &mut Lexer<'s, Token<'s>>) -> Result<Option<Value<'s>>> {
    if let Some(token) = lexer.next() {
        match token {
            Ok(Token::Bool(b)) => Ok(Value::Bool(b)),
            Ok(Token::BraceOpen) => parse_object(lexer),
            Ok(Token::BracketOpen) => parse_array(lexer),
            Ok(Token::Null) => Ok(Value::Null),
            Ok(Token::Number(n)) => Ok(Value::Number(n)),
            Ok(Token::String(s)) => Ok(Value::String(s)),
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
            Ok(Token::String(s)) if !awaits_comma => {
                array.push(Value::String(s));
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

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::BraceClose) if !awaits_key => return Ok(Value::Object(map)),
            Ok(Token::Comma) if awaits_comma => awaits_key = true,
            Ok(Token::String(key)) if !awaits_comma => {
                match lexer.next() {
                    Some(Ok(Token::Colon)) => (),
                    _ => return Err(("unexpected token here, expecting ':'", lexer.span())),
                }
                if let Some(value) = parse_value(lexer)? {
                    map.insert(key, value);
                    awaits_key = false;
                } else {
                    return Err(("unexpected end of stream while expecting value", lexer.span()));
                }
            }
            _ => return Err(("unexpected token here (context: object)", lexer.span())),
        }
        awaits_comma = !awaits_key;
    }
    Err(("unmatched opening brace defined here", span))
}
