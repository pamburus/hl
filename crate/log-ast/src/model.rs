use super::ast::Container;
use log_format::{Format, Span};

// ---

pub trait FormatExt: Format {
    #[inline]
    fn parse_entry_into<S>(&mut self, source: S, target: &mut Segment<S>) -> Result<Option<Span>, Self::Error>
    where
        S: AsRef<[u8]> + Default,
    {
        target.set_to_first_entry(source, self)
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
    pub fn set_to_first_entry<F>(&mut self, buf: S, format: &mut F) -> Result<Option<Span>, F::Error>
    where
        F: Format + ?Sized,
    {
        self.container.clear();
        let result = format
            .parse(buf.as_ref(), self.container.metaroot())
            .map(|(span, _)| span)
            .map_err(|(e, _)| e);
        self.buf = Some(buf);
        result
    }

    #[inline]
    pub fn morph<B2, F>(self, buf: B2, format: &mut F) -> Result<Segment<B2>, F::Error>
    where
        B2: AsRef<[u8]> + Default,
        F: Format + ?Sized,
    {
        let mut segment = Segment::new().with_buf::<B2>();
        segment.set_to_first_entry(buf, format)?;
        Ok(segment)
    }
}
