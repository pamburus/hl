// std imports
use std::sync::Arc;

// third-party imports
use crossbeam_queue::SegQueue;

// workspace imports
use log_ast::model::FormatExt;
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
    Fields, RawRecord, Record,
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
            recycled: SegQueue::new(),
        }
    }

    pub fn parse(&mut self, source: Arc<str>) -> impl Iterator<Record> {
        SegmentIter { source, parser: self }
    }

    pub fn recycle(&mut self, record: Record) {
        self.recycled.push(record.into());
    }
}

pub struct SegmentIter<'a, F: Format> {
    source: Arc<str>,
    parser: &'a Parser<F>,
}

impl<'a, F> Iterator for SegmentIter<'a, F>
where
    F: Format,
{
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Result<Record>> {
        // TODO: implement recycling
        let container = self
            .recycled
            .pop()
            .unwrap_or_else(|| ast::Container::with_capacity(256));

        let target = container.metaroot();
        let index = target.next_index();

        let mut record = Record::default();
        let mut pc = PriorityController::default();

        let target = Builder::new(
            &self.settings,
            &*self.source,
            &mut pc,
            &mut record,
            self.container.metaroot(),
        );

        let segment = match self.parser.format.parse_entry(self.source.clone(), target) {
            Ok(Some(segment)) => segment,
            Ok(None) => return None,
            Err(e) => Some(Err(e)),
        };

        let mut raw_record = RawRecord::new(segment, index);

        let Some(output) = self.format.parse(container) else {
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
