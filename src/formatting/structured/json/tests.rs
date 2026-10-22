use super::*;
use crate::formatting::structured::{Encoder, NumberSource};

fn encode(f: impl FnOnce(&mut JsonEncoder, &mut Vec<u8>)) -> String {
    let mut buf = Vec::new();
    let mut encoder = JsonEncoder::default();
    f(&mut encoder, &mut buf);
    String::from_utf8(buf).unwrap()
}

fn string(s: EncodedString) -> String {
    encode(|e, buf| e.string(buf, s))
}

fn number(s: &str, source: NumberSource) -> String {
    encode(|e, buf| e.number(buf, s, source))
}

#[test]
fn test_json_string_verbatim() {
    assert_eq!(string(EncodedString::json(r#""plain text""#)), r#""plain text""#);
    assert_eq!(string(EncodedString::json(r#""café""#)), r#""café""#);
    assert_eq!(string(EncodedString::json(r#""""#)), r#""""#);
}

#[test]
fn test_json_string_normalized() {
    assert_eq!(string(EncodedString::json(r#""a\/b""#)), r#""a/b""#);
    assert_eq!(string(EncodedString::json(r#""A""#)), r#""A""#);
    assert_eq!(string(EncodedString::json(r#""é""#)), r#""é""#);
    assert_eq!(string(EncodedString::json(r#""😀""#)), r#""😀""#);
    assert_eq!(string(EncodedString::json(r#""a\"b\\c\n\t""#)), r#""a\"b\\c\n\t""#);
    assert_eq!(string(EncodedString::json(r#""\u0001""#)), r#""\u0001""#);
    // Lone surrogates are replaced.
    assert_eq!(string(EncodedString::json(r#""a\ud800b""#)), "\"a\u{fffd}b\"");
    assert_eq!(string(EncodedString::json(r#""\udc00""#)), "\"\u{fffd}\"");
    assert_eq!(string(EncodedString::json(r#""\ud800A""#)), "\"\u{fffd}A\"");
    // Raw control characters, as quoted logfmt values may hold, are escaped.
    assert_eq!(string(EncodedString::json("\"a\tb\"")), r#""a\tb""#);
}

#[test]
fn test_raw_string_escaped() {
    assert_eq!(string(EncodedString::raw("plain")), r#""plain""#);
    assert_eq!(string(EncodedString::raw("a\"b\\c")), r#""a\"b\\c""#);
    assert_eq!(string(EncodedString::raw("line\nnext\x7f")), r#""line\nnext\u007f""#);
    assert_eq!(string(EncodedString::raw("\x1b")), r#""\u001b""#);
    assert_eq!(string(EncodedString::raw("日本")), r#""日本""#);
}

#[test]
fn test_numbers() {
    for n in ["0", "-1", "1.25", "1e10", "-1.5E-3", "10000000000"] {
        assert_eq!(number(n, NumberSource::Json), n);
        assert_eq!(number(n, NumberSource::Logfmt), n);
    }
    let test = |input, expected| assert_eq!(number(input, NumberSource::Logfmt), expected, "{input}");
    test("+5", "5");
    test(".5", "0.5");
    test("5.", "5");
    test("007", "7");
    test("-007.50", "-7.50");
    test("5.e3", "5e3");
    test("+.5e-3", "0.5e-3");
    test("-.5", "-0.5");
    test("00", "0");
    test("-0", "-0");
    test("0.5", "0.5");
    test("1E+5", "1E+5");
}

#[test]
fn test_compact_structure() {
    let output = encode(|e, buf| {
        e.begin_record(buf);
        e.key(buf, "a");
        e.number(buf, "1", NumberSource::Json);
        e.key(buf, "b");
        e.begin_object(buf);
        e.key(buf, "c");
        e.boolean(buf, true);
        e.key(buf, "d");
        e.null(buf);
        e.end_object(buf);
        e.key(buf, "e");
        e.begin_array(buf);
        e.item(buf);
        e.begin_object(buf);
        e.end_object(buf);
        e.item(buf);
        e.boolean(buf, false);
        e.end_array(buf);
        e.key(buf, "key \"q\"");
        e.begin_array(buf);
        e.end_array(buf);
        e.end_record(buf);
    });
    assert_eq!(
        output,
        r#"{"a":1,"b":{"c":true,"d":null},"e":[{},false],"key \"q\"":[]}"#
    );
}
