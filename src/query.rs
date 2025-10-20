// std imports
use std::{
    io::{BufRead, BufReader, Read},
    ops::{BitAnd, BitOr, Not},
    sync::{Arc, LazyLock},
};

// third-party imports
use closure::closure;
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;
use serde_json as json;
use wildflower::Pattern;

// local imports
use crate::error::{Error, Result};
use crate::level::RelaxedLevel;
use crate::model::{
    FieldFilter, FieldFilterKey, Level, Number, NumericOp, Record, RecordFilter, RecordFilterNone, UnaryBoolOp,
    ValueMatchPolicy,
};
use crate::types::FieldKind;

// ---

#[derive(Parser)]
#[grammar = "query.pest"]
pub struct QueryParser;

#[derive(Clone)]
pub struct Query {
    filter: Arc<dyn RecordFilter + Sync + Send>,
}

impl Query {
    pub fn parse(str: impl AsRef<str>) -> Result<Self> {
        let mut pairs = QueryParser::parse(Rule::input, str.as_ref())?;
        expression(pairs.next().unwrap())
    }

    pub fn and(self, rhs: Query) -> Query {
        Query::new(OpAnd { lhs: self, rhs })
    }

    pub fn or(self, rhs: Query) -> Query {
        Query::new(OpOr { lhs: self, rhs })
    }

    pub fn new<F: RecordFilter + Sync + Send + 'static>(filter: F) -> Self {
        Self {
            filter: Arc::new(filter),
        }
    }
}

impl Not for Query {
    type Output = Query;

    fn not(self) -> Self::Output {
        Query::new(OpNot { arg: self })
    }
}

impl BitAnd for Query {
    type Output = Query;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl BitOr for Query {
    type Output = Query;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

impl RecordFilter for Query {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.filter.apply(record)
    }
}

impl Default for Query {
    #[inline]
    fn default() -> Self {
        <&Query>::default().clone()
    }
}

impl Default for &'static Query {
    #[inline]
    fn default() -> Self {
        static QUERY_NONE: LazyLock<Query> = LazyLock::new(|| Query::new(RecordFilterNone {}));

        &QUERY_NONE
    }
}

// ---

fn expression(pair: Pair<Rule>) -> Result<Query> {
    match pair.as_rule() {
        Rule::expr_or => binary_op::<OpOr>(pair),
        Rule::expr_and => binary_op::<OpAnd>(pair),
        Rule::expr_not => not(pair),
        Rule::primary => primary(pair),
        _ => unreachable!(),
    }
}

fn binary_op<Op: BinaryOp + Sync + Send + 'static>(pair: Pair<Rule>) -> Result<Query> {
    let mut inner = pair.into_inner();
    let mut result = expression(inner.next().unwrap())?;
    for inner in inner {
        result = Query::new(Op::new(result, expression(inner)?));
    }
    Ok(result)
}

fn not(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::expr_not);

    Ok(Query::new(OpNot {
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
        Rule::level_filter => level_filter(inner),
        _ => unreachable!(),
    }
}

fn field_filter(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::field_filter);

    let mut inner = pair.into_inner();
    let lhs = inner.next().unwrap();
    let op = inner.next().unwrap().as_rule();
    let rhs = inner.next().unwrap();

    let (match_policy, negated) = match (op, rhs.as_rule()) {
        (Rule::op_in | Rule::op_not_in, Rule::string_set) => (
            ValueMatchPolicy::In(parse_string_set(rhs)?.into_iter().collect()),
            op == Rule::op_not_in,
        ),
        (Rule::op_equal | Rule::op_not_equal, Rule::string) => {
            (ValueMatchPolicy::Exact(parse_string(rhs)?), op == Rule::op_not_equal)
        }
        (Rule::op_like | Rule::op_not_like, Rule::string) => (
            ValueMatchPolicy::WildCard(Pattern::new(parse_string(rhs)?.to_string())),
            op == Rule::op_not_like,
        ),
        (Rule::op_contain | Rule::op_not_contain, Rule::string) => (
            ValueMatchPolicy::SubString(parse_string(rhs)?),
            op == Rule::op_not_contain,
        ),
        (Rule::op_regex_match | Rule::op_not_regex_match, Rule::string) => (
            ValueMatchPolicy::RegularExpression(parse_string(rhs)?.parse()?),
            op == Rule::op_not_regex_match,
        ),
        (Rule::op_in | Rule::op_not_in, Rule::number_set) => (
            ValueMatchPolicy::Numerically(NumericOp::In(parse_number_set(rhs)?)),
            op == Rule::op_not_in,
        ),
        (Rule::op_equal, Rule::number) => (ValueMatchPolicy::Numerically(NumericOp::Eq(parse_number(rhs)?)), false),
        (Rule::op_not_equal, Rule::number) => (ValueMatchPolicy::Numerically(NumericOp::Ne(parse_number(rhs)?)), false),
        (Rule::op_ge, Rule::number) => (ValueMatchPolicy::Numerically(NumericOp::Ge(parse_number(rhs)?)), false),
        (Rule::op_gt, Rule::number) => (ValueMatchPolicy::Numerically(NumericOp::Gt(parse_number(rhs)?)), false),
        (Rule::op_le, Rule::number) => (ValueMatchPolicy::Numerically(NumericOp::Le(parse_number(rhs)?)), false),
        (Rule::op_lt, Rule::number) => (ValueMatchPolicy::Numerically(NumericOp::Lt(parse_number(rhs)?)), false),
        _ => unreachable!(),
    };

    Ok(Query::new(FieldFilter::new(
        parse_field_name(lhs)?.borrowed(),
        match_policy,
        if negated {
            UnaryBoolOp::Negate
        } else {
            UnaryBoolOp::None
        },
    )))
}

fn level_filter(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::level_filter);

    let mut inner = pair.into_inner();

    let op = inner.next().unwrap().as_rule();
    let level = parse_level(inner.next().unwrap())?;
    Ok(match op {
        Rule::op_equal => LevelFilter::query(closure!(clone level, | l | l == level)),
        Rule::op_not_equal => LevelFilter::query(closure!(clone level, | l | l != level)),
        Rule::op_lt => LevelFilter::query(closure!(clone level, | l | l > level)),
        Rule::op_le => LevelFilter::query(closure!(clone level, | l | l >= level)),
        Rule::op_gt => LevelFilter::query(closure!(clone level, | l | l < level)),
        Rule::op_ge => LevelFilter::query(closure!(clone level, | l | l <= level)),
        _ => unreachable!(),
    })
}

fn parse_string(pair: Pair<Rule>) -> Result<String> {
    assert_eq!(pair.as_rule(), Rule::string);

    let inner = pair.into_inner().next().unwrap();
    Ok(match inner.as_rule() {
        Rule::json_string => json::from_str(inner.as_str())?,
        Rule::simple_string => inner.as_str().into(),
        _ => unreachable!(),
    })
}

fn parse_string_set(pair: Pair<Rule>) -> Result<Vec<String>> {
    assert_eq!(pair.as_rule(), Rule::string_set);

    let inner = pair.into_inner().next().unwrap();
    Ok(match inner.as_rule() {
        Rule::string_set_literal => parse_string_set_literal(inner)?,
        Rule::string_set_file => parse_string_set_file(inner)?,
        _ => unreachable!(),
    })
}

fn parse_string_set_literal(pair: Pair<Rule>) -> Result<Vec<String>> {
    assert_eq!(pair.as_rule(), Rule::string_set_literal);

    let inner = pair.into_inner();
    inner.map(|p| parse_string(p)).collect::<Result<Vec<_>>>()
}

fn parse_string_set_file(pair: Pair<Rule>) -> Result<Vec<String>> {
    assert_eq!(pair.as_rule(), Rule::string_set_file);

    let inner = pair.into_inner().next().unwrap();
    let filename = parse_string(inner)?;
    let stream: Box<dyn Read> = if filename == "-" {
        Box::new(std::io::stdin())
    } else {
        Box::new(std::fs::File::open(&filename).map_err(|e| Error::FailedToReadFile {
            path: filename.clone(),
            source: e,
        })?)
    };
    BufReader::new(stream)
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let line = line?;
            if line.starts_with('"') {
                json::from_str(line.as_str()).map_err(|e| Error::FailedToParseJsonLine { line: i + 1, source: e })
            } else {
                Ok(line.to_owned())
            }
        })
        .collect::<Result<Vec<_>>>()
        .map_err(|e| Error::FailedToLoadFile {
            path: filename.clone(),
            source: Box::new(e),
        })
}

fn parse_number(pair: Pair<Rule>) -> Result<Number> {
    assert_eq!(pair.as_rule(), Rule::number);

    let inner = pair.as_str();
    inner.parse()
}

fn parse_number_set(pair: Pair<Rule>) -> Result<Vec<Number>> {
    assert_eq!(pair.as_rule(), Rule::number_set);

    let inner = pair.into_inner();
    inner.map(|p| parse_number(p)).collect::<Result<Vec<_>>>()
}

fn parse_level(pair: Pair<Rule>) -> Result<Level> {
    assert_eq!(pair.as_rule(), Rule::level);

    let mut inner = pair.into_inner();
    let level = parse_string(inner.next().unwrap())?;
    Ok(RelaxedLevel::try_from(level.as_str())?.into())
}

fn parse_field_name(pair: Pair<Rule>) -> Result<FieldFilterKey<String>> {
    assert_eq!(pair.as_rule(), Rule::field_name);

    let inner = pair.into_inner().next().unwrap();
    Ok(match inner.as_rule() {
        Rule::json_string => FieldFilterKey::Custom(json::from_str(inner.as_str())?),
        _ => match inner.as_str() {
            "message" => FieldFilterKey::Predefined(FieldKind::Message),
            "logger" => FieldFilterKey::Predefined(FieldKind::Logger),
            "caller" => FieldFilterKey::Predefined(FieldKind::Caller),
            _ => FieldFilterKey::Custom(inner.as_str().trim_start_matches('.').to_owned()),
        },
    })
}

// ---

trait BinaryOp: RecordFilter {
    fn new(lhs: Query, rhs: Query) -> Self;
}

// ---

struct OpOr {
    lhs: Query,
    rhs: Query,
}

impl RecordFilter for OpOr {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.lhs.apply(record) || self.rhs.apply(record)
    }
}

impl BinaryOp for OpOr {
    fn new(lhs: Query, rhs: Query) -> Self {
        Self { lhs, rhs }
    }
}

// ---

struct OpAnd {
    lhs: Query,
    rhs: Query,
}

impl RecordFilter for OpAnd {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.lhs.apply(record) && self.rhs.apply(record)
    }
}

impl BinaryOp for OpAnd {
    fn new(lhs: Query, rhs: Query) -> Self {
        Self { lhs, rhs }
    }
}

// ---

struct OpNot {
    arg: Query,
}

impl RecordFilter for OpNot {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        !self.arg.apply(record)
    }
}

// ---

struct LevelFilter<F> {
    f: F,
}

impl<F: Fn(Level) -> bool + Send + Sync + 'static> LevelFilter<F> {
    fn new(f: F) -> Self {
        Self { f }
    }

    fn query(f: F) -> Query {
        Query::new(Self::new(f))
    }
}

impl<F: Fn(Level) -> bool> RecordFilter for LevelFilter<F> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        record.level.map(&self.f).unwrap_or(false)
    }
}

// ---

#[cfg(test)]
mod tests;
