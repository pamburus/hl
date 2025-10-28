// third-party imports
use assert_matches::assert_matches;
use chrono::TimeZone;
use maplit::hashmap;
use rstest::rstest;
use serde_logfmt::logfmt;

// local imports
use super::*;
use crate::{
    error::Error,
    format,
    settings::{Field, FieldShowOption},
};

#[test]
fn test_raw_record_parser_empty_line() {
    // note: in v1 an empty line would produce a record with zero fields
    let settings = Settings::default();
    let mut parser = settings.new_parser(format::Auto::default(), b"").unwrap();
    assert!(parser.next().is_none());
}

#[test]
fn test_raw_record_parser_empty_object() {
    let settings = Settings::default();
    let mut parser = settings.new_parser(format::Auto::default(), b"{}").unwrap();

    let rec = parser.next().unwrap().unwrap();
    assert_eq!(rec.fields().iter().count(), 0);
}

#[test]
fn test_raw_record_parser_invalid_type() {
    // note: in v1 an invalid type would produce parse error
    let settings = Settings::default();
    let mut parser = settings.new_parser(format::Json {}, b"12").unwrap();
    assert!(parser.next().is_none());
}
