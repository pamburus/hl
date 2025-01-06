pub use super::record::{
    build::{Builder, PriorityController, Settings},
    {Fields, Record},
};
use crate::{error::Result, format::Format, model::v2::ast};

// ---

pub struct Parser {
    settings: Settings,
}

impl Parser {
    #[inline]
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    pub fn new_unit<'p>(&'p self) -> Unit<'p, 'static> {
        Unit::new(&self)
    }
}

// ---

pub struct Unit<'p, 'a> {
    parser: &'p Parser,
    container: ast::Container<'a>,
}

impl<'p, 'a> Unit<'p, 'a> {
    fn new(parser: &'p Parser) -> Self {
        Self {
            parser,
            container: ast::Container::default(),
        }
    }

    pub fn parse<F>(&mut self, format: F, input: &'a [u8]) -> Result<Option<Record<'a>>>
    where
        F: Format,
    {
        self.container.clear();
        self.container.reserve(128);

        let mut record = Record::default();
        let mut pc = PriorityController::default();

        let target = Builder::new(&self.parser.settings, &mut pc, &mut record, self.container.metaroot());

        let Some(output) = format.parse(input, target)? else {
            return Ok(None);
        };

        if let Some(root) = self.container.roots().iter().next() {
            if let ast::Value::Composite(ast::Composite::Object) = root.value() {
                record.span = output.span;
                record.ast = std::mem::take(&mut self.container);

                return Ok(Some(record));
            }
        }

        Ok(None)
    }

    #[inline]
    pub fn recycle(&mut self, record: Record<'a>) {
        self.container = record.ast;
        self.container.clear();
    }
}
