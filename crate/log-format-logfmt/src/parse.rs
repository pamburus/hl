use super::{
    error::{Error, ErrorKind, MakeError},
    token::{Lexer, Token},
};
use upstream::{
    Span,
    ast::Build,
    token::{Composite, Scalar, String},
};

// ---

#[inline]
pub fn parse_line<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<(Option<Span>, B), (Error, B)> {
    let start = lexer.span().end;

    let (key, target) = match parse_key(lexer, target) {
        Ok((Some(key), target)) => (key, target),
        Ok((None, target)) => return Ok((None, target)),
        Err(e) => return Err(e),
    };

    let r = target.add_composite(Composite::Object, |mut target| {
        let mut key = key;
        loop {
            target = match target.add_composite(Composite::Field(String::Plain(key)), |target| {
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
                Err(e) => return Err((lexer.make_error(e), target)),
            };

            if token == Token::Eol {
                break;
            }

            let Token::Space = token else {
                return Err((lexer.make_error(ErrorKind::ExpectedSpace), target));
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

    r.map(|t| (Some(Span::new(start, lexer.span().end)), t))
}

#[inline]
fn parse_key<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<(Option<Span>, B), (Error, B)> {
    loop {
        let Some(token) = lexer.next() else {
            return Ok((None, target));
        };

        let token = match token {
            Ok(token) => token,
            Err(e) => return Err((lexer.make_error(e), target)),
        };

        match token {
            Token::Space => continue,
            Token::Key(key) => return Ok((Some(key), target)),
            _ => return Err((lexer.make_error(ErrorKind::ExpectedKey), target)),
        }
    }
}

#[inline]
fn parse_value<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<B, (Error, B)> {
    let empty = |lexer: &mut Lexer<'s>, target: B| {
        let mut span = lexer.span();
        span.end = span.start;
        Ok(target.add_scalar(Scalar::String(upstream::String::Plain(span))))
    };

    let Some(token) = lexer.next() else {
        return empty(lexer, target);
    };

    let token = match token {
        Ok(token) => token,
        Err(e) => return Err((lexer.make_error(e), target)),
    };

    let target = match token {
        Token::Value(scalar) => target.add_scalar(scalar),
        Token::Space => return empty(lexer, target),
        _ => return Err((lexer.make_error(ErrorKind::UnexpectedToken), target)),
    };

    Ok(target)
}

#[cfg(test)]
mod tests;
