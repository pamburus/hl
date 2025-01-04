pub use super::record::{
    build::{Builder, PriorityController, Settings},
    Record,
};
use crate::{error::Result, format::Format, model::v2::ast};

// ---

pub struct ParserSetup {
    settings: Settings,
}

impl ParserSetup {
    #[inline]
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    #[inline]
    pub fn new_parser(&self) -> Parser {
        Parser::new(&self.settings)
    }
}

type Span = std::ops::Range<usize>;

// ---

pub struct Parser<'settings, 's> {
    settings: &'settings Settings,
    container: ast::Container<'s>,
    record: Record<'s>,
}

impl<'settings, 's> Parser<'settings, 's> {
    pub fn new(settings: &'settings Settings) -> Self {
        Self {
            settings,
            container: ast::Container::new(),
            record: Record::default(),
        }
    }

    pub fn parse<'p, F>(&'p mut self, format: F, input: &'s [u8]) -> Result<Option<(&'p Record<'s>, Span)>>
    where
        F: Format,
    {
        self.container.clear();
        self.record = Record::default();

        let mut pc = PriorityController::default();
        let target = Builder::new(&self.settings, &mut pc, &mut self.record, self.container.metaroot());

        let Some(output) = format.parse(input, target)? else {
            return Ok(None);
        };

        Ok(Some((&self.record, output.span)))
    }
}
