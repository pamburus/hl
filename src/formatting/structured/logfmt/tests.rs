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

fn raw(s: &str) -> String {
    string(EncodedString::raw(s))
}

fn key(k: &str) -> String {
    encode(|e, buf| {
        e.key(buf, k);
        e.string(buf, EncodedString::raw("x"));
    })
}

#[test]
fn test_bare_strings() {
    assert_eq!(raw("started"), "started");
    assert_eq!(raw("café"), "café");
    assert_eq!(raw("a.b:c/d"), "a.b:c/d");
    assert_eq!(string(EncodedString::json(r#""started""#)), "started");
    assert_eq!(string(EncodedString::json(r#""a\/b""#)), "a/b");
    assert_eq!(string(EncodedString::json(r#""é""#)), "é");
}

#[test]
fn test_quoted_strings() {
    assert_eq!(raw("slow request"), r#""slow request""#);
    assert_eq!(raw("a=b"), r#""a=b""#);
    assert_eq!(raw("a\"b"), r#""a\"b""#);
    assert_eq!(raw("\"a"), r#""\"a""#);
    assert_eq!(raw("a\tb\nc"), r#""a\tb\nc""#);
    assert_eq!(raw("\x01"), r#""\u0001""#);
    assert_eq!(raw("é ü"), r#""é ü""#);
    assert_eq!(raw(""), r#""""#);
    assert_eq!(string(EncodedString::json(r#""""#)), r#""""#);
    assert_eq!(string(EncodedString::json(r#""a b""#)), r#""a b""#);
    assert_eq!(string(EncodedString::json(r#""a\nb""#)), r#""a\nb""#);
    assert_eq!(string(EncodedString::json(r#""a b""#)), r#""a b""#);
}

#[test]
fn test_strings_typed_otherwise_are_quoted() {
    for s in ["42", "-1.5", "+5", "1e3", "true", "false", "null"] {
        assert_eq!(raw(s), format!("\"{s}\""), "{s}");
    }
    assert_eq!(string(EncodedString::json(r#""42""#)), r#""42""#);
    assert_eq!(string(EncodedString::json(r#""true""#)), r#""true""#);
}

#[test]
fn test_text() {
    assert_eq!(encode(|e, buf| e.text(buf, &["main.go", ":", "42"])), "main.go:42");
    assert_eq!(encode(|e, buf| e.text(buf, &["f", " @ ", "a.go"])), r#""f @ a.go""#);
    assert_eq!(encode(|e, buf| e.text(buf, &["4", "2"])), r#""42""#);
    assert_eq!(encode(|e, buf| e.text(buf, &[])), r#""""#);
}

#[test]
fn test_keys() {
    assert_eq!(key("a"), "a=x");
    assert_eq!(key("a.b"), "a.b=x");
    assert_eq!(key("user name"), r#""user name"=x"#);
    assert_eq!(key("items[0]"), r#""items[0]"=x"#);
    assert_eq!(key("a\"b"), r#""a\"b"=x"#);
    assert_eq!(key(""), r#"""=x"#);
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

#[test]
fn test_numbers_verbatim() {
    for n in ["0", "-1.5", "1e10", "+5", ".5", "007", "5."] {
        assert_eq!(encode(|e, buf| e.number(buf, n, NumberSource::Logfmt)), n);
    }
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
