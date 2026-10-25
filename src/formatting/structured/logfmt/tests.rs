use rstest::rstest;

use super::*;

fn encode(f: impl FnOnce(&mut LogfmtEncoder, &mut Vec<u8>)) -> String {
    let mut buf = Vec::new();
    let mut encoder = LogfmtEncoder::default();
    encoder.begin_record(&mut buf);
    f(&mut encoder, &mut buf);
    encoder.end_record(&mut buf);
    String::from_utf8(buf).unwrap()
}

fn string(s: EncodedString) -> String {
    encode(|e, buf| e.string(buf, s))
}

/// Quotes `json` written after `before` in a buffer, checking that `before` is kept.
fn quote(before: &str, json: &str) -> String {
    let mut buf = format!("{before}{json}").into_bytes();
    quote_in_place(&mut buf, before.len());
    let output = String::from_utf8(buf).unwrap();
    assert!(output.starts_with(before), "{output}");
    output[before.len()..].to_owned()
}

fn key(k: &str) -> String {
    encode(|e, buf| {
        e.key(buf, k);
        e.string(buf, EncodedString::raw("x"));
    })
}

#[rstest]
#[case::empty("", r#""""#)]
#[case::no_escapes("[1,2]", r#""[1,2]""#)]
#[case::object(r#"{"a":"x"}"#, r#""{\"a\":\"x\"}""#)]
#[case::backslash(r#"["a\\b"]"#, r#""[\"a\\\\b\"]""#)]
#[case::escaped_quote(r#"["a\"b"]"#, r#""[\"a\\\"b\"]""#)]
#[case::only_escapes(r#""""#, r#""\"\"""#)]
#[case::long_run(&format!(r#"["{}"]"#, "x".repeat(100)), &format!(r#""[\"{}\"]""#, "x".repeat(100)))]
#[case::long_runs_with_escapes(
    &format!(r#"{{"{x}":"{x}\\{x}\"{x}"}}"#, x = "x".repeat(40)),
    &format!(r#""{{\"{x}\":\"{x}\\\\{x}\\\"{x}\"}}""#, x = "x".repeat(40))
)]
fn test_quote_in_place(#[case] json: &str, #[case] expected: &str, #[values("", "k=")] before: &str) {
    assert_eq!(quote(before, json), expected);
}

#[rstest]
#[case::word(EncodedString::raw("started"), "started")]
#[case::unicode(EncodedString::raw("café"), "café")]
#[case::punctuation(EncodedString::raw("a.b:c/d"), "a.b:c/d")]
#[case::json_word(EncodedString::json(r#""started""#), "started")]
#[case::json_escaped_slash(EncodedString::json(r#""a\/b""#), "a/b")]
#[case::json_unicode(EncodedString::json(r#""é""#), "é")]
fn test_bare_strings(#[case] input: EncodedString, #[case] expected: &str) {
    assert_eq!(string(input), expected);
}

#[rstest]
#[case::space(EncodedString::raw("slow request"), r#""slow request""#)]
#[case::equals(EncodedString::raw("a=b"), r#""a=b""#)]
#[case::quote(EncodedString::raw("a\"b"), r#""a\"b""#)]
#[case::leading_quote(EncodedString::raw("\"a"), r#""\"a""#)]
#[case::control(EncodedString::raw("a\tb\nc"), r#""a\tb\nc""#)]
#[case::low_control(EncodedString::raw("\x01"), r#""\u0001""#)]
#[case::unicode_space(EncodedString::raw("é ü"), r#""é ü""#)]
#[case::empty(EncodedString::raw(""), r#""""#)]
#[case::json_empty(EncodedString::json(r#""""#), r#""""#)]
#[case::json_space(EncodedString::json(r#""a b""#), r#""a b""#)]
#[case::json_newline(EncodedString::json(r#""a\nb""#), r#""a\nb""#)]
#[case::json_equals(EncodedString::json(r#""a=b""#), r#""a=b""#)]
#[case::json_space_then_escape(EncodedString::json(r#""a b\/c""#), r#""a b/c""#)]
#[case::json_equals_then_escape(EncodedString::json(r#""a=b\nc""#), r#""a=b\nc""#)]
#[case::json_escape_then_space(EncodedString::json(r#""a\/b c""#), r#""a/b c""#)]
fn test_quoted_strings(#[case] input: EncodedString, #[case] expected: &str) {
    assert_eq!(string(input), expected);
}

#[rstest]
#[case::integer(EncodedString::raw("42"), r#""42""#)]
#[case::float(EncodedString::raw("-1.5"), r#""-1.5""#)]
#[case::plus(EncodedString::raw("+5"), r#""+5""#)]
#[case::exponent(EncodedString::raw("1e3"), r#""1e3""#)]
#[case::bool_true(EncodedString::raw("true"), r#""true""#)]
#[case::bool_false(EncodedString::raw("false"), r#""false""#)]
#[case::null(EncodedString::raw("null"), r#""null""#)]
#[case::json_integer(EncodedString::json(r#""42""#), r#""42""#)]
#[case::json_bool(EncodedString::json(r#""true""#), r#""true""#)]
fn test_strings_typed_otherwise_are_quoted(#[case] input: EncodedString, #[case] expected: &str) {
    assert_eq!(string(input), expected);
}

#[rstest]
#[case::bare(&["main.go", ":", "42"], "main.go:42")]
#[case::spaced(&["f", " @ ", "a.go"], r#""f @ a.go""#)]
#[case::number_like(&["4", "2"], r#""42""#)]
#[case::empty(&[], r#""""#)]
fn test_text(#[case] parts: &[&str], #[case] expected: &str) {
    assert_eq!(encode(|e, buf| e.text(buf, parts)), expected);
}

#[rstest]
#[case::word("ok", "ok")]
#[case::empty("", r#""""#)]
fn test_bare_source_strings(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(encode(|e, buf| e.bare_string(buf, input)), expected);
}

#[rstest]
#[case::word("a", "a=x")]
#[case::dotted("a.b", "a.b=x")]
#[case::space("user name", r#""user name"=x"#)]
#[case::brackets("items[0]", r#""items[0]"=x"#)]
#[case::quote("a\"b", r#""a\"b"=x"#)]
#[case::empty("", r#"""=x"#)]
fn test_keys(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(key(input), expected);
}

#[test]
fn test_pairs() {
    let output = encode(|e, buf| {
        e.key(buf, "a");
        e.number(buf, "1", NumberSource::Json);
        e.key(buf, "b");
        e.boolean(buf, true);
        e.key(buf, "c");
        e.null(buf);
    });
    assert_eq!(output, "a=1 b=true c=null");
}

#[rstest]
fn test_numbers_verbatim(#[values("0", "-1.5", "1e10", "+5", ".5", "007", "5.")] input: &str) {
    assert_eq!(encode(|e, buf| e.number(buf, input, NumberSource::Logfmt)), input);
}

#[test]
fn test_plain() {
    let output = encode(|e, buf| {
        assert!(e.plain(buf, |buf| {
            buf.extend(b"2026-07-06T10:00:00Z");
            true
        }));
        assert!(!e.plain(buf, |buf| {
            buf.extend(b"junk");
            false
        }));
    });
    assert_eq!(output, "2026-07-06T10:00:00Z");
}

#[test]
fn test_nested_values_are_quoted_json() {
    let output = encode(|e, buf| {
        e.key(buf, "http");
        e.begin_object(buf);
        e.key(buf, "status");
        e.number(buf, "200", NumberSource::Json);
        e.key(buf, "path");
        e.string(buf, EncodedString::raw("/a \"b\" \\c"));
        e.key(buf, "tags");
        e.begin_array(buf);
        e.item(buf);
        e.string(buf, EncodedString::raw("x"));
        e.item(buf);
        e.number(buf, "+5", NumberSource::Logfmt);
        e.end_array(buf);
        e.end_object(buf);
        e.key(buf, "next");
        e.string(buf, EncodedString::raw("y"));
    });
    assert_eq!(
        output,
        r#"http="{\"status\":200,\"path\":\"/a \\\"b\\\" \\\\c\",\"tags\":[\"x\",5]}" next=y"#
    );

    // Every kind of string is written as JSON inside a nested value.
    let output = encode(|e, buf| {
        e.key(buf, "o");
        e.begin_object(buf);
        e.key(buf, "bare");
        e.bare_string(buf, "x");
        e.key(buf, "text");
        e.text(buf, &["a", " b"]);
        e.key(buf, "plain");
        e.plain(buf, |buf| {
            buf.extend(b"p");
            true
        });
        e.end_object(buf);
    });
    assert_eq!(output, r#"o="{\"bare\":\"x\",\"text\":\"a b\",\"plain\":\"p\"}""#);
}

#[test]
fn test_arrays_are_quoted_json() {
    let output = encode(|e, buf| {
        e.key(buf, "tags");
        e.begin_array(buf);
        e.item(buf);
        e.string(buf, EncodedString::raw("é"));
        e.item(buf);
        e.begin_object(buf);
        e.end_object(buf);
        e.end_array(buf);
        e.key(buf, "empty");
        e.begin_array(buf);
        e.end_array(buf);
    });
    assert_eq!(output, r#"tags="[\"é\",{}]" empty="[]""#);
}

#[test]
fn test_quoted_json_reads_back() {
    let output = encode(|e, buf| {
        e.key(buf, "v");
        e.begin_array(buf);
        e.item(buf);
        e.string(buf, EncodedString::raw("a\"b\\c\nd"));
        e.end_array(buf);
    });
    let parsed: std::collections::HashMap<String, String> = serde_logfmt::logfmt::from_str(&output).unwrap();
    assert_eq!(parsed["v"], r#"["a\"b\\c\nd"]"#);
}
