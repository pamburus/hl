// std imports
use std::sync::Arc;

use crossbeam_queue::SegQueue;
// workspace imports
use log_format::Format;

// local imports
use crate::{
    error::Result,
    format::{self},
    model::v2::ast,
};

// re-exports
pub use super::record::{
    build::{Builder, PriorityController, Settings},
    {Fields, Record},
};

// ---

pub struct Parser<F: Format> {
    settings: Arc<Settings>,
    format: F,
    recycled: SegQueue<ast::Segment>,
}

impl<F> Parser<F>
where
    F: Format,
{
    pub fn new(settings: Arc<Settings>, format: F) -> Self {
        Self {
            settings,
            format,
            segment: ast::Segment::with_capacity(1024),
        }
    }

    pub fn parse(&mut self, segment: Arc<str>) -> impl Iterator<Record> {}
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
