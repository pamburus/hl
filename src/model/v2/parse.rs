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

    pub fn new_state(&self) -> ParserState {
        ParserState::default()
    }

    pub fn parse<'s, F>(&self, state: &mut ParserState<'s>, format: F, input: &'s [u8]) -> Result<Option<Record<'s>>>
    where
        F: Format,
    {
        state.container.clear();
        state.container.reserve(128);
        let mut record = Record::default();

        let mut pc = PriorityController::default();
        let target = Builder::new(&self.settings, &mut pc, &mut record, state.container.metaroot());

        let Some(output) = format.parse(input, target)? else {
            return Ok(None);
        };

        if let Some(root) = state.container.roots().iter().next() {
            if let ast::Value::Composite(ast::Composite::Object) = root.value() {
                record.span = output.span;
                record.ast = std::mem::take(&mut state.container);

                return Ok(Some(record));
            }
        }

        Ok(None)
    }
}

// ---

#[derive(Default)]
pub struct ParserState<'s> {
    container: ast::Container<'s>,
}

impl<'s> ParserState<'s> {
    #[inline]
    pub fn clear(&mut self) {
        self.container.clear();
    }

    pub fn consume(&mut self, record: Record<'s>) {
        self.container = record.ast;
    }
}
