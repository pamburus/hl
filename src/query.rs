// third-party imports
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde_json as json;

// local imports
use crate::error::Result;
use crate::model::{FieldFilter, Record, RecordFilter};

// ---

#[derive(Parser)]
#[grammar = "query.pest"]
pub struct QueryParser;

pub type Query = Box<dyn RecordFilter + Sync>;

// ---

pub fn parse(str: &str) -> Result<Query> {
    let mut pairs = QueryParser::parse(Rule::whole, str)?;
    Ok(new_query(pairs.next().unwrap())?)
}

fn new_query(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::query);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::logical_binary_op => new_logical_binary_op(inner),
        Rule::statement => new_statement(inner),
        _ => unreachable!(),
    }
}

fn new_logical_binary_op(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::logical_binary_op);

    let mut inner = pair.into_inner();
    let lhs = new_statement(inner.next().unwrap())?;
    let op = inner.next().unwrap();
    let rhs = new_query(inner.next().unwrap())?;
    match op.as_rule() {
        Rule::and => Ok(Box::new(And { lhs, rhs })),
        Rule::or => Ok(Box::new(Or { lhs, rhs })),
        Rule::xor => Ok(Box::new(Xor { lhs, rhs })),
        _ => unreachable!(),
    }
}

fn new_statement(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::statement);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::logical_unary_op => new_logical_unary_op(inner),
        Rule::primary => new_primary(inner),
        _ => unreachable!(),
    }
}

fn new_logical_unary_op(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::logical_unary_op);

    let mut inner = pair.into_inner();
    let op = inner.next().unwrap();
    let arg = new_primary(inner.next().unwrap())?;
    match op.as_rule() {
        Rule::not => Ok(Box::new(Not { arg })),
        _ => unreachable!(),
    }
}

fn new_primary(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::primary);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::query => new_query(inner),
        Rule::term => new_term(inner),
        _ => unreachable!(),
    }
}

fn new_term(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::term);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::field_filter => new_field_filter(inner),
        _ => unreachable!(),
    }
}

fn new_field_filter(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::field_filter);

    let mut inner = pair.into_inner();
    let lhs = inner.next().unwrap().as_str();
    let op = inner.next().unwrap().as_str();
    let rhs = new_string(inner.next().unwrap())?;
    Ok(Box::new(FieldFilter::parse(&format!("{}{}{}", lhs, op, rhs))?))
}

fn new_string(pair: Pair<Rule>) -> Result<String> {
    assert_eq!(pair.as_rule(), Rule::string);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::json_string => Ok(json::from_str(inner.as_str())?),
        Rule::simple_string => Ok(inner.as_str().into()),
        _ => unreachable!(),
    }
}

// ---

struct And {
    lhs: Query,
    rhs: Query,
}

impl RecordFilter for And {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        self.lhs.apply(record) && self.rhs.apply(record)
    }
}

// ---

struct Or {
    lhs: Query,
    rhs: Query,
}

impl RecordFilter for Or {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        self.lhs.apply(record) || self.rhs.apply(record)
    }
}

// ---

struct Xor {
    lhs: Query,
    rhs: Query,
}

impl RecordFilter for Xor {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        self.lhs.apply(record) != self.rhs.apply(record)
    }
}

// ---

struct Not {
    arg: Query,
}

impl RecordFilter for Not {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        !self.arg.apply(record)
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let mut pairs = QueryParser::parse(Rule::whole, ".a=10").unwrap();
        let p1 = pairs.next().unwrap();
        assert_eq!(p1.as_rule(), Rule::statement);
        let mut pi1 = p1.into_inner();
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next(), None);
    }
}
