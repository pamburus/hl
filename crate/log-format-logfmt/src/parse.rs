use super::{
    error::{Error, ErrorKind, MakeError},
    token::{Lexer, Token},
};
use upstream::{
    ast::Build,
    source::ByteSlice,
    token::{Composite, Scalar, String},
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
fn parse_key<'s, B: Build>(lexer: &mut Lexer<'s>, target: B) -> Result<(Option<ByteSlice>, B), (Error, B)> {
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
    let empty = |lexer: &mut Lexer<'s>, target: B| {
        let slice = lexer.slice().slice(0..0);
        return Ok(target.add_scalar(Scalar::String(upstream::String::Plain(slice))));
    };

    let Some(token) = lexer.next() else {
        return empty(lexer, target);
    };

    let token = match token {
        Ok(token) => token,
        Err(e) => return Err((lexer.make_error(e).into(), target)),
    };

    let target = match token {
        Token::Value(scalar) => target.add_scalar(scalar),
        Token::Space => return empty(lexer, target),
        _ => return Err((lexer.make_error(ErrorKind::UnexpectedToken).into(), target)),
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
    };

    #[test]
    fn test_parse_line() {
        let input = br#"a=1 b=2 c=3"#.into();
        let mut lexer = Lexer::new(&input);
        let mut container = Container::new();
        assert_eq!(parse_line(&mut lexer, container.metaroot()).detach().0.unwrap(), true);

        let mut roots = container.roots().iter();
        let root = roots.next().unwrap();
        assert!(roots.next().is_none());
        assert!(matches!(root.value(), Composite(Object)));

        let mut children = root.children().iter();

        let key = children.next().unwrap();
        if let &Composite(Field(Plain(slice))) = &key.value() {
            assert_eq!(slice, b"a");
        } else {
            panic!("expected field key");
        }

        let value = key.children().iter().next().unwrap();
        if let &Scalar(Number(slice)) = &value.value() {
            assert_eq!(slice, b"1");
        } else {
            panic!("expected field value");
        }

        let key = children.next().unwrap();
        if let &Composite(Field(Plain(slice))) = &key.value() {
            assert_eq!(slice, b"b");
        } else {
            panic!("expected field key");
        }

        let value = key.children().iter().next().unwrap();
        if let &Scalar(Number(slice)) = &value.value() {
            assert_eq!(slice, b"2");
        } else {
            panic!("expected field value");
        }

        let key = children.next().unwrap();
        if let &Composite(Field(Plain(slice))) = &key.value() {
            assert_eq!(slice, b"c");
        } else {
            panic!("expected field key");
        }

        let value = key.children().iter().next().unwrap();
        if let &Scalar(Number(slice)) = &value.value() {
            assert_eq!(slice, b"3");
        } else {
            panic!("expected field value");
        }

        assert!(children.next().is_none());
    }
}
