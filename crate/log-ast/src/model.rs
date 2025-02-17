use std::ops::Range;

use flat_tree::tree;
use log_format::{ast::BuilderDetach, Format, Span};

use super::ast::{self, Container, Node, SiblingsIter};

// ---

#[cfg(not(feature = "bytes"))]
type Source = str;

#[cfg(feature = "bytes")]
type Source = [u8];

pub trait FormatExt: Format {
    #[inline]
    fn parse_segment<S>(&mut self, source: S, container: Container) -> (Option<Span>, Result<Segment<S>, Self::Error>)
    where
        S: AsRef<Source>,
    {
        Segment::parse_to_container(source, self, container)
    }
}

impl<F> FormatExt for F where F: Format {}

// ---

#[derive(Debug)]
pub struct Segment<S> {
    source: S,
    container: Container,
}

impl<S> Segment<S>
where
    S: AsRef<Source>,
{
    #[inline]
    pub fn entries(&self) -> Entries {
        let source = self.source.as_ref();
        Entries {
            source,
            roots: self.container.roots(),
        }
    }

    pub fn source(&self) -> &Source {
        self.source.as_ref()
    }

    #[inline]
    pub fn parse<F>(source: S, format: &mut F) -> (Option<Span>, Result<Self, F::Error>)
    where
        F: Format + ?Sized,
    {
        Self::parse_to_container(source, format, Container::new())
    }

    #[inline]
    pub fn parse_to_container<F>(
        source: S,
        format: &mut F,
        mut container: Container,
    ) -> (Option<Span>, Result<Self, F::Error>)
    where
        F: Format + ?Sized,
    {
        container.clear();
        let mut end = 0;
        let mut target = container.metaroot();
        let result = loop {
            let result = format.parse(&source.as_ref().as_bytes()[end..], target).detach();
            target = result.1;
            match result.0 {
                Ok(Some(span)) => end += span.end,
                Ok(None) => break Ok(Self { source, container }),
                Err(e) => break Err(e),
            }
        };
        let span = if end != 0 { Some(Span::with_end(end)) } else { None };
        (span, result)
    }
}

impl<S> From<Segment<S>> for Container {
    #[inline]
    fn from(segment: Segment<S>) -> Self {
        segment.container
    }
}

// ---

pub struct Entries<'s> {
    source: &'s Source,
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
    source: &'s Source,
    items: tree::SiblingsIter<'s, ast::Value>,
}

impl<'s> Iterator for EntriesIter<'s> {
    type Item = Entry<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let source = self.source;
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
    source: &'s Source,
    span: Span,
}

impl<'s> Number<'s> {
    #[inline]
    fn new(source: &'s Source, span: Span) -> Self {
        Self { source, span }
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.span
    }

    #[inline]
    pub fn text(&self) -> &'s Source {
        slice(self.source, self.span)
    }
}

#[derive(Debug, Clone)]
pub struct String<'s> {
    source: &'s Source,
    span: Span,
}

impl<'s> String<'s> {
    #[inline]
    fn new(source: &'s Source, span: Span) -> Self {
        Self { source, span }
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.span
    }

    #[inline]
    pub fn text(&self) -> &'s Source {
        slice(self.source, self.span)
    }
}

#[derive(Debug, Clone)]
pub struct Array<'s> {
    source: &'s Source,
    node: Node<'s>,
}

impl<'s> Array<'s> {
    #[inline]
    fn new(source: &'s Source, node: Node<'s>) -> Self {
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
    source: &'s Source,
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
    source: &'s Source,
    node: Node<'s>,
}

impl<'s> Object<'s> {
    #[inline]
    fn new(source: &'s Source, node: Node<'s>) -> Self {
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
    source: &'s Source,
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
fn convert_value<'s>(source: &'s Source, node: Node<'s>) -> Value<'s> {
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

#[inline]
#[cfg(not(feature = "bytes"))]
fn slice(source: &Source, span: Span) -> &Source {
    // SAFETY: `span` is always within the bounds of `source`.
    // This is guaranteed by the parser.
    // The parser always returns valid spans.
    // The parser is the only way to create a `Segment`.
    // UTF-8 validation boundaries are also guaranteed by the parser.
    unsafe { std::str::from_utf8_unchecked(&source.as_bytes()[Range::from(span)]) }
}

#[inline]
#[cfg(feature = "bytes")]
fn slice(source: &Source, span: Span) -> &Source {
    source[Range::from(span)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use log_format_json::JsonFormat;

    #[test]
    fn test_segment() {
        let buf = r#"{"a":10}"#;
        let result = Segment::parse(buf, &mut JsonFormat);
        assert_eq!(result.0, Some(Span::with_end(8)));

        let segment = result.1.unwrap();
        assert_eq!(segment.entries().len(), 1);
        let entires = segment.entries().into_iter().collect::<Vec<_>>();
        assert_eq!(entires.len(), 1);
        let fields = entires[0].into_iter().collect::<Vec<_>>();
        assert_eq!(fields.len(), 1);
        let (key, value) = &fields[0];
        assert_eq!(key.text(), "a");
        match value {
            Value::Number(number) => {
                assert_eq!(number.text(), "10");
            }
            _ => panic!("unexpected value: {:?}", value),
        }
    }
}
