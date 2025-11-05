use super::*;
use rstest::rstest;

use chrono::TimeZone;
use maplit::hashmap;
use serde_logfmt::logfmt;

use crate::settings::{Field, FieldShowOption};

#[test]
fn test_raw_record_parser_empty_line() {
    let parser = RawRecordParser::new();
    let stream = parser.parse(b"");
    let mut stream = match stream {
        s @ RawRecordStream::Empty => s,
        _ => panic!(),
    };

    assert!(stream.next().is_none());
}

#[test]
fn test_raw_record_parser_empty_object() {
    let parser = RawRecordParser::new();
    let stream = parser.parse(b"{}");
    let mut stream = match stream {
        RawRecordStream::Json(s) => s,
        _ => panic!(),
    };

    let rec = stream.next().unwrap().unwrap();
    assert_eq!(rec.prefix, b"");
    assert_eq!(rec.record.fields.as_slices().0.len(), 0);
    assert_eq!(rec.record.fields.as_slices().1.len(), 0);
}

#[test]
fn test_raw_record_parser_invalid_type() {
    let parser = RawRecordParser::new().format(Some(InputFormat::Json));
    let mut stream = parser.parse(b"12");
    assert!(matches!(stream.next(), Some(Err(Error::JsonParseError(_)))));
}

#[test]
fn test_raw_record_parser_default() {
    let parser1 = RawRecordParser::new();
    let parser2 = RawRecordParser::default();

    // Both should behave identically
    let stream1 = parser1.parse(b"{}");
    let stream2 = parser2.parse(b"{}");

    // Verify they both parse empty JSON correctly
    // Use a helper function to check discriminant without requiring Debug on inner types
    fn is_json_stream<Json, Logfmt>(stream: &RawRecordStream<Json, Logfmt>) -> bool {
        matches!(stream, RawRecordStream::Json(_))
    }

    assert!(is_json_stream(&stream1));
    assert!(is_json_stream(&stream2));
}

#[test]
fn test_raw_value_auto() {
    let value = RawValue::auto("123");
    assert_eq!(value, RawValue::Number("123"));

    let value = RawValue::auto("-123");
    assert_eq!(value, RawValue::Number("-123"));

    let value = RawValue::auto("123.0");
    assert_eq!(value, RawValue::Number("123.0"));

    let value = RawValue::auto("true");
    assert_eq!(value, RawValue::Boolean(true));

    let value = RawValue::auto("false");
    assert_eq!(value, RawValue::Boolean(false));

    let value = RawValue::auto("null");
    assert_eq!(value, RawValue::Null);

    let value = RawValue::auto(r#""123""#);
    assert_eq!(value, RawValue::String(EncodedString::json(r#""123""#)));

    let value = RawValue::auto(r#"something"#);
    assert_eq!(value, RawValue::String(EncodedString::raw(r#"something"#)));
}

#[test]
fn test_raw_value_is_empty() {
    let value = RawValue::Number("0");
    assert!(!value.is_empty());

    let value = RawValue::Number("123");
    assert!(!value.is_empty());

    let value = RawValue::String(EncodedString::raw(""));
    assert!(value.is_empty());

    let value = RawValue::String(EncodedString::raw("aa"));
    assert!(!value.is_empty());

    let value = RawValue::String(EncodedString::json(r#""""#));
    assert!(value.is_empty());

    let value = RawValue::String(EncodedString::json(r#""aa""#));
    assert!(!value.is_empty());

    let value = RawValue::Boolean(true);
    assert!(!value.is_empty());

    let value = RawValue::Null;
    assert!(value.is_empty());

    let value = RawValue::Object(RawObject::Json(json::from_str("{}").unwrap()));
    assert!(value.is_empty());

    let value = RawValue::Object(RawObject::Json(json::from_str(r#"{"a":1}"#).unwrap()));
    assert!(!value.is_empty());

    let value = RawValue::Array(RawArray::Json(json::from_str("[]").unwrap()));
    assert!(value.is_empty());

    let value = RawValue::Array(RawArray::Json(json::from_str(r#"[1]"#).unwrap()));
    assert!(!value.is_empty());
}

#[test]
fn test_raw_value_raw_str() {
    let value = RawValue::Number("123");
    assert_eq!(value.raw_str(), "123");

    let value = RawValue::String(EncodedString::raw("123"));
    assert_eq!(value.raw_str(), "123");

    let value = RawValue::String(EncodedString::json(r#""123""#));
    assert_eq!(value.raw_str(), r#""123""#);

    let value = RawValue::Boolean(true);
    assert_eq!(value.raw_str(), "true");

    let value = RawValue::Null;
    assert_eq!(value.raw_str(), "null");

    let value = RawValue::Object(RawObject::Json(json::from_str("{}").unwrap()));
    assert_eq!(value.raw_str(), "{}");

    let value = RawValue::Array(RawArray::Json(json::from_str("[]").unwrap()));
    assert_eq!(value.raw_str(), "[]");
}

#[test]
fn test_raw_value_parse() {
    let value = RawValue::Number("123");
    assert_eq!(value.parse::<i64>().unwrap(), 123);
    assert_eq!(value.parse::<&str>().unwrap(), "123");

    let value = RawValue::String(EncodedString::raw("123"));
    assert_eq!(value.parse::<i64>().unwrap(), 123);
    assert_eq!(value.parse::<&str>().unwrap(), "123");

    let value = RawValue::String(EncodedString::json(r#""123""#));
    assert_eq!(value.parse::<&str>().unwrap(), "123");

    let value = RawValue::Boolean(true);
    assert!(value.parse::<bool>().unwrap());
    assert_eq!(value.parse::<&str>().unwrap(), "true");

    let value = RawValue::Boolean(false);
    assert!(!value.parse::<bool>().unwrap());
    assert_eq!(value.parse::<&str>().unwrap(), "false");

    let value = RawValue::Null;
    assert!(value.parse::<()>().is_ok());

    let value = RawValue::Object(RawObject::Json(json::from_str(r#"{"a":123}"#).unwrap()));
    assert_eq!(value.parse::<HashMap<_, _>>().unwrap(), hashmap! {"a" => 123});

    let value = RawValue::Array(RawArray::Json(json::from_str("[1,42]").unwrap()));
    assert_eq!(value.parse::<Vec<i64>>().unwrap(), vec![1, 42]);
}

#[test]
fn test_raw_value_object() {
    let v1 = RawObject::Json(json::from_str(r#"{"a":123}"#).unwrap());
    let v2 = RawObject::Json(json::from_str(r#"{"a":42}"#).unwrap());
    assert_eq!(RawValue::Object(v1), RawValue::Object(v1));
    assert_ne!(RawValue::Object(v1), RawValue::Object(v2));
    assert_ne!(RawValue::Object(v1), RawValue::Number("42"));
}

#[test]
fn test_raw_value_array() {
    let v1 = RawArray::Json(json::from_str(r#"[42]"#).unwrap());
    let v2 = RawArray::Json(json::from_str(r#"[43]"#).unwrap());
    assert_eq!(RawValue::Array(v1), RawValue::Array(v1));
    assert_ne!(RawValue::Array(v1), RawValue::Array(v2));
    assert_ne!(RawValue::Array(v1), RawValue::Number("42"));
}

#[test]
fn test_field_filter_json_str_simple() {
    let filter = FieldFilter::parse("mod=test").unwrap();
    let record = parse(r#"{"mod":"test"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"test2"}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":"\"test\""}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_json_str_empty() {
    let filter = FieldFilter::parse("mod=").unwrap();
    let record = parse(r#"{"mod":""}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"t"}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"v":""}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_json_str_quoted() {
    let filter = FieldFilter::parse(r#"mod="test""#).unwrap();
    let record = parse(r#"{"mod":"test"}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":"test2"}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":"\"test\""}"#);
    assert!(filter.apply(&record));
}

#[test]
fn test_field_filter_json_str_escaped() {
    let filter = FieldFilter::parse("mod=te st").unwrap();
    let record = parse(r#"{"mod":"te st"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"test"}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":"te\u0020st"}"#);
    assert!(filter.apply(&record));
}

#[test]
fn test_field_filter_json_int() {
    let filter = FieldFilter::parse("v=42").unwrap();
    let record = parse(r#"{"v":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"v":"42"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"v":423}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"v":"423"}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_json_int_escaped() {
    let filter = FieldFilter::parse("v=42").unwrap();
    let record = parse(r#"{"v":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"v":"4\u0032"}"#);
    assert!(filter.apply(&record));
}

#[test]
fn test_field_filter_logfmt_str_simple() {
    let filter = FieldFilter::parse("mod=test").unwrap();
    let record = parse("mod=test");
    assert!(filter.apply(&record));
    let record = parse("mod=test2");
    assert!(!filter.apply(&record));
    let record = parse(r#"mod="test""#);
    assert!(filter.apply(&record));
    let record = parse(r#"mod="\"test\"""#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_logfmt_str_empty() {
    let filter = FieldFilter::parse("mod=").unwrap();
    let record = parse(r#"mod="""#);
    assert!(filter.apply(&record));
    let record = parse("mod=t");
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_logfmt_str_quoted() {
    let filter = FieldFilter::parse(r#"mod="test""#).unwrap();
    let record = parse("mod=test");
    assert!(!filter.apply(&record));
    let record = parse(r#"mod=test2"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"mod="test""#);
    assert!(!filter.apply(&record));
    let record = parse(r#"mod="\"test\"""#);
    assert!(filter.apply(&record));
}

#[test]
fn test_field_filter_logfmt_str_escaped() {
    let filter = FieldFilter::parse("mod=te st").unwrap();
    let record = parse(r#"mod="te st""#);
    assert!(filter.apply(&record));
    let record = parse(r#"mod=test"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"mod="te\u0020st""#);
    assert!(filter.apply(&record));
}

#[test]
fn test_field_filter_logfmt_int() {
    let filter = FieldFilter::parse("v=42").unwrap();
    let record = parse(r#"v=42"#);
    assert!(filter.apply(&record));
    let record = parse(r#"v="42""#);
    assert!(filter.apply(&record));
    let record = parse(r#"v=423"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"v="423""#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_logfmt_int_escaped() {
    let filter = FieldFilter::parse("v=42").unwrap();
    let record = parse(r#"v=42"#);
    assert!(filter.apply(&record));
    let record = parse(r#"v="4\u0032""#);
    assert!(filter.apply(&record));
}

#[test]
fn test_parse_single_word() {
    let result = try_parse("test");
    assert!(result.is_err());
    assert!(matches!(
        result.err(),
        Some(Error::LogfmtParseError(logfmt::Error::ExpectedKeyValueDelimiter))
    ));
}

#[test]
fn test_record_filter_empty() {
    let filter = Filter::default();
    let record = parse(r#"{"v":42}"#);
    assert!(filter.apply(&record));
}

#[test]
fn test_record_filter_level() {
    let filter = Filter {
        level: Some(Level::Error),
        ..Default::default()
    };
    let record = parse(r#"{"level":"error"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"level":"info"}"#);
    assert!(!filter.apply(&record));

    let filter = Level::Error;
    let record = parse(r#"{"level":"error"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"level":"info"}"#);
    assert!(!filter.apply(&record));

    let filter = Some(Level::Info);
    let record = parse(r#"{"level":"info"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"level":"error"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"level":"debug"}"#);
    assert!(!filter.apply(&record));

    let filter: Option<Level> = None;
    let record = parse(r#"{"level":"info"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"level":"error"}"#);
    assert!(filter.apply(&record));
}

#[test]
fn test_record_filter_since() {
    let filter = Filter {
        since: Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()),
        ..Default::default()
    };
    let record = parse(r#"{"ts":"2021-01-01T00:00:00Z"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"ts":"2020-01-01T00:00:00Z"}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_record_filter_until() {
    let filter = Filter {
        until: Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()),
        ..Default::default()
    };
    let record = parse(r#"{"ts":"2021-01-01T00:00:00Z"}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"ts":"2022-01-01T00:00:00Z"}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_record_filter_fields() {
    let filter = Filter {
        fields: FieldFilterSet::new(["mod=test", "v=42"]).unwrap(),
        ..Default::default()
    };
    let record = parse(r#"{"mod":"test","v":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"test","v":43}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":"test2","v":42}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_record_filter_all() {
    let filter = Filter {
        level: Some(Level::Error),
        since: Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()),
        until: Some(Utc.with_ymd_and_hms(2021, 1, 2, 0, 0, 0).unwrap()),
        fields: FieldFilterSet::new(["mod=test", "v=42"]).unwrap(),
    };
    let record = parse(r#"{"level":"error","ts":"2021-01-01T00:00:00Z","mod":"test","v":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"level":"info","ts":"2021-01-01T00:00:00Z","mod":"test","v":42}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"level":"error","ts":"2021-01-01T00:00:00Z","mod":"test","v":43}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"level":"error","ts":"2021-01-01T00:00:00Z","mod":"test2","v":42}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_record_filter_or() {
    let filter = FieldFilter::parse("mod=test")
        .unwrap()
        .or(FieldFilter::parse("v=42").unwrap());
    let record = parse(r#"{"mod":"test","v":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"test","v":43}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"test2","v":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"test2","v":43}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_record_filter_and() {
    let filter = FieldFilter::parse("mod=test")
        .unwrap()
        .and(FieldFilter::parse("v=42").unwrap());
    let record = parse(r#"{"mod":"test","v":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":"test","v":43}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":"test2","v":42}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":"test2","v":43}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_key_match() {
    let filter = FieldFilter::parse("mod.test=42").unwrap();
    let record = parse(r#"{"mod.test":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":{"test":42}}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":{"test":43}}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":{"test":42,"test2":42}}"#);
    assert!(filter.apply(&record));
}

#[test]
fn test_field_filter_key_match_partial() {
    let filter = FieldFilter::parse("mod.test=42").unwrap();
    let record = parse(r#"{"mod.test":42}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod.test2":42}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":{"test":42}}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":{"test2":42}}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":{"test":42,"test2":42}}"#);
    assert!(filter.apply(&record));
    let filter = FieldFilter::parse("mod.test.inner=42").unwrap();
    let record = parse(r#"{"mod":{"test":{"inner":42}}}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod.test":{"inner":42}}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":{"test":{"inner":43}}}"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_key_match_partial_nested() {
    let filter = FieldFilter::parse("mod.test=42").unwrap();
    let record = parse(r#"{"mod":{"test":42}}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":{"test2":42}}"#);
    assert!(!filter.apply(&record));
    let record = parse(r#"{"mod":{"test":42,"test2":42}}"#);
    assert!(filter.apply(&record));
    let record = parse(r#"{"mod":{"test":42,"test2":42,"test3":42}}"#);
    assert!(filter.apply(&record));
}

#[test]
fn test_field_filter_caller() {
    let filter = FieldFilter::parse("caller~=somesource.py").unwrap();
    let record = parse(r#"caller=somesource.py:42"#);
    assert!(filter.apply(&record));
    let record = parse(r#"caller=somesource.go:42"#);
    assert!(!filter.apply(&record));
}

#[test]
fn test_field_filter_array() {
    let any = FieldFilter::parse("span.[].name=a").unwrap();
    let first = FieldFilter::parse("span.[0].name=b").unwrap();

    let record = parse(r#"{"span":[{"name":"a"},{"name":"b"}]}"#);
    assert!(any.apply(&record));
    assert!(!first.apply(&record));

    let inv = FieldFilter::parse("span.[0a].name=a").unwrap();
    assert!(!inv.apply(&record));

    let inv = FieldFilter::parse("span.[0]x=a").unwrap();
    assert!(!inv.apply(&record));

    let inv = FieldFilter::parse("span.[0.name=a").unwrap();
    assert!(!inv.apply(&record));

    let record = parse(r#"{"span":[{"name":"b"},{"name":"c"}]}"#);
    assert!(!any.apply(&record));
    assert!(first.apply(&record));

    let record = parse(r#"{"span":[]}"#);
    assert!(!any.apply(&record));
    assert!(!first.apply(&record));

    let record = parse(r#"{"span":{"name":"a"}}"#);
    assert!(!any.apply(&record));
    assert!(!first.apply(&record));

    let inv = FieldFilter::parse("span.[0=b").unwrap();
    assert!(!inv.apply(&record));

    let record = parse(r#"{"span":10}"#);
    assert!(!any.apply(&record));
    assert!(!first.apply(&record));

    let any = FieldFilter::parse("span.[]=a").unwrap();
    let first = FieldFilter::parse("span.[0]=b").unwrap();

    let record = parse(r#"{"span":["a","b"]}"#);
    assert!(any.apply(&record));
    assert!(!first.apply(&record));

    let inv = FieldFilter::parse("span.[0=b").unwrap();
    assert!(!inv.apply(&record));

    let inv = FieldFilter::parse("span.x=b").unwrap();
    assert!(!inv.apply(&record));

    let inv = FieldFilter::parse("span.[98172389172389172312983761823]=b").unwrap();
    assert!(!inv.apply(&record));
}

#[test]
fn test_raw_object() {
    let obj = RawObject::Json(json::from_str(r#"{"a":1,"b":2}"#).unwrap());
    let obj = obj.parse().unwrap();
    assert_eq!(obj.fields.len(), 2);
    assert_eq!(obj.fields[0].0, "a");
    assert_eq!(obj.fields[1].0, "b");
}

#[test]
fn test_raw_array() {
    let arr = RawArray::Json(json::from_str(r#"[1,2]"#).unwrap());
    let arr = arr.parse::<2>().unwrap();
    assert_eq!(arr.items.len(), 2);
    assert_eq!(arr.items[0].raw_str(), "1");
    assert_eq!(arr.items[1].raw_str(), "2");
}

#[test]
fn test_array_parser_invalid_type() {
    let arr = RawArray::Json(json::from_str(r#"12"#).unwrap());
    let result = arr.parse::<2>();
    assert!(matches!(result, Err(Error::JsonParseError(_))));
}

#[rstest]
#[case(br#"{"some":{"deep":{"message":"test"}}}"#, Some(r#""test""#))]
#[case(br#"{"some":{"deep":[{"message":"test"}]}}"#, None)]
fn test_nested_predefined_fields(#[case] input: &[u8], #[case] expected: Option<&str>) {
    let predefined = PredefinedFields {
        message: Field {
            names: vec!["some.deep.message".into()],
            show: FieldShowOption::Always,
        }
        .into(),
        ..Default::default()
    };
    let settings = ParserSettings::new(&predefined, [], None);
    let parser = Parser::new(settings);

    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let record = parser.parse(&record.record);
    assert_eq!(record.message.map(|x| x.raw_str()), expected);
}

#[rstest]
#[case(br#"{"ts":""}"#, None)]
#[case(br#"{"ts":"3"}"#, Some("3"))]
#[case(br#"ts="""#, None)]
#[case(br#"ts="#, None)]
#[case(br#"ts=1"#, Some("1"))]
#[case(br#"ts="2""#, Some("2"))]
fn test_timestamp(#[case] input: &[u8], #[case] expected: Option<&str>) {
    let parser = Parser::new(ParserSettings::default());
    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let record = parser.parse(&record.record);
    assert_eq!(record.ts.map(|x| x.raw()), expected);
}

#[rstest]
#[case(br#"{"level":""}"#, None)]
#[case(br#"{"level":"info"}"#, Some(Level::Info))]
#[case(br#"level="""#, None)]
#[case(br#"level="#, None)]
#[case(br#"level=info"#, Some(Level::Info))]
#[case(br#"level="info""#, Some(Level::Info))]
fn test_level(#[case] input: &[u8], #[case] expected: Option<Level>) {
    let parser = Parser::new(ParserSettings::default());
    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let record = parser.parse(&record.record);
    assert_eq!(record.level, expected);
}

#[rstest]
#[case(br#"{"logger":""}"#, None)]
#[case(br#"{"logger":"x"}"#, Some("x"))]
#[case(br#"logger="""#, None)]
#[case(br#"logger="#, None)]
#[case(br#"logger=x"#, Some("x"))]
#[case(br#"logger="x""#, Some("x"))]
fn test_logger(#[case] input: &[u8], #[case] expected: Option<&str>) {
    let parser = Parser::new(ParserSettings::default());
    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let record = parser.parse(&record.record);
    assert_eq!(record.logger, expected);
}

#[rstest]
#[case(br#"{"caller":""}"#, Caller::none())]
#[case(br#"{"caller":"x"}"#, Caller::with_name("x"))]
#[case(br#"caller="""#, Caller::none())]
#[case(br#"caller="#, Caller::none())]
#[case(br#"caller=x"#, Caller::with_name("x"))]
#[case(br#"caller="x""#, Caller::with_name("x"))]
fn test_caller(#[case] input: &[u8], #[case] expected: Caller) {
    let parser = Parser::new(ParserSettings::default());
    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let record = parser.parse(&record.record);
    assert_eq!(record.caller, expected);
}

#[rstest]
#[case(br#"{"file":""}"#, Caller::none())] // 1
#[case(br#"{"file":"x"}"#, Caller::with_file_line("x", ""))] // 2
#[case(br#"file="""#, Caller::none())] // 3
#[case(br#"file="#, Caller::none())] // 4
#[case(br#"file=x"#, Caller::with_file_line("x", ""))] // 5
#[case(br#"file="x""#, Caller::with_file_line("x", ""))] // 6
#[case(br#"{"line":""}"#, Caller::none())] // 7
#[case(br#"{"line":"8"}"#, Caller::with_file_line("", "8"))] // 8
#[case(br#"line="""#, Caller::none())] // 9
#[case(br#"line="#, Caller::none())] // 10
#[case(br#"line=11"#, Caller::with_file_line("", "11"))] // 11
#[case(br#"line="12""#, Caller::with_file_line("", "12"))] // 12
#[case(br#"{"file":"","line":""}"#, Caller::none())] // 13
#[case(br#"{"file":"x","line":"14"}"#, Caller::with_file_line("x", "14"))] // 14
#[case(br#"file="" line="""#, Caller::none())] // 15
#[case(br#"file= line="#, Caller::none())] // 16
#[case(br#"file=x line=17"#, Caller::with_file_line("x", "17"))] // 17
#[case(br#"file="x" line="18""#, Caller::with_file_line("x", "18"))] // 18
#[case(br#"{"file":"","line":"19"}"#, Caller::with_file_line("", "19"))] // 19
#[case(br#"{"file":"x","line":""}"#, Caller::with_file_line("x", ""))] // 20
#[case(br#"file="" line="21""#, Caller::with_file_line("", "21"))] // 21
#[case(br#"file= line=22"#, Caller::with_file_line("", "22"))] // 22
#[case(br#"file=x line="#, Caller::with_file_line("x", ""))] // 23
#[case(br#"file="x" line="#, Caller::with_file_line("x", ""))] // 24
#[case(br#"file="x" line=21 line=25"#, Caller::with_file_line("x", "25"))] // 25
#[case(br#"file=x line=26 file=y"#, Caller::with_file_line("y", "26"))] // 26
#[case(br#"{"file":123, "file": {}, "line":27}"#, Caller::with_file_line("", "27"))] // 27
#[case(br#"{"caller":"a", "file": "b", "line":28}"#, Caller{name:"a", file:"b",line:"28"})] // 28
#[case(br#"{"file": "b", "line":{}}"#, Caller::with_file_line("b", ""))] // 29
fn test_caller_file_line(#[case] input: &[u8], #[case] expected: Caller) {
    let predefined = PredefinedFields {
        caller_file: Field {
            names: vec!["file".into()],
            show: FieldShowOption::Always,
        }
        .into(),
        caller_line: Field {
            names: vec!["line".into()],
            show: FieldShowOption::Always,
        }
        .into(),
        ..Default::default()
    };
    let parser = Parser::new(ParserSettings::new(&predefined, [], None));
    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let record = parser.parse(&record.record);
    assert_eq!(record.caller, expected);
}

#[rstest]
#[case(b"price=1", RawValue::Number("1"))] // 1
#[case(b"price=1.1", RawValue::Number("1.1"))] // 2
#[case(b"price=1.1.1", RawValue::String(EncodedString::raw("1.1.1")))] // 3
#[case(b"price=3.787e+04", RawValue::Number("3.787e+04"))] // 4
fn test_logfmt_number(#[case] input: &[u8], #[case] expected: RawValue) {
    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let actual = RawValue::auto(record.record.fields().next().unwrap().1.raw_str());
    assert_eq!(actual, expected);
}

#[rstest]
#[case(br#"message=Synced"#, RawValue::String(EncodedString::raw("Synced")))] // 1
#[case(br#"message="Synced""#, RawValue::String(EncodedString::json(r#""Synced""#)))] // 2
#[case(br#"message="Not synced""#, RawValue::String(EncodedString::json(r#""Not synced""#)))] // 3
fn test_logfmt_string(#[case] input: &[u8], #[case] expected: RawValue) {
    let record = RawRecord::parser().parse(input).next().unwrap().unwrap();
    let actual = RawValue::auto(record.record.fields().next().unwrap().1.raw_str());
    assert_eq!(actual, expected);
}

#[rstest]
#[case(r#"msg=Synced"#, "msg=Synced", true)] // 1
#[case(r#"msg="Synced""#, "msg=Synced", true)] // 2
#[case(r#"msg="Not synced""#, "msg=Not synced", true)] // 3
fn test_logfmt_string_filter(#[case] input: &str, #[case] filter: &str, #[case] expected: bool) {
    let filter = FieldFilter::parse(filter).unwrap();
    let record = parse(input);
    assert_eq!(filter.apply(&record), expected);
}

#[rstest]
#[case("price?!=3", r#"price=3"#, false)] // 1
#[case("price?!=3", r#"price=4"#, true)] // 2
#[case("price?!=3", r#"price=3 price=3"#, false)] // 3
#[case("price?!=3", r#"price=3 price=4"#, true)] // 4
#[case("price?!=3", r#"price=2 price=4"#, true)] // 5
#[case("price?!=3", r#"x=a"#, true)] // 6
#[case("price?=3", r#"price=3"#, true)] // 7
#[case("price?=3", r#"price=4"#, false)] // 8
#[case("price?=3", r#"price=3 price=3"#, true)] // 9
#[case("price?=3", r#"price=3 price=4"#, true)] // 10
#[case("price?=3", r#"price=2 price=4"#, false)] // 11
#[case("price?=3", r#"x=a"#, true)] // 12
fn test_logfmt_filter_include_absent(#[case] filter: &str, #[case] input: &str, #[case] expected: bool) {
    let filter = FieldFilter::parse(filter).unwrap();
    let record = parse(input);
    assert_eq!(filter.apply(&record), expected);
}

fn parse(s: &str) -> Record<'_> {
    try_parse(s).unwrap()
}

fn try_parse(s: &str) -> Result<Record<'_>> {
    let items = RawRecord::parser().parse(s.as_bytes()).collect_vec();
    assert_eq!(items.len(), 1);
    let raw = items.into_iter().next().unwrap()?.record;
    let parser = Parser::new(ParserSettings::default());
    Ok(parser.parse(&raw))
}
