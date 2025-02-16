use super::ast::Container;
use log_format::Format;

// ---

pub struct Segment<B = [u8; 0]> {
    buf: B,
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
            buf: [0; 0],
            container: Container::with_capacity(capacity),
        }
    }
}

impl<B> Segment<B>
where
    B: AsRef<[u8]> + Default,
{
    #[inline]
    pub fn with_buf<B2: Default>(self) -> Segment<B2> {
        Segment {
            buf: Default::default(),
            container: self.container,
        }
    }

    #[inline]
    pub fn build<F>(buf: B, format: &mut F) -> Result<Self, F::Error>
    where
        F: Format,
    {
        Segment::new().morph(buf, format)
    }

    #[inline]
    pub fn set<F>(&mut self, buf: B, format: &mut F) -> Result<(), F::Error>
    where
        F: Format,
    {
        self.container.clear();
        format.parse(buf.as_ref(), self.container.metaroot()).map_err(|x| x.0)?;
        self.buf = buf;
        Ok(())
    }

    #[inline]
    pub fn morph<B2, F>(self, buf: B2, format: &mut F) -> Result<Segment<B2>, F::Error>
    where
        B2: AsRef<[u8]> + Default,
        F: Format,
    {
        let mut segment = Segment::new().with_buf::<B2>();
        segment.set(buf, format)?;
        Ok(segment)
    }
}
