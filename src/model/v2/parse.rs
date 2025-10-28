pub use super::record::{
    build::{Builder, PriorityController, Settings},
    {Fields, Record},
};
use crate::{
    error::Result,
    format::{self, Format},
    model::v2::ast,
};

// ---

pub trait NewParser {
    type Parser<'a, P: format::Parse<'a>>: Parse<'a>;

    fn new_parser<'a, F: Format>(self, format: F, input: &'a [u8]) -> Result<Self::Parser<'a, F::Parser<'a>>>;
}

impl<'s> NewParser for &'s Settings {
    type Parser<'a, P: format::Parse<'a>> = Parser<'s, 'a, P>;

    fn new_parser<'a, F: Format>(self, format: F, input: &'a [u8]) -> Result<Self::Parser<'a, F::Parser<'a>>> {
        Ok(Parser::new(self, format.new_parser(input)?))
    }
}

// ---

pub trait Parse<'a>: Iterator<Item = Result<Record<'a>>> {
    fn recycle(&mut self, record: Record<'a>);
}

// ---

pub struct Parser<'s, 'a, P: format::Parse<'a>> {
    settings: &'s Settings,
    inner: P,
    container: ast::Container<'a>,
}

impl<'s, 'a, P> Parser<'s, 'a, P>
where
    P: format::Parse<'a>,
{
    pub fn new(settings: &'s Settings, inner: P) -> Self {
        Self {
            settings,
            inner,
            container: ast::Container::default(),
        }
    }
}

impl<'s, 'a, P> Iterator for Parser<'s, 'a, P>
where
    P: format::Parse<'a>,
{
    type Item = Result<Record<'a>>;

    fn next(&mut self) -> Option<Result<Record<'a>>> {
        self.container.clear();
        self.container.reserve(128);

        let mut record = Record::default();
        let mut pc = PriorityController::default();

        let target = Builder::new(&self.settings, &mut pc, &mut record, self.container.metaroot());

        let Some(output) = self.inner.parse(target) else {
            return None;
        };
        let span = match output {
            Err(e) => return Some(Err(e.into())),
            Ok(output) => output.span,
        };

        if let Some(root) = self.container.roots().iter().next() {
            if let ast::Value::Composite(ast::Composite::Object) = root.value() {
                record.span = span;
                record.ast = std::mem::take(&mut self.container);

                return Some(Ok(record));
            }
        }

        None
    }
}

impl<'s, 'a, P> Parse<'a> for Parser<'s, 'a, P>
where
    P: format::Parse<'a>,
{
    #[inline]
    fn recycle(&mut self, record: Record<'a>) {
        self.container = record.ast;
        self.container.clear();
    }
}

#[cfg(test)]
mod tests;
