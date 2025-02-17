use std::{default::Default, ops::Range};

use flat_tree::tree;
use log_format::{ast::BuilderDetach, Format, Span};

use super::ast::{self, Container, Node, SiblingsIter};

// ---

pub trait FormatExt: Format {
    #[inline]
    fn parse_into<S>(&mut self, source: S, target: &mut Segment<S>) -> (Option<Span>, Result<(), Self::Error>)
    where
        S: AsRef<[u8]>,
    {
        target.set(source, self)
    }
}

impl<F> FormatExt for F where F: Format {}

// ---

#[derive(Debug)]
pub struct Segment<S> {
    source: Option<S>,
    container: Container,
}

impl<S> Segment<S>
where
    S: AsRef<[u8]>,
{
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn entries(&self) -> Entries {
        let source = self.source.as_ref().map(|x| x.as_ref());
        Entries {
            source,
            roots: self.container.roots(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            source: None,
            container: Container::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn build<F>(buf: S, format: &mut F) -> (Option<Span>, Result<Self, F::Error>)
    where
        F: Format + ?Sized,
    {
        let mut segment = Segment::new();
        let result = segment.set(buf, format);
        (result.0, result.1.map(|_| segment))
    }

    #[inline]
    pub fn set<F>(&mut self, buf: S, format: &mut F) -> (Option<Span>, Result<(), F::Error>)
    where
        F: Format + ?Sized,
    {
        self.container.clear();
        let mut end = 0;
        let mut target = self.container.metaroot();
        let result = loop {
            let result = format.parse(&buf.as_ref()[end..], target).detach();
            target = result.1;
            match result.0 {
                Ok(Some(span)) => end += span.end,
                Ok(None) => break Ok(()),
                Err(e) => break Err(e),
            }
        };
        self.source = Some(buf);
        let span = if end != 0 { Some(Span::with_end(end)) } else { None };
        (span, result)
    }

    #[inline]
    pub fn morph<B2, F>(self, buf: B2, format: &mut F) -> Result<Segment<B2>, F::Error>
    where
        B2: AsRef<[u8]>,
        F: Format + ?Sized,
    {
        let mut segment = Segment::<B2>::new();
        let result = segment.set(buf, format);
        result.1?;
        Ok(segment)
    }
}

impl<S> Default for Segment<S> {
    #[inline]
    fn default() -> Self {
        Self {
            source: None,
            container: Container::new(),
        }
    }
}

// ---

pub struct Entries<'s> {
    source: Option<&'s [u8]>,
    roots: tree::Roots<'s, ast::Value>,
}

impl<'s> Entries<'s> {
    #[inline]
    pub fn len(&self) -> usize {
        self.roots.len()
    }
}

impl<'s> IntoIterator for Entries<'s> {
    type Item = Entry<'s>;
    type IntoIter = EntriesIter<'s>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        EntriesIter {
            source: self.source,
            items: self.roots.into_iter(),
        }
    }
}

pub struct EntriesIter<'s> {
    source: Option<&'s [u8]>,
    items: tree::SiblingsIter<'s, ast::Value>,
}

impl<'s> Iterator for EntriesIter<'s> {
    type Item = Entry<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let source = self.source?;
        self.items.next().map(|node| match node.value() {
            ast::Value::Composite(ast::Composite::Object) => Object::new(source, node),
            _ => panic!("unexpected root value: {:?}", node.value()),
        })
    }
}

pub type Entry<'s> = Object<'s>;

// ---

#[derive(Debug, Clone)]
pub enum Value<'s> {
    Null,
    Bool(bool),
    Number(Number<'s>),
    String(String<'s>),
    Array(Array<'s>),
    Object(Object<'s>),
}

#[derive(Debug, Clone)]
pub struct Number<'s> {
    source: &'s [u8],
    span: Span,
}

impl<'s> Number<'s> {
    #[inline]
    fn new(source: &'s [u8], span: Span) -> Self {
        Self { source, span }
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.span
    }

    #[inline]
    pub fn text(&self) -> &'s [u8] {
        &self.source[Range::from(self.span)]
    }
}

#[derive(Debug, Clone)]
pub struct String<'s> {
    source: &'s [u8],
    span: Span,
}

impl<'s> String<'s> {
    #[inline]
    fn new(source: &'s [u8], span: Span) -> Self {
        Self { source, span }
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.span
    }

    #[inline]
    pub fn text(&self) -> &'s [u8] {
        &self.source[Range::from(self.span)]
    }
}

#[derive(Debug, Clone)]
pub struct Array<'s> {
    source: &'s [u8],
    node: Node<'s>,
}

impl<'s> Array<'s> {
    #[inline]
    fn new(source: &'s [u8], node: Node<'s>) -> Self {
        Self { source, node }
    }
}

impl<'s> IntoIterator for &Array<'s> {
    type Item = Value<'s>;
    type IntoIter = ArrayIter<'s>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ArrayIter {
            source: self.source,
            children: self.node.children().into_iter(),
        }
    }
}

pub struct ArrayIter<'s> {
    source: &'s [u8],
    children: SiblingsIter<'s>,
}

impl<'s> Iterator for ArrayIter<'s> {
    type Item = Value<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.children.next().map(|node| convert_value(self.source, node))
    }
}

#[derive(Debug, Clone)]
pub struct Object<'s> {
    source: &'s [u8],
    node: Node<'s>,
}

impl<'s> Object<'s> {
    #[inline]
    fn new(source: &'s [u8], node: Node<'s>) -> Self {
        Self { source, node }
    }
}

impl<'s> IntoIterator for &Object<'s> {
    type Item = (String<'s>, Value<'s>);
    type IntoIter = ObjectIter<'s>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ObjectIter {
            source: self.source,
            children: self.node.children().into_iter(),
        }
    }
}

pub struct ObjectIter<'s> {
    source: &'s [u8],
    children: SiblingsIter<'s>,
}

impl<'s> Iterator for ObjectIter<'s> {
    type Item = (String<'s>, Value<'s>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.children.next().map(|node| {
            let key = match node.value() {
                ast::Value::Composite(ast::Composite::Field(key)) => match key {
                    ast::String::Plain(span) => String::new(self.source, *span),
                    ast::String::JsonEscaped(span) => String::new(self.source, *span),
                },
                _ => unreachable!(),
            };
            let value = convert_value(self.source, node.children().into_iter().next().unwrap());
            (key, value)
        })
    }
}

#[inline]
fn convert_value<'s>(source: &'s [u8], node: Node<'s>) -> Value<'s> {
    match node.value() {
        ast::Value::Scalar(scalar) => match scalar {
            ast::Scalar::Null => Value::Null,
            ast::Scalar::Bool(b) => Value::Bool(*b),
            ast::Scalar::Number(span) => Value::Number(Number::new(source, *span)),
            ast::Scalar::String(ast::String::Plain(span)) => Value::String(String::new(source, *span)),
            ast::Scalar::String(ast::String::JsonEscaped(span)) => Value::String(String::new(source, *span)),
        },
        ast::Value::Composite(composite) => match composite {
            ast::Composite::Array => Value::Array(Array::new(source, node)),
            ast::Composite::Object => Value::Object(Object::new(source, node)),
            ast::Composite::Field(_) => unreachable!(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log_format_json::JsonFormat;

    #[test]
    fn test_segment() {
        let mut segment = Segment::new();
        let buf = br#"{"a":10}"#;
        let result = segment.set(buf, &mut JsonFormat);
        assert_eq!(result.0, Some(Span::with_end(8)));
        assert!(result.1.is_ok());
        assert_eq!(segment.entries().len(), 1);
        let entires = segment.entries().into_iter().collect::<Vec<_>>();
        assert_eq!(entires.len(), 1);
        let fields = entires[0].into_iter().collect::<Vec<_>>();
        assert_eq!(fields.len(), 1);
        let (key, value) = &fields[0];
        assert_eq!(key.text(), b"a");
        match value {
            Value::Number(number) => {
                assert_eq!(number.text(), b"10");
            }
            _ => panic!("unexpected value: {:?}", value),
        }
    }
}
