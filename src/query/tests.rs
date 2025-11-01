use super::*;
use crate::model::{
    Parser as RecordParser, RawRecord,
    v2::{compat::ParserSettings, parse::NewParser},
};

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
fn test_query_or() {
    let queries = [
        Query::parse(".a=1").unwrap().or(Query::parse(".b=2").unwrap()),
        Query::parse(".a=1 or .b=2").unwrap(),
    ];

    for query in &queries {
        let record = parse(r#"{"a":1}"#);
        assert!(record.matches(query));
        let record = parse(r#"{"b":2}"#);
        assert!(record.matches(query));
        let record = parse(r#"{"c":3}"#);
        assert!(!record.matches(query));
    }
}

#[test]
fn test_query_and() {
    let queries = [
        Query::parse(".a=1").unwrap().and(Query::parse(".b=2").unwrap()),
        Query::parse(".a=1 and .b=2").unwrap(),
    ];

    for query in &queries {
        let record = parse(r#"{"a":1,"b":2}"#);
        assert!(record.matches(query));
        let record = parse(r#"{"a":1,"b":3}"#);
        assert!(!record.matches(query));
        let record = parse(r#"{"a":2,"b":2}"#);
        assert!(!record.matches(query));
    }
}

#[test]
fn test_query_not() {
    let queries = [!Query::parse(".a=1").unwrap(), Query::parse("not .a=1").unwrap()];

    for query in &queries {
        let record = parse(r#"{"a":1}"#);
        assert!(!record.matches(query));
        let record = parse(r#"{"a":2}"#);
        assert!(record.matches(query));
    }
}

#[test]
fn test_query_bitwise_operators() {
    let q1 = Query::parse(".a=1").unwrap();
    let q2 = Query::parse(".b=2").unwrap();

    // Test BitAnd (&)
    let and_query1 = q1.clone() & q2.clone();
    let and_query2 = q1.clone().and(q2.clone());

    let record = parse(r#"{"a":1,"b":2}"#);
    assert!(record.matches(&and_query1));
    assert!(record.matches(&and_query2));

    let record = parse(r#"{"a":1,"b":3}"#);
    assert!(!record.matches(&and_query1));
    assert!(!record.matches(&and_query2));

    // Test BitOr (|)
    let or_query1 = q1.clone() | q2.clone();
    let or_query2 = q1.clone().or(q2.clone());

    let record = parse(r#"{"a":1,"b":3}"#);
    assert!(record.matches(&or_query1));
    assert!(record.matches(&or_query2));

    let record = parse(r#"{"a":0,"b":0}"#);
    assert!(!record.matches(&or_query1));
    assert!(!record.matches(&or_query2));

    // Test Not (!)
    let not_query = !q1.clone();
    let record = parse(r#"{"a":1}"#);
    assert!(!record.matches(&not_query));
    let record = parse(r#"{"a":2}"#);
    assert!(record.matches(&not_query));
}

#[test]
fn test_query_level() {
    let query = Query::parse("level=info").unwrap();
    let record = parse(r#"{"level":"info"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"level":"error"}"#);
    assert!(!record.matches(&query));
}

#[test]
fn test_query_json_str_simple() {
    for q in &["mod=test", r#"mod="test""#] {
        let query = Query::parse(q).unwrap();
        let record = parse(r#"{"mod":"test"}"#);
        assert!(record.matches(&query));
        let record = parse(r#"{"mod":"test2"}"#);
        assert!(!record.matches(&query));
        let record = parse(r#"{"mod":"\"test\""}"#);
        assert!(!record.matches(&query));
    }
}

#[test]
fn test_query_json_str_empty() {
    let query = Query::parse(r#"mod="""#).unwrap();
    let record = parse(r#"{"mod":""}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"mod":"t"}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"v":""}"#);
    assert!(!record.matches(&query));
}

#[test]
fn test_query_json_str_quoted() {
    let query = Query::parse(r#"mod="\"test\"""#).unwrap();
    let record = parse(r#"{"mod":"test"}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"mod":"test2"}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"mod":"\"test\""}"#);
    assert!(record.matches(&query));
}

#[test]
fn test_query_json_int() {
    let query = Query::parse("some-value=1447015572184281088").unwrap();
    let record = parse(r#"{"some-value":1447015572184281088}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"some-value":1447015572184281089}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"some-value":"1447015572184281088"}"#);
    assert!(record.matches(&query));
}

#[test]
fn test_query_json_int_escaped() {
    let query = Query::parse("v=42").unwrap();
    let record = parse(r#"{"v":42}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"4\u0032"}"#);
    assert!(record.matches(&query));
}

#[test]
fn test_query_json_float() {
    let query = Query::parse("v > 0.5").unwrap();
    let record = parse(r#"{"v":0.4}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"v":0.5}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"v":2}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"x":42}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"v":"0.4"}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"v":"0.6"}"#);
    assert!(record.matches(&query));
}

#[test]
fn test_query_json_in_str() {
    let query = Query::parse("v in (a,b,c)").unwrap();
    let record = parse(r#"{"v":"a"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"b"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"c"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"d"}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"x":"a"}"#);
    assert!(!record.matches(&query));
}

#[test]
fn test_query_json_in_int() {
    let query = Query::parse("v in (1,2)").unwrap();
    let record = parse(r#"{"v":1}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"1"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":2}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":3}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"v":"3"}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"x":1}"#);
    assert!(!record.matches(&query));
}

#[test]
fn query_in_set_file_valid() {
    let query = Query::parse("v in @src/testing/assets/query/set-valid").unwrap();
    let record = parse(r#"{"v":"line"}"#);
    assert!(!record.matches(&query));
    let record = parse(r#"{"v":"line1"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"line2"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"line3"}"#);
    assert!(record.matches(&query));
    let record = parse(r#"{"v":"line4"}"#);
    assert!(!record.matches(&query));
}

#[test]
fn query_in_set_file_invalid() {
    let filename = "src/testing/assets/query/set-invalid";
    let result = Query::parse(format!("v in @{}", filename));
    assert!(result.is_err());
    let err = result.err().unwrap();
    if let Error::FailedToLoadFile { path, source } = &err {
        assert_eq!(path, filename);
        if let Error::FailedToParseJsonLine { line, source } = &**source {
            assert_eq!(line, &2);
            assert!(source.is_eof());
        } else {
            panic!("unexpected error: {:?}", err);
        }
    } else {
        panic!("unexpected error: {:?}", err);
    }
}

#[test]
fn query_in_set_file_not_found() {
    let filename = "src/testing/assets/query/set-not-found";
    let result = Query::parse(format!("v in @{}", filename));
    assert!(result.is_err());
    let err = result.err().unwrap();
    if let Error::FailedToReadFile { path, source } = &err {
        assert_eq!(path, filename);
        assert!(source.kind() == std::io::ErrorKind::NotFound);
    } else {
        panic!("unexpected error: {:?}", err);
    }
}

fn parse(s: &str) -> Record<'_> {
    let settings = ParserSettings::default();
    let mut parser = settings
        .new_parser(crate::format::Auto::default(), s.as_bytes())
        .unwrap();
    parser.next().unwrap().unwrap()
}
