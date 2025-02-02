use super::{
    error::{Error, ErrorKind, MakeError},
    token::InnerToken,
};
use upstream::{
    ast2::Build,
    token::{Composite, String},
    Span,
};

// ---

pub type Lexer<'s> = logos::Lexer<'s, InnerToken>;

#[inline]
pub fn parse_line<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<(bool, B), (Error, B)> {
    let (key, target) = match parse_key(lexer, target) {
        Ok((Some(key), target)) => (key, target),
        Ok((None, target)) => return Ok((false, target)),
        Err(e) => return Err(e),
    };

    let r = target.add_composite(Composite::Object, |mut target| {
        let mut key = key;
        loop {
            target = match target.add_composite(Composite::Field(String::Plain(key).into()), |target| {
                parse_value(lexer, target)
            }) {
                Ok(target) => target,
                Err(e) => return Err(e),
            };

            let Some(token) = lexer.next() else {
                break;
            };

            let token = match token {
                Ok(token) => token,
                Err(e) => return Err((lexer.make_error(e).into(), target)),
            };

            if token == InnerToken::Eol {
                break;
            }

            let InnerToken::Space = token else {
                return Err((lexer.make_error(ErrorKind::ExpectedSpace).into(), target));
            };

            match parse_key(lexer, target) {
                Ok((Some(k), t)) => {
                    target = t;
                    key = k;
                }
                Ok((None, t)) => {
                    target = t;
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(target)
    });

    r.map(|t| (true, t))
}

#[inline]
fn parse_key<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<(Option<Span>, B), (Error, B)> {
    loop {
        let Some(token) = lexer.next() else {
            return Ok((None, target));
        };

        let token = match token {
            Ok(token) => token,
            Err(e) => return Err((lexer.make_error(e).into(), target)),
        };

        match token {
            InnerToken::Space => continue,
            InnerToken::Key(key) => return Ok((Some(key), target)),
            _ => return Err((lexer.make_error(ErrorKind::ExpectedKey).into(), target)),
        }
    }
}

#[inline]
fn parse_value<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<B, (Error, B)> {
    let Some(token) = lexer.next() else {
        return Err((lexer.make_error(ErrorKind::UnexpectedEof).into(), target));
    };

    let token = match token {
        Ok(token) => token,
        Err(e) => return Err((lexer.make_error(e).into(), target)),
    };

    let target = match token {
        InnerToken::Scalar(scalar) => target.add_scalar(scalar),
        _ => return Err((lexer.make_error(ErrorKind::UnexpectedEof).into(), target)),
    };

    Ok(target)
}
