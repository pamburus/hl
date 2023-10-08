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
    let mut pairs = QueryParser::parse(Rule::input, str)?;
    Ok(expression(pairs.next().unwrap())?)
}

fn expression(pair: Pair<Rule>) -> Result<Query> {
    match pair.as_rule() {
        Rule::or => binary_op::<Or>(pair),
        Rule::xor => binary_op::<Xor>(pair),
        Rule::and => binary_op::<And>(pair),
        Rule::not => not(pair),
        Rule::primary => primary(pair),
        _ => unreachable!(),
    }
}

fn binary_op<Op: BinaryOp + Sync + 'static>(pair: Pair<Rule>) -> Result<Query> {
    let mut inner = pair.into_inner();
    let mut result = expression(inner.next().unwrap())?;
    for inner in inner {
        result = Box::new(Op::new(result, expression(inner)?));
    }
    Ok(result)
}

fn not(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::not);

    Ok(Box::new(Not {
        arg: expression(pair.into_inner().next().unwrap())?,
    }))
}

fn primary(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::primary);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::term => term(inner),
        _ => expression(inner),
    }
}

fn term(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::term);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::field_filter => field_filter(inner),
        _ => unreachable!(),
    }
}

fn field_filter(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::field_filter);

    let mut inner = pair.into_inner();
    let lhs = inner.next().unwrap().as_str();
    let op = match inner.next().unwrap().as_rule() {
        Rule::equal => "=",
        Rule::not_equal => "!=",
        Rule::like => "~=",
        Rule::not_like => "!~=",
        Rule::regex_match => "~~=",
        Rule::not_regex_match => "!~~=",
        _ => unreachable!(),
    };
    let rhs = string(inner.next().unwrap())?;
    Ok(Box::new(FieldFilter::parse(&format!("{}{}{}", lhs, op, rhs))?))
}

fn string(pair: Pair<Rule>) -> Result<String> {
    assert_eq!(pair.as_rule(), Rule::string);

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::json_string => Ok(json::from_str(inner.as_str())?),
        Rule::simple_string => Ok(inner.as_str().into()),
        _ => unreachable!(),
    }
}

// ---

trait BinaryOp: RecordFilter {
    fn new(lhs: Query, rhs: Query) -> Self;
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

impl BinaryOp for Or {
    fn new(lhs: Query, rhs: Query) -> Self {
        Self { lhs, rhs }
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

impl BinaryOp for Xor {
    fn new(lhs: Query, rhs: Query) -> Self {
        Self { lhs, rhs }
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

impl BinaryOp for And {
    fn new(lhs: Query, rhs: Query) -> Self {
        Self { lhs, rhs }
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
    fn test_or_3() {
        let mut pairs = QueryParser::parse(Rule::input, ".a=1 or .b=2 or .c=3").unwrap();
        let p1 = pairs.next().unwrap();
        assert_eq!(p1.as_rule(), Rule::or);
        let mut pi1 = p1.into_inner();
        assert_eq!(pi1.len(), 3);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next(), None);
    }

    #[test]
    fn test_and_3() {
        let mut pairs = QueryParser::parse(Rule::input, ".a=1 and .b=2 and .c=3").unwrap();
        let p1 = pairs.next().unwrap();
        assert_eq!(p1.as_rule(), Rule::and);
        let mut pi1 = p1.into_inner();
        assert_eq!(pi1.len(), 3);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next(), None);
    }

    #[test]
    fn test_or_and() {
        let mut pairs = QueryParser::parse(Rule::input, ".a=1 or .b=2 and .c=3").unwrap();
        let p1 = pairs.next().unwrap();
        assert_eq!(p1.as_rule(), Rule::or);
        let mut pi1 = p1.into_inner();
        assert_eq!(pi1.len(), 2);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::and);
        assert_eq!(pi1.next(), None);
    }

    #[test]
    fn test_and_or() {
        let mut pairs = QueryParser::parse(Rule::input, ".a=1 and .b=2 or .c=3").unwrap();
        let p1 = pairs.next().unwrap();
        assert_eq!(p1.as_rule(), Rule::or);
        let mut pi1 = p1.into_inner();
        assert_eq!(pi1.len(), 2);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::and);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next(), None);
    }
}
