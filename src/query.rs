// third-party imports
use closure::closure;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde_json as json;
use wildflower::Pattern;

// local imports
use crate::error::Result;
use crate::level::RelaxedLevel;
use crate::model::{
    FieldFilter, FieldFilterKey, Level, Number, NumericOp, Record, RecordFilter, UnaryBoolOp, ValueMatchPolicy,
};
use crate::types::FieldKind;

// ---

#[derive(Parser)]
#[grammar = "query.pest"]
pub struct QueryParser;

pub struct Query {
    filter: Box<dyn RecordFilter + Sync>,
}

impl Query {
    pub fn parse(str: &str) -> Result<Self> {
        let mut pairs = QueryParser::parse(Rule::input, str)?;
        Ok(expression(pairs.next().unwrap())?)
    }

    pub fn and(self, rhs: Query) -> Query {
        Query::new(And { lhs: self, rhs })
    }

    pub fn or(self, rhs: Query) -> Query {
        Query::new(Or { lhs: self, rhs })
    }

    pub fn not(self) -> Query {
        Query::new(Not { arg: self })
    }

    fn new<F: RecordFilter + Sync + 'static>(filter: F) -> Self {
        Self {
            filter: Box::new(filter),
        }
    }
}

impl RecordFilter for Query {
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        self.filter.apply(record)
    }
}

// ---

fn expression(pair: Pair<Rule>) -> Result<Query> {
    match pair.as_rule() {
        Rule::expr_or => binary_op::<Or>(pair),
        Rule::expr_and => binary_op::<And>(pair),
        Rule::expr_not => not(pair),
        Rule::primary => primary(pair),
        _ => unreachable!(),
    }
}

fn binary_op<Op: BinaryOp + Sync + 'static>(pair: Pair<Rule>) -> Result<Query> {
    let mut inner = pair.into_inner();
    let mut result = expression(inner.next().unwrap())?;
    for inner in inner {
        result = Query::new(Op::new(result, expression(inner)?));
    }
    Ok(result)
}

fn not(pair: Pair<Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::expr_not);

    Ok(Query::new(Not {
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
        (Rule::op_in | Rule::op_not_in, Rule::string_set) => {
            (ValueMatchPolicy::In(parse_string_set(rhs)?), op == Rule::op_not_in)
        }
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

    let inner = pair.into_inner();
    inner.map(|p| parse_string(p)).collect::<Result<Vec<_>>>()
}

fn parse_number(pair: Pair<Rule>) -> Result<Number> {
    assert_eq!(pair.as_rule(), Rule::number);

    let inner = pair.as_str();
    Ok(inner.parse()?)
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
    fn apply<'a>(&self, record: &'a Record<'a>) -> bool {
        record.level.map(&self.f).unwrap_or(false)
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Parser as RecordParser, ParserSettings, RawRecord};

    #[test]
    fn test_or_3() {
        let mut pairs = QueryParser::parse(Rule::input, ".a=1 or .b=2 or .c=3").unwrap();
        let p1 = pairs.next().unwrap();
        assert_eq!(p1.as_rule(), Rule::expr_or);
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
        assert_eq!(p1.as_rule(), Rule::expr_and);
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
        assert_eq!(p1.as_rule(), Rule::expr_or);
        let mut pi1 = p1.into_inner();
        assert_eq!(pi1.len(), 2);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::expr_and);
        assert_eq!(pi1.next(), None);
    }

    #[test]
    fn test_and_or() {
        let mut pairs = QueryParser::parse(Rule::input, ".a=1 and .b=2 or .c=3").unwrap();
        let p1 = pairs.next().unwrap();
        assert_eq!(p1.as_rule(), Rule::expr_or);
        let mut pi1 = p1.into_inner();
        assert_eq!(pi1.len(), 2);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::expr_and);
        assert_eq!(pi1.next().unwrap().as_rule(), Rule::primary);
        assert_eq!(pi1.next(), None);
    }

    #[test]
    fn test_query_json_str_simple() {
        for q in &["mod=test", r#"mod="test""#] {
            let query = Query::parse(q).unwrap();
            let record = parse(r#"{"mod":"test"}"#);
            assert_eq!(record.matches(&query), true);
            let record = parse(r#"{"mod":"test2"}"#);
            assert_eq!(record.matches(&query), false);
            let record = parse(r#"{"mod":"\"test\""}"#);
            assert_eq!(record.matches(&query), false);
        }
    }

    #[test]
    fn test_query_json_str_empty() {
        let query = Query::parse(r#"mod="""#).unwrap();
        let record = parse(r#"{"mod":""}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"mod":"t"}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"v":""}"#);
        assert_eq!(record.matches(&query), false);
    }

    #[test]
    fn test_query_json_str_quoted() {
        let query = Query::parse(r#"mod="\"test\"""#).unwrap();
        let record = parse(r#"{"mod":"test"}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"mod":"test2"}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"mod":"\"test\""}"#);
        assert_eq!(record.matches(&query), true);
    }

    #[test]
    fn test_query_json_int() {
        let query = Query::parse("some-value=1447015572184281088").unwrap();
        let record = parse(r#"{"some-value":1447015572184281088}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"some-value":1447015572184281089}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"some-value":"1447015572184281088"}"#);
        assert_eq!(record.matches(&query), true);
    }

    #[test]
    fn test_query_json_int_escaped() {
        let query = Query::parse("v=42").unwrap();
        let record = parse(r#"{"v":42}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"v":"4\u0032"}"#);
        assert_eq!(record.matches(&query), true);
    }

    #[test]
    fn test_query_json_float() {
        let query = Query::parse("v > 0.5").unwrap();
        let record = parse(r#"{"v":0.4}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"v":0.5}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"v":2}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"x":42}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"v":"0.4"}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"v":"0.6"}"#);
        assert_eq!(record.matches(&query), true);
    }

    #[test]
    fn test_query_json_in_str() {
        let query = Query::parse("v in (a,b,c)").unwrap();
        let record = parse(r#"{"v":"a"}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"v":"b"}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"v":"c"}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"v":"d"}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"x":"a"}"#);
        assert_eq!(record.matches(&query), false);
    }

    #[test]
    fn test_query_json_in_int() {
        let query = Query::parse("v in (1,2)").unwrap();
        let record = parse(r#"{"v":1}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"v":"1"}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"v":2}"#);
        assert_eq!(record.matches(&query), true);
        let record = parse(r#"{"v":3}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"v":"3"}"#);
        assert_eq!(record.matches(&query), false);
        let record = parse(r#"{"x":1}"#);
        assert_eq!(record.matches(&query), false);
    }

    fn parse(s: &str) -> Record {
        let raw = RawRecord::parser().parse(s.as_bytes()).next().unwrap().unwrap().record;
        let parser = RecordParser::new(ParserSettings::default());
        parser.parse(raw)
    }
}
