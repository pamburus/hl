// third-party imports
use pest::Parser;
use pest_derive::Parser;

// local imports
use crate::error::Result;
use crate::model::{FieldFilter, RecordFilter};

// ---

#[derive(Parser)]
#[grammar = "query.pest"]
pub struct QueryParser;

// ---

struct RecordFilterQuery {
    q: Box<dyn RecordFilter>,
}

impl RecordFilterQuery {
    pub fn parse(str: &str) -> Result<Self> {
        QueryParser::parse(Rule::whole, str)?;
        Ok(Self {
            q: Box::new(FieldFilter::parse(str)?),
        })
    }
}
