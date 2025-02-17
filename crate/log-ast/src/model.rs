use std::default::Default;

use log_format::{ast::BuilderDetach, Format, Span};

use super::ast::Container;

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
pub struct Segment<B> {
    buf: Option<B>,
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
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: None,
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
        self.buf = Some(buf);
        let span = if end != 0 { Some(Span::with_end(0)) } else { None };
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
            buf: None,
            container: Container::new(),
        }
    }
}
