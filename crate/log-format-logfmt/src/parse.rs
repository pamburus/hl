use super::{
    error::{Error, ErrorKind, MakeError},
    token::{Lexer, Token},
};
use upstream::{
    ast::Build,
    token::{Composite, String},
    Span,
};

// ---

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

            if token == Token::Eol {
                break;
            }

            let Token::Space = token else {
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
            Token::Space => continue,
            Token::Key(key) => return Ok((Some(key), target)),
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
        Token::Value(scalar) => target.add_scalar(scalar),
        _ => return Err((lexer.make_error(ErrorKind::UnexpectedEof).into(), target)),
    };

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use log_ast::ast::{Container, Value::*};
    use upstream::{
        ast::BuilderDetach,
        token::{Composite::*, Scalar::*, String::*},
        Span,
    };

    #[test]
    fn test_parse_line() {
        let input = br#"a=1 b=2 c=3"#;
        let mut lexer = Lexer::new(input);
        let mut container = Container::new();
        assert_eq!(parse_line(&mut lexer, container.metaroot()).detach().0.unwrap(), true);

        let mut roots = container.roots().iter();
        let root = roots.next().unwrap();
        assert!(roots.next().is_none());
        assert!(matches!(root.value(), Composite(Object)));

        let mut children = root.children().iter();
        let key = children.next().unwrap();
        if let &Composite(Field(Plain(span))) = key.value() {
            assert_eq!(span, Span::from(0..1));
        } else {
            panic!("expected field key");
        }
        let value = key.children().iter().next().unwrap();
        assert!(matches!(value.value(), Scalar(Number(Span { start: 2, end: 3 }))));

        let key = children.next().unwrap();
        assert!(matches!(
            key.value(),
            Composite(Field(Plain(Span { start: 4, end: 5 })))
        ));
        let value = key.children().iter().next().unwrap();
        assert!(matches!(value.value(), Scalar(Number(Span { start: 6, end: 7 }))));

        let key = children.next().unwrap();
        assert!(matches!(
            key.value(),
            Composite(Field(Plain(Span { start: 8, end: 9 })))
        ));
        let value = key.children().iter().next().unwrap();
        assert!(matches!(value.value(), Scalar(Number(Span { start: 10, end: 11 }))));

        assert!(children.next().is_none());
    }
}
