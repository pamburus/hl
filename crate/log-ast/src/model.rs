use super::ast::Container;
use log_format::{ast::BuilderDetach, Format, Span};

// ---

pub trait FormatExt: Format {
    #[inline]
    fn parse_into<S>(&mut self, source: S, target: &mut Segment<S>) -> (Option<Span>, Result<(), Self::Error>)
    where
        S: AsRef<[u8]> + Default,
    {
        target.set(source, self)
    }
}

impl<F> FormatExt for F where F: Format {}

pub struct Segment<B = [u8; 0]> {
    buf: Option<B>,
    container: Container,
}

impl Segment {
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: None,
            container: Container::with_capacity(capacity),
        }
    }
}

impl<S> Segment<S>
where
    S: AsRef<[u8]> + Default,
{
    #[inline]
    pub fn with_buf<B2: Default>(self) -> Segment<B2> {
        Segment {
            buf: Default::default(),
            container: self.container,
        }
    }

    #[inline]
    pub fn build<F>(buf: S, format: &mut F) -> Result<Self, F::Error>
    where
        F: Format + ?Sized,
    {
        Segment::new().morph(buf, format)
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
        B2: AsRef<[u8]> + Default,
        F: Format + ?Sized,
    {
        let mut segment = Segment::new().with_buf::<B2>();
        let result = segment.set(buf, format);
        result.1?;
        Ok(segment)
    }
}
