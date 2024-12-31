// local imports
use crate::{
    container::{Build, BuildExt, FlatBuild, FlatBuildExt, ScalarKind},
    error::Result,
    token::{Lexer, Token},
};

// ---

#[inline]
pub(crate) fn parse_value<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Result<Option<T>> {
    if let Some(token) = lexer.next() {
        parse_value_token(lexer, target, token.refine(lexer)?).map(Some)
    } else {
        Ok(None)
    }
}

#[inline]
pub(crate) fn parse_value_flat<'s, T: FlatBuild<'s>>(lexer: &mut Lexer<'s>, target: &mut T) -> Result<Option<()>> {
    if let Some(token) = lexer.next() {
        parse_value_token_flat(lexer, target, token.refine(lexer)?).map(Some)
    } else {
        Ok(None)
    }
}

#[inline]
fn parse_value_token<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, target: T, token: Token<'s>) -> Result<T> {
    let source = lexer.slice();

    match token {
        Token::Bool(b) => Ok(target.add_scalar(source, ScalarKind::Bool(b))),
        Token::BraceOpen => target.add_object(|target| parse_object(lexer, target)),
        Token::BracketOpen => target.add_array(|target| parse_array(lexer, target)),
        Token::Null => Ok(target.add_scalar(source, ScalarKind::Null)),
        Token::Number(_) => Ok(target.add_scalar(source, ScalarKind::Number)),
        Token::String(s) => Ok(target.add_scalar(source, ScalarKind::String(s.into()))),
        _ => Err(("unexpected token here (context: value)", lexer.span())),
    }
}

#[inline]
fn parse_value_token_flat<'s, T: FlatBuild<'s>>(lexer: &mut Lexer<'s>, target: &mut T, token: Token<'s>) -> Result<()> {
    let source = lexer.slice();

    match token {
        Token::Bool(b) => {
            target.scalar(source, ScalarKind::Bool(b));
        }
        Token::BraceOpen => {
            target.object_begin();
            parse_object_flat(lexer, target)?;
            target.object_end();
        }
        Token::BracketOpen => {
            target.array_begin();
            parse_array_flat(lexer, target)?;
            target.array_end();
        }
        Token::Null => {
            target.scalar(source, ScalarKind::Null);
        }
        Token::Number(_) => {
            target.scalar(source, ScalarKind::Number);
        }
        Token::String(s) => {
            target.scalar(source, ScalarKind::String(s.into()));
        }
        _ => {
            return Err(("unexpected token here (context: value)", lexer.span()));
        }
    }

    Ok(())
}

/// Parse a token stream into an array and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '[' was consumed.
#[inline]
fn parse_array<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<T> {
    let span = lexer.span();
    let mut awaits_comma = false;
    let mut awaits_value = false;

    while let Some(token) = lexer.next() {
        let token = token.refine(lexer)?;
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

/// Parse a token stream into an array and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '[' was consumed.
#[inline]
fn parse_array_flat<'s, T: FlatBuild<'s>>(lexer: &mut Lexer<'s>, target: &mut T) -> Result<()> {
    let span = lexer.span();
    let mut awaits_comma = false;
    let mut awaits_value = false;

    while let Some(token) = lexer.next() {
        let token = token.refine(lexer)?;
        match token {
            Token::BracketClose if !awaits_value => return Ok(()),
            Token::Comma if awaits_comma => awaits_value = true,
            _ => {
                parse_value_token_flat(lexer, target, token)?;
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
#[inline]
fn parse_object<'s, T: Build<'s>>(lexer: &mut Lexer<'s>, mut target: T) -> Result<T> {
    let span = lexer.span();

    enum Awaits {
        Key,
        Comma,
    }

    let mut awaits = Awaits::Key;

    while let Some(token) = lexer.next() {
        match (token.refine(lexer)?, &mut awaits) {
            (Token::BraceClose, Awaits::Key | Awaits::Comma) => {
                return Ok(target);
            }
            (Token::Comma, Awaits::Comma) => {
                awaits = Awaits::Key;
            }
            (Token::String(s), Awaits::Key) => {
                target = target.add_field(|mut target| {
                    target = target.add_key(lexer.slice(), s.into());

                    match lexer.next() {
                        Some(Ok(Token::Colon)) => (),
                        _ => return Err(("unexpected token here, expecting ':'", lexer.span())),
                    }

                    let Some(target) = parse_value(lexer, target)? else {
                        return Err(("unexpected end of stream while expecting value", lexer.span()));
                    };

                    Ok(target)
                })?;

                awaits = Awaits::Comma;
            }
            _ => {
                return Err(("unexpected token here (context: object)", lexer.span()));
            }
        }
    }

    Err(("unmatched opening brace defined here", span))
}

/// Parse a token stream into an object and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '{' was consumed.
#[inline]
fn parse_object_flat<'s, T: FlatBuild<'s>>(lexer: &mut Lexer<'s>, target: &mut T) -> Result<()> {
    let span = lexer.span();

    enum Awaits {
        Key,
        Comma,
    }

    let mut awaits = Awaits::Key;

    while let Some(token) = lexer.next() {
        match (token.refine(lexer)?, &mut awaits) {
            (Token::BraceClose, Awaits::Key | Awaits::Comma) => {
                return Ok(());
            }
            (Token::Comma, Awaits::Comma) => {
                awaits = Awaits::Key;
            }
            (Token::String(s), Awaits::Key) => {
                target.field_begin(lexer.slice(), s.into());

                match lexer.next() {
                    Some(Ok(Token::Colon)) => (),
                    _ => return Err(("unexpected token here, expecting ':'", lexer.span())),
                }

                if parse_value_flat(lexer, target)?.is_none() {
                    return Err(("unexpected end of stream while expecting value", lexer.span()));
                };

                target.field_end();

                awaits = Awaits::Comma;
            }
            _ => {
                return Err(("unexpected token here (context: object)", lexer.span()));
            }
        }
    }

    Err(("unmatched opening brace defined here", span))
}

// ---

trait RefineErr {
    type Output;

    fn refine(self, lexer: &Lexer) -> Result<Self::Output>;
}

impl<T> RefineErr for std::result::Result<T, ()> {
    type Output = T;

    #[inline]
    fn refine(self, lexer: &Lexer) -> Result<T> {
        self.map_err(|_| ("unexpected characters or end of stream", lexer.span()))
    }
}
