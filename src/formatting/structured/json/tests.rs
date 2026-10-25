use rstest::rstest;

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

#[rstest]
#[case::plain(EncodedString::json(r#""plain text""#), r#""plain text""#)]
#[case::unicode(EncodedString::json(r#""café""#), r#""café""#)]
#[case::empty(EncodedString::json(r#""""#), r#""""#)]
// Escapes are normalized.
#[case::escaped_slash(EncodedString::json(r#""a\/b""#), r#""a/b""#)]
#[case::escaped_ascii(EncodedString::json(r#""\u0041""#), r#""A""#)]
#[case::escaped_unicode(EncodedString::json(r#""\u00e9""#), r#""é""#)]
#[case::surrogate_pair(EncodedString::json(r#""\ud83d\ude00""#), r#""😀""#)]
#[case::short_escapes(EncodedString::json(r#""a\"b\\c\n\t""#), r#""a\"b\\c\n\t""#)]
#[case::control(EncodedString::json(r#""\u0001""#), r#""\u0001""#)]
// Lone surrogates are replaced.
#[case::lone_high_surrogate(EncodedString::json(r#""a\ud800b""#), "\"a\u{fffd}b\"")]
#[case::lone_low_surrogate(EncodedString::json(r#""\udc00""#), "\"\u{fffd}\"")]
#[case::high_surrogate_then_ascii(EncodedString::json(r#""\ud800A""#), "\"\u{fffd}A\"")]
// Raw strings are escaped.
#[case::raw_plain(EncodedString::raw("plain"), r#""plain""#)]
#[case::raw_quote_and_backslash(EncodedString::raw("a\"b\\c"), r#""a\"b\\c""#)]
#[case::raw_control(EncodedString::raw("line\nnext\x7f"), r#""line\nnext\u007f""#)]
#[case::raw_escape(EncodedString::raw("\x1b"), r#""\u001b""#)]
#[case::raw_unicode(EncodedString::raw("日本"), r#""日本""#)]
fn test_strings(#[case] input: EncodedString, #[case] expected: &str) {
    assert_eq!(string(input), expected);
}

#[rstest]
fn test_numbers_verbatim(
    #[values("0", "-1", "1.25", "1e10", "-1.5E-3", "10000000000")] input: &str,
    #[values(NumberSource::Json, NumberSource::Logfmt)] source: NumberSource,
) {
    assert_eq!(number(input, source), input);
}

#[rstest]
#[case::plus("+5", "5")]
#[case::leading_dot(".5", "0.5")]
#[case::trailing_dot("5.", "5")]
#[case::leading_zeros("007", "7")]
#[case::negative_leading_zeros("-007.50", "-7.50")]
#[case::trailing_dot_exponent("5.e3", "5e3")]
#[case::plus_leading_dot_exponent("+.5e-3", "0.5e-3")]
#[case::negative_leading_dot("-.5", "-0.5")]
#[case::double_zero("00", "0")]
#[case::negative_zero("-0", "-0")]
#[case::fraction("0.5", "0.5")]
#[case::exponent_plus("1E+5", "1E+5")]
fn test_logfmt_numbers_normalized(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(number(input, NumberSource::Logfmt), expected);
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
