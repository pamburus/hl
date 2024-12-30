// local imports
use crate::{
    container::{Build, BuildExt, ScalarKind, StringKind},
    error::Result,
    token::{Lexer, Token},
};

// ---

pub(crate) fn parse_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Result<Option<T>> {
    if let Some(token) = lexer.next() {
        parse_value_token(lexer, target, token?).map(Some)
    } else {
        Ok(None)
    }
}

fn parse_value_token<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T, token: Token<'s>) -> Result<T> {
    let source = lexer.slice();

    match token {
        Token::Bool(b) => Ok(target.add_scalar(source, ScalarKind::Bool(b))),
        Token::BraceOpen => target.add_object(|target| parse_object(lexer, target)),
        Token::BracketOpen => target.add_array(|target| parse_array(lexer, target)),
        Token::Null => Ok(target.add_scalar(source, ScalarKind::Null)),
        Token::Number(_) => Ok(target.add_scalar(source, ScalarKind::Number)),
        Token::PlainString(_) => Ok(target.add_scalar(source, ScalarKind::String(StringKind::Plain))),
        Token::EscapedString(_) => Ok(target.add_scalar(source, ScalarKind::String(StringKind::Escaped))),
        _ => Err(("unexpected token here (context: value)", lexer.span())),
    }
}

/// Parse a token stream into an array and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '[' was consumed.
fn parse_array<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<T> {
    let span = lexer.span();
    let mut awaits_comma = false;
    let mut awaits_value = false;

    while let Some(token) = lexer.next() {
        let token = token?;
        match token {
            Token::BracketClose if !awaits_value => return Ok(target),
            Token::Comma if awaits_comma => awaits_value = true,
            _ => {
                target = parse_value_token(lexer, target, token)?;
                awaits_value = false;
            }
        }
        awaits_comma = !awaits_value;
    }
    Err(("unmatched opening bracket defined here", span))
}

/// Parse a token stream into an object and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '{' was consumed.
fn parse_object<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<T> {
    // let mut map = HashMap::new();
    let span = lexer.span();

    let insert = |lexer: &mut Lexer<'s>, target: T, kind| {
        target.add_field(|mut target| {
            target = target.add_key(lexer.slice(), kind);

            match lexer.next() {
                Some(Ok(Token::Colon)) => (),
                _ => return Err(("unexpected token here, expecting ':'", lexer.span())),
            }

            let Some(target) = parse_value(lexer, target)? else {
                return Err(("unexpected end of stream while expecting value", lexer.span()));
            };

            Ok(target)
        })
    };

    enum Awaits {
        Key,
        Comma,
    }

    let mut awaits = Awaits::Key;

    while let Some(token) = lexer.next() {
        match (token?, &mut awaits) {
            (Token::BraceClose, Awaits::Key | Awaits::Comma) => {
                return Ok(target);
            }
            (Token::Comma, Awaits::Comma) => {
                awaits = Awaits::Key;
            }
            (Token::PlainString(_), Awaits::Key) => {
                target = insert(lexer, target, StringKind::Plain)?;
                awaits = Awaits::Comma;
            }
            (Token::EscapedString(_), Awaits::Key) => {
                target = insert(lexer, target, StringKind::Escaped)?;
                awaits = Awaits::Comma;
            }
            _ => {
                return Err(("unexpected token here (context: object)", lexer.span()));
            }
        }
    }

    Err(("unmatched opening brace defined here", span))
}
