use super::{
    error::{Error, ErrorKind, MakeError},
    InnerToken,
};
use upstream::{
    ast::{Build, Discard},
    token::{Composite, Scalar},
};

// ---

pub type Lexer<'s> = logos::Lexer<'s, InnerToken>;

#[inline]
pub fn parse_value<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<(bool, B), (Error, B)> {
    if let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => return Err((lexer.make_error(e).into(), target)),
        };
        parse_value_token(lexer, target, token).map(|target| (true, target))
    } else {
        Ok((false, target))
    }
}

#[inline]
pub fn parse_object<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<(bool, B), (Error, B)> {
    if let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => return Err((lexer.make_error(e).into(), target)),
        };
        match token {
            InnerToken::BraceOpen => parse_object_inner(lexer, target).map(|target| (true, target)),
            _ => Err((lexer.make_error(ErrorKind::ExpectedObject).into(), target)),
        }
    } else {
        Ok((false, target))
    }
}

#[inline]
fn parse_field_value<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<B, (Error, B)> {
    match parse_value(lexer, target) {
        Ok((true, target)) => Ok(target),
        Ok((false, target)) => Err((lexer.make_error(ErrorKind::UnexpectedToken).into(), target)),
        Err(e) => Err(e),
    }
}

#[inline]
fn parse_value_token<'s, B: Build>(lexer: &mut Lexer<'s>, mut target: B, token: InnerToken) -> Result<B, (Error, B)> {
    match token {
        InnerToken::Scalar(scalar) => Ok(target.add_scalar(scalar)),
        InnerToken::BraceOpen => {
            let mut skipped = true;
            target = target.add_composite(Composite::Object, |target| {
                skipped = false;
                parse_object_inner(lexer, target)
            })?;
            if skipped {
                target = target.discard(|target| parse_object_inner(lexer, target))?;
            }
            Ok(target)
        }
        InnerToken::BracketOpen => {
            let mut skipped = true;
            target = target.add_composite(Composite::Array, |target| {
                skipped = false;
                parse_array_inner(lexer, target)
            })?;
            if skipped {
                target = target.discard(|target| parse_array_inner(lexer, target))?;
            }
            Ok(target)
        }
        _ => Err((lexer.make_error(ErrorKind::UnexpectedToken).into(), target)),
    }
}

#[inline]
fn parse_array_inner<'s, B: Build>(lexer: &mut Lexer<'s>, mut target: B) -> Result<B, (Error, B)> {
    let span = lexer.span();

    let mut awaits_comma = false;
    let mut awaits_value = false;

    while let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => return Err((lexer.make_error(e).into(), target)),
        };

        match token {
            InnerToken::BracketClose if !awaits_value => return Ok(target),
            InnerToken::Comma if awaits_comma => awaits_value = true,
            _ => {
                target = parse_value_token(lexer, target, token)?;
                awaits_value = false;
            }
        }
        awaits_comma = !awaits_value;
    }

    Err((span.make_error(ErrorKind::UnmatchedBracket).into(), target))
}

#[inline]
fn parse_object_inner<'s, B: Build>(lexer: &mut Lexer<'s>, mut target: B) -> Result<B, (Error, B)> {
    let span = lexer.span();

    enum Awaits {
        Key,
        Comma,
    }

    let mut awaits = Awaits::Key;

    while let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => return Err(((lexer.make_error(e).into()), target)),
        };

        match (token, &mut awaits) {
            (InnerToken::BraceClose, Awaits::Key | Awaits::Comma) => {
                return Ok(target);
            }
            (InnerToken::Comma, Awaits::Comma) => {
                awaits = Awaits::Key;
            }
            (InnerToken::Scalar(Scalar::String(s)), Awaits::Key) => {
                match lexer.next() {
                    Some(Ok(InnerToken::Colon)) => (),
                    _ => return Err((lexer.make_error(ErrorKind::UnexpectedToken).into(), target)),
                }

                let mut skipped = true;
                target = target.add_composite(Composite::Field(s), |target| {
                    skipped = false;
                    parse_field_value(lexer, target)
                })?;

                if skipped {
                    target = target.discard(|target| parse_field_value(lexer, target))?;
                }

                awaits = Awaits::Comma;
            }
            _ => return Err((lexer.make_error(ErrorKind::UnexpectedToken).into(), target)),
        }
    }

    Err((span.make_error(ErrorKind::UnmatchedBrace).into(), target))
}
