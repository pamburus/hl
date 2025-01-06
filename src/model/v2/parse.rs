pub use super::record::{
    build::{Builder, PriorityController, Settings},
    {Fields, Record},
};
use crate::{error::Result, format::Format, model::v2::ast};

// ---

pub trait NewParser {
    type Parser<'a, F: Format>: Parse<'a>;

    fn new_parser<'a, F: Format>(self, format: F, input: &'a [u8]) -> Self::Parser<'a>;
}

impl<'s> NewParser for &'s Settings {
    type Parser<'a, F: Format> = Parser<'s, 'a, F>;

    fn new_parser<'a, F: Format>(self, format: F, input: &'a [u8]) -> Parser {
        Parser::new(self, format, input)
    }
}

// ---

pub trait Parse<'a>: Iterator<Item = Result<Record<'a>>> {
    fn recycle(&mut self, record: Record<'a>);
}

// ---

pub struct Parser<'s, 'a, F: Format> {
    settings: &'s Settings,
    format: F,
    container: ast::Container<'a>,
}

impl<'s, 'a, F> Parser<'s, 'a, F>
where
    F: Format,
{
    fn new(settings: &'s Settings, format: F, input: &'a [u8]) -> Self {
        Self {
            settings,
            format,
            container: ast::Container::default(),
        }
    }
}

impl<'s, 'a, F> Iterator for Parser<'s, 'a, F>
where
    F: Format,
{
    type Item = Result<Record<'a>>;

    fn next(&mut self) -> Option<Result<Record<'a>>>
    where
        F: Format,
    {
        self.container.clear();
        self.container.reserve(128);

        let mut record = Record::default();
        let mut pc = PriorityController::default();

        let target = Builder::new(&self.settings, &mut pc, &mut record, self.container.metaroot());

        let Some(output) = self.format.parse(input, target)? else {
            return None;
        };

        if let Some(root) = self.container.roots().iter().next() {
            if let ast::Value::Composite(ast::Composite::Object) = root.value() {
                record.span = output.span;
                record.ast = std::mem::take(&mut self.container);

                return Some(Ok(record));
            }
        }

        None
    }
}

impl<'s, 'a, F> Parse<'a> for Parser<'s, 'a, F>
where
    F: Format,
{
    #[inline]
    fn recycle(&mut self, record: Record<'a>) {
        self.container = record.ast;
        self.container.clear();
    }
}
