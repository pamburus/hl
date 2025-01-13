use super::{
    error::{Error, ErrorKind, MakeError},
    InnerToken,
};
use upstream::{
    build::{Hitch, HitchResult, Unhitch},
    token::Composite,
    Build,
};

// ---

type Lexer<'s> = logos::Lexer<'s, InnerToken>;

#[inline]
pub fn parse_value<'s, T: Build>(lexer: &mut Lexer<'s>, target: T) -> HitchResult<Option<()>, T::Error, T>
where
    T::Error: From<Error>,
{
    if let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => return Err((lexer.make_error(e).into(), target)),
        };
        parse_value_token(lexer, target, token).map(|(x, target)| (Some(x), target))
    } else {
        Ok((None, target))
    }
}

#[inline]
fn parse_value_token<'s, T: Build>(lexer: &mut Lexer<'s>, target: T, token: InnerToken) -> HitchResult<(), T::Error, T>
where
    T::Error: From<Error>,
{
    match token {
        InnerToken::Scalar(scalar) => Ok(((), target.add_scalar(scalar))),
        InnerToken::BraceOpen => target.add_composite(Composite::Object, |target| parse_object(lexer, target)),
        InnerToken::BracketOpen => target.add_composite(Composite::Array, |target| parse_array(lexer, target)),
        _ => Err((lexer.make_error(ErrorKind::UnexpectedToken).into(), target)),
    }
}

#[inline]
fn parse_array<'s, T: Build>(lexer: &mut Lexer<'s>, mut target: T) -> HitchResult<(), T::Error, T>
where
    T::Error: From<Error>,
{
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
    Err(("unmatched opening bracket defined here", span))
}

#[inline]
fn parse_object<'s, T: Build>(lexer: &mut Lexer<'s>, mut target: T) -> HitchResult<(), T::Error, T> {
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
