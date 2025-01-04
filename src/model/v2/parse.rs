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

    pub fn parse<'s, F>(
        &self,
        state: &'s mut ParserState<'s>,
        format: F,
        input: &'s [u8],
    ) -> Result<(&'s mut ParserState<'s>, Option<Span>)>
    where
        F: Format,
    {
        state.container.clear();
        let mut record = Record::default();

        let mut pc = PriorityController::default();
        let target = Builder::new(&self.settings, &mut pc, &mut record, state.container.metaroot());

        let Some(output) = format.parse(input, target)? else {
            return Ok((state, None));
        };

        Ok((state, Some(output.span)))
    }

    pub fn make_record<'s>(&self, state: &'s ParserState<'s>) -> Record<'s> {
        let mut record = Record::default();

        if let Some(root) = state.container.roots().iter().next() {
            if let ast::Value::Composite(ast::Composite::Object) = root.value() {
                record.fields = Fields::new(root.children());
            }
        }

        record
    }
}

type Span = std::ops::Range<usize>;

// ---

#[derive(Default)]
pub struct ParserState<'s> {
    container: ast::Container<'s>,
}

impl ParserState<'_> {
    #[inline]
    pub fn clear(&mut self) {
        self.container.clear();
    }
}
