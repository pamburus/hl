// std imports
use std::str::Utf8Error;

// workspace imports
use encstr::EncodedString;
use flat_tree::tree;
use log_format::{ast::BuilderDetach, Format, Span};

use super::{
    ast::{self, Container, Node, SiblingsIter},
    source::{Slice, Source},
};

// ---

pub trait FormatExt: Format {
    #[inline]
    fn parse_entry<S>(&mut self, source: S, container: Container) -> Result<Option<Segment<S>>, Self::Error>
    where
        S: Source + Clone,
    {
        Segment::parse_entry_to_container(source, self, container)
    }

    #[inline]
    fn parse_segment<S>(&mut self, source: S, container: Container) -> Result<Option<Segment<S>>, Self::Error>
    where
        S: Source + Clone,
    {
        Segment::parse_to_container(source, self, container)
    }
}

impl<F> FormatExt for F where F: Format {}

// ---

#[derive(Debug)]
pub struct Segment<S> {
    source: S,
    span: Span,
    container: Container,
}

impl<S> Segment<S>
where
    S: Source + Clone,
{
    #[inline]
    pub fn entries(&self) -> Entries<S::Ref<'_>> {
        Entries {
            source: self.source.as_ref(),
            roots: self.container.roots(),
        }
    }

    #[inline]
    pub fn entry(&self, index: ast::Index) -> Option<Entry<S::Ref<'_>>> {
        self.container.nodes().get(index).and_then(|node| match node.value() {
            ast::Value::Composite(ast::Composite::Object) => Some(Object::new(self.source.as_ref(), node)),
            _ => None,
        })
    }

    #[inline]
    pub fn source(&self) -> &S::Slice<'_> {
        self.source.slice(self.span)
    }

    #[inline]
    pub fn parse<F>(source: S, format: &mut F) -> Result<Option<Self>, F::Error>
    where
        F: Format + ?Sized,
    {
        Self::parse_to_container(source, format, Container::new())
    }

    #[inline]
    pub fn parse_to_container<F>(source: S, format: &mut F, mut container: Container) -> Result<Option<Self>, F::Error>
    where
        F: Format + ?Sized,
    {
        container.clear();
        let mut end = 0;
        let mut target = container.metaroot();
        loop {
            let result = format.parse(&source.bytes()[end..], target).detach();
            target = result.1;
            match result.0 {
                Ok(Some(span)) => end += span.end,
                Ok(None) => {
                    if end == 0 {
                        break Ok(None);
                    }
                    break Ok(Some(Self {
                        source,
                        span: Span::with_end(end),
                        container,
                    }));
                }
                Err(e) => break Err(e),
            }
        }
    }

    #[inline]
    pub fn parse_entry_to_container<F>(
        source: S,
        format: &mut F,
        mut container: Container,
    ) -> Result<Option<Self>, F::Error>
    where
        F: Format + ?Sized,
    {
        let target = container.metaroot();
        let result = format.parse(source.bytes(), target).detach();
        match result.0 {
            Ok(Some(span)) => Ok(Some(Self {
                source,
                span,
                container,
            })),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl<S> From<Segment<S>> for Container {
    #[inline]
    fn from(segment: Segment<S>) -> Self {
        segment.container
    }
}

// ---

pub struct Entries<'s, S> {
    source: S,
    roots: tree::Roots<'s, ast::Value>,
}

impl<'s, S> Entries<'s, S> {
    #[inline]
    pub fn len(&self) -> usize {
        self.roots.len()
    }
}

impl<'s, S> IntoIterator for Entries<'s, S>
where
    S: Source + Clone,
{
    type Item = Entry<'s, S>;
    type IntoIter = EntriesIter<'s, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        EntriesIter {
            source: self.source,
            items: self.roots.into_iter(),
        }
    }
}

pub struct EntriesIter<'s, S> {
    source: S,
    items: tree::SiblingsIter<'s, ast::Value>,
}

impl<'s, S> Iterator for EntriesIter<'s, S>
where
    S: Source + Clone,
{
    type Item = Entry<'s, S>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let source = self.source.clone();
        self.items.next().map(|node| match node.value() {
            ast::Value::Composite(ast::Composite::Object) => Object::new(source, node),
            _ => panic!("unexpected root value: {:?}", node.value()),
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.items.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.items.count()
    }
}

pub type Entry<'s, S> = Object<'s, S>;

// ---

#[derive(Debug, Clone)]
pub enum Value<'s, S> {
    Null,
    Bool(bool),
    Number(Number<S>),
    String(String<S>),
    Array(Array<'s, S>),
    Object(Object<'s, S>),
}

#[derive(Debug, Clone)]
pub struct Number<S> {
    source: S,
    span: Span,
}

impl<S> Number<S>
where
    S: Source,
{
    #[inline]
    fn new(source: S, span: Span) -> Self {
        Self { source, span }
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.span
    }

    #[inline]
    pub fn source(&self) -> &S::Slice<'_> {
        self.source.slice(self.span)
    }
}

#[derive(Debug, Clone)]
pub struct String<S> {
    source: S,
    inner: ast::String,
}

impl<S> String<S>
where
    S: Source,
{
    #[inline]
    fn new(source: S, inner: ast::String) -> Self {
        Self { source, inner }
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.inner.span()
    }

    #[inline]
    pub fn get(&self) -> Result<EncodedString, Utf8Error> {
        match &self.inner {
            ast::String::Plain(span) => Ok(EncodedString::raw(self.source.slice(*span).str()?)),
            ast::String::JsonEscaped(span) => Ok(EncodedString::json(self.source.slice(*span).str()?)),
        }
    }

    #[inline]
    pub fn source(&self) -> &S::Slice<'_> {
        self.source.slice(self.span())
    }
}

#[derive(Debug, Clone)]
pub struct Array<'s, S> {
    source: S,
    node: Node<'s>,
}

impl<'s, S> Array<'s, S> {
    #[inline]
    fn new(source: S, node: Node<'s>) -> Self {
        Self { source, node }
    }
}

impl<'s, S> IntoIterator for &Array<'s, S>
where
    S: Source + Clone,
{
    type Item = Value<'s, S>;
    type IntoIter = ArrayIter<'s, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ArrayIter {
            source: self.source.clone(),
            children: self.node.children().into_iter(),
        }
    }
}

pub struct ArrayIter<'s, S> {
    source: S,
    children: SiblingsIter<'s>,
}

impl<'s, S> Iterator for ArrayIter<'s, S>
where
    S: Source + Clone,
{
    type Item = Value<'s, S>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.children
            .next()
            .map(|node| convert_value(self.source.clone(), node))
    }
}

#[derive(Debug, Clone)]
pub struct Object<'s, S> {
    source: S,
    node: Node<'s>,
}

impl<'s, S> Object<'s, S> {
    #[inline]
    fn new(source: S, node: Node<'s>) -> Self {
        Self { source, node }
    }
}

impl<'s, S> IntoIterator for &Object<'s, S>
where
    S: Source + Clone,
{
    type Item = Field<'s, S>;
    type IntoIter = ObjectIter<'s, S>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ObjectIter {
            source: self.source.clone(),
            children: self.node.children().into_iter(),
        }
    }
}

pub struct ObjectIter<'s, S> {
    source: S,
    children: SiblingsIter<'s>,
}

impl<'s, S> Iterator for ObjectIter<'s, S>
where
    S: Source + Clone,
{
    type Item = Field<'s, S>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.children.next().map(|node| {
            let key = match node.value() {
                ast::Value::Composite(ast::Composite::Field(key)) => String::new(self.source.clone(), *key),
                _ => unreachable!(),
            };
            let value = convert_value(self.source.clone(), node.children().into_iter().next().unwrap());

            Field::new(node.index(), key, value)
        })
    }
}

pub struct Field<'s, S> {
    index: ast::Index,
    key: String<S>,
    value: Value<'s, S>,
}

impl<'s, S> Field<'s, S>
where
    S: Source + Clone,
{
    #[inline]
    fn new(index: ast::Index, key: String<S>, value: Value<'s, S>) -> Self {
        Self { index, key, value }
    }

    #[inline]
    pub fn index(&self) -> ast::Index {
        self.index
    }

    #[inline]
    pub fn key(&self) -> &String<S> {
        &self.key
    }

    #[inline]
    pub fn value(&self) -> &Value<'s, S> {
        &self.value
    }
}

#[inline]
fn convert_value<'s, S>(source: S, node: Node<'s>) -> Value<'s, S>
where
    S: Source,
{
    match node.value() {
        ast::Value::Scalar(scalar) => match scalar {
            ast::Scalar::Null => Value::Null,
            ast::Scalar::Bool(b) => Value::Bool(*b),
            ast::Scalar::Number(span) => Value::Number(Number::new(source, *span)),
            ast::Scalar::String(s) => Value::String(String::new(source, *s)),
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
        let buf = r#"{"a":10}"#;
        let segment = Segment::parse(buf, &mut JsonFormat).unwrap().unwrap();
        assert_eq!(segment.source().len(), 8);
        assert_eq!(segment.entries().len(), 1);
        let entires = segment.entries().into_iter().collect::<Vec<_>>();
        assert_eq!(entires.len(), 1);
        let fields = entires[0].into_iter().collect::<Vec<_>>();
        assert_eq!(fields.len(), 1);
        let field = &fields[0];
        assert_eq!(field.key().source(), "a");
        match field.value() {
            Value::Number(number) => {
                assert_eq!(number.source(), "10");
            }
            _ => panic!("unexpected value: {:?}", field.value()),
        }
    }
}
