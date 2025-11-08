use super::*;
use crate::model::{Parser as RecordParser, ParserSettings, RawRecord};
use rstest::rstest;

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

#[rstest]
#[case("msg=test", "msg=test", true)] // 1
#[case("msg=test", "msg=other", false)] // 2
#[case("msg=test", "message=test", true)] // 3
#[case("msg=test", "message=other", false)] // 4
#[case("message=test", "msg=test", true)] // 5
#[case("message=test", "msg=other", false)] // 6
#[case("message=test", "message=test", true)] // 7
#[case("message=test", "message=other", false)] // 8
fn test_query_field_name_message_alias(#[case] raw_query: &str, #[case] input: &str, #[case] should_match: bool) {
    // Test that both "msg" and "message" work as field name aliases for querying the message field
    let query = Query::parse(raw_query).unwrap();

    let record = parse(input);
    assert_eq!(
        record.matches(&query),
        should_match,
        "Query {:?} should {} input {:?}",
        raw_query,
        if should_match { "match" } else { "not match" },
        input,
    );
}

#[rstest]
// Test include absent modifier (?) with exact match: field present
#[case(".price?=3", r#"{"price":3}"#, true)] // 1
// Test include absent modifier (?) with exact match: field absent
#[case(".price?=3", r#"{"x":1}"#, true)] // 2
// Test include absent modifier (?) with negation: field present but doesn't match
#[case(".price?!=3", r#"{"price":4}"#, true)] // 3
// Test include absent modifier (?) with negation: field absent
#[case(".price?!=3", r#"{"x":1}"#, true)] // 4
// Test include absent modifier (?) with negation: field present and matches (should fail)
#[case(".price?!=3", r#"{"price":3}"#, false)] // 5
// Test without include absent modifier: field absent (should fail)
#[case(".price=3", r#"{"x":1}"#, false)] // 6
// Test without include absent modifier: field present and matches
#[case(".price=3", r#"{"price":3}"#, true)] // 7
// Test with negation without include absent modifier: field absent (should fail)
#[case(".price!=3", r#"{"x":1}"#, false)] // 8
// Test with negation without include absent modifier: field present and doesn't match
#[case(".price!=3", r#"{"price":4}"#, true)] // 9
fn test_query_include_absent_modifier_single_field(
    #[case] raw_query: &str,
    #[case] input: &str,
    #[case] should_match: bool,
) {
    let query = Query::parse(raw_query).unwrap();
    let record = parse(input);
    assert_eq!(
        record.matches(&query),
        should_match,
        "Query {:?} should {} input {:?}",
        raw_query,
        if should_match { "match" } else { "not match" },
        input,
    );
}

#[rstest]
// Test include absent modifier with repeated fields (same key, multiple values)
// Query: .price?=3 | Field present with matching value
#[case(".price?=3", r#"price=3"#, true)] // 1
// Query: .price?=3 | Field repeated, one matches
#[case(".price?=3", r#"price=3 price=3"#, true)] // 2
// Query: .price?=3 | Field repeated, one matches among others
#[case(".price?=3", r#"price=3 price=4"#, true)] // 3
// Query: .price?=3 | Field repeated, none match
#[case(".price?=3", r#"price=2 price=4"#, false)] // 4
// Query: .price?=3 | Field absent
#[case(".price?=3", r#"x=a"#, true)] // 5
// Query: .price?!=3 | Field present with matching value (negation fails)
#[case(".price?!=3", r#"price=3"#, false)] // 6
// Query: .price?!=3 | Field repeated, all same and match (negation fails)
#[case(".price?!=3", r#"price=3 price=3"#, false)] // 7
// Query: .price?!=3 | Field repeated, some don't match (negation succeeds)
#[case(".price?!=3", r#"price=3 price=4"#, true)] // 8
// Query: .price?!=3 | Field repeated, none match value (negation succeeds)
#[case(".price?!=3", r#"price=2 price=4"#, true)] // 9
// Query: .price?!=3 | Field absent (negation succeeds with include absent)
#[case(".price?!=3", r#"x=a"#, true)] // 10
fn test_query_include_absent_modifier_repeated_fields(
    #[case] raw_query: &str,
    #[case] input: &str,
    #[case] should_match: bool,
) {
    let query = Query::parse(raw_query).unwrap();
    let record = parse(input);
    assert_eq!(
        record.matches(&query),
        should_match,
        "Query {:?} should {} input {:?}",
        raw_query,
        if should_match { "match" } else { "not match" },
        input,
    );
}

#[rstest]
// Test with substring match and include absent modifier
#[case(".msg?~=hello", r#"msg=helloworld"#, true)] // 1
#[case(".msg?~=hello", r#"msg=goodbye"#, false)] // 2
#[case(".msg?~=hello", r#"x=other"#, true)] // 3
// Test with regex match and include absent modifier
#[case(".code?~~=\"^[0-9]{3}$\"", r#"code=404"#, true)] // 4
#[case(".code?~~=\"^[0-9]{3}$\"", r#"code=40"#, false)] // 5
#[case(".code?~~=\"^[0-9]{3}$\"", r#"x=other"#, true)] // 6
// Test negation with substring
#[case(".msg?!~=error", r#"msg=success"#, true)] // 7
#[case(".msg?!~=error", r#"msg=erroroccurred"#, false)] // 8
#[case(".msg?!~=error", r#"x=other"#, true)] // 9
// Test in operator with include absent
#[case(".status?in(ok,good)", r#"status=ok"#, true)] // 10
#[case(".status?in(ok,good)", r#"status=bad"#, false)] // 11
#[case(".status?in(ok,good)", r#"x=other"#, true)] // 12
fn test_query_include_absent_with_operators(#[case] raw_query: &str, #[case] input: &str, #[case] should_match: bool) {
    let query = Query::parse(raw_query).unwrap();
    let record = parse(input);
    assert_eq!(
        record.matches(&query),
        should_match,
        "Query {:?} should {} input {:?}",
        raw_query,
        if should_match { "match" } else { "not match" },
        input,
    );
}

#[rstest]
// Test repeated fields (logfmt format where same key appears multiple times)
// Query: .tag=v1 | Field repeated with matching value
#[case(".tag=v1", r#"tag=v1"#, true)] // 1
// Query: .tag=v1 | Field repeated multiple times, one matches
#[case(".tag=v1", r#"tag=v1 tag=v2"#, true)] // 2
// Query: .tag=v1 | Field repeated multiple times, none match
#[case(".tag=v1", r#"tag=v2 tag=v3"#, false)] // 3
// Query: .tag=v1 | Field absent
#[case(".tag=v1", r#"x=y"#, false)] // 4
// Query: .tag?=v1 | Field repeated, one matches (with include absent)
#[case(".tag?=v1", r#"tag=v1 tag=v2"#, true)] // 5
// Query: .tag?=v1 | Field absent (with include absent)
#[case(".tag?=v1", r#"x=y"#, true)] // 6
// Query: .tag!=v1 | Field repeated, none match
#[case(".tag!=v1", r#"tag=v2 tag=v3"#, true)] // 7
// Query: .tag!=v1 | Field repeated, one matches (negation succeeds on non-matching value)
#[case(".tag!=v1", r#"tag=v1 tag=v2"#, true)] // 8
// Query: .tag?!=v1 | Field repeated with one matching (negation with include absent)
#[case(".tag?!=v1", r#"tag=v1"#, false)] // 9
// Query: .tag?!=v1 | Field absent (negation with include absent succeeds)
#[case(".tag?!=v1", r#"x=y"#, true)] // 10
fn test_query_repeated_fields(#[case] raw_query: &str, #[case] input: &str, #[case] should_match: bool) {
    let query = Query::parse(raw_query).unwrap();
    let record = parse(input);
    assert_eq!(
        record.matches(&query),
        should_match,
        "Query {:?} should {} input {:?}",
        raw_query,
        if should_match { "match" } else { "not match" },
        input,
    );
}

#[rstest]
// Test exists operator with single field
#[case("exists(.price)", r#"{"price":3}"#, true)] // 1
#[case("exists(.price)", r#"{"x":1}"#, false)] // 2
// Test exists operator with single field (alternate form: exist)
#[case("exist(.price)", r#"{"price":3}"#, true)] // 3
#[case("exist(.price)", r#"{"x":1}"#, false)] // 4
// Test exists operator with missing field
#[case("exists(.missing)", r#"{"a":1,"b":2}"#, false)] // 5
// Test exists operator combined with other conditions (and)
#[case("exists(.price) and .price=3", r#"{"price":3}"#, true)] // 6
#[case("exists(.price) and .price=5", r#"{"price":3}"#, false)] // 7
#[case("exists(.price) and .price=3", r#"{"x":1}"#, false)] // 8
// Test exists operator combined with other conditions (or)
#[case("exists(.price) or .name=test", r#"{"name":"test"}"#, true)] // 9
#[case("exists(.price) or .name=other", r#"{"x":1}"#, false)] // 10
// Test exists operator with negation
#[case("not exists(.price)", r#"{"price":3}"#, false)] // 11
#[case("not exists(.price)", r#"{"x":1}"#, true)] // 12
// Test exists operator with predefined fields
#[case("exists(msg)", r#"msg=hello"#, true)] // 13
#[case("exists(msg)", r#"x=hello"#, false)] // 14
fn test_query_exists_operator(#[case] raw_query: &str, #[case] input: &str, #[case] should_match: bool) {
    let query = Query::parse(raw_query).unwrap();
    let record = parse(input);
    assert_eq!(
        record.matches(&query),
        should_match,
        "Query {:?} should {} input {:?}",
        raw_query,
        if should_match { "match" } else { "not match" },
        input,
    );
}

#[rstest]
#[case("value > 3.14", r#"{"value":"a"}"#, false)]
fn test_query_numerical_type_mismatch(#[case] raw_query: &str, #[case] input: &str, #[case] should_match: bool) {
    let query = Query::parse(raw_query).unwrap();
    let record = parse(input);
    assert_eq!(
        record.matches(&query),
        should_match,
        "Query {:?} should {} input {:?}",
        raw_query,
        if should_match { "match" } else { "not match" },
        input,
    );
}

fn parse(s: &str) -> Record<'_> {
    let raw = RawRecord::parser().parse(s.as_bytes()).next().unwrap().unwrap().record;
    let parser = RecordParser::new(ParserSettings::default());
    parser.parse(&raw)
}
