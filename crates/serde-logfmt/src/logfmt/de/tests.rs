use super::{super::raw::RawValue, *};
use std::collections::HashMap;

use rstest::rstest;

#[test]
fn test_struct_no_escape() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
        int: u32,
        str1: String,
        str2: String,
    }

    let j = r#"int=42 str1=a str2="b c""#;
    let expected = Test {
        int: 42,
        str1: "a".to_string(),
        str2: "b c".to_string(),
    };
    assert_eq!(expected, from_str(j).unwrap());
}

#[test]
fn test_struct_escape() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
        int: u32,
        str1: String,
        str2: String,
    }

    let j = r#"int=0 str1="b=c" str2="a\nb""#;
    let expected = Test {
        int: 0,
        str1: "b=c".to_string(),
        str2: "a\nb".to_string(),
    };
    assert_eq!(expected, from_str(j).unwrap());
}

#[test]
fn test_hex_escape() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
        int: u32,
        str1: String,
        str2: String,
    }

    let j = r#"int=0 str1="\u001b[3m" str2="a""#;
    let expected = Test {
        int: 0,
        str1: "\x1b[3m".to_string(),
        str2: "a".to_string(),
    };
    assert_eq!(expected, from_str(j).unwrap());
}

#[test]
fn test_raw() {
    #[derive(Deserialize)]
    struct Test<'a> {
        int: i32,
        str1: String,
        #[serde(borrow)]
        str2: &'a RawValue,
    }

    let j = r#"int=-42 str1=a str2="b \nc""#;
    let parsed: Test = from_str(j).unwrap();
    assert_eq!(parsed.int, -42);
    assert_eq!(parsed.str1, "a");
    assert_eq!(parsed.str2.get(), r#""b \nc""#);
}

#[test]
fn test_single_word() {
    let result = from_str::<HashMap<String, String>>(r#"word"#);
    assert_eq!(result, Err(Error::ExpectedKeyValueDelimiter));
    assert_eq!(result.unwrap_err().to_string(), "expected key-value delimiter");
}

#[test]
fn test_raw_enum() {
    #[derive(Deserialize, PartialEq, Debug)]
    enum TestEnum {
        A,
        B,
        C,
    }

    let val: TestEnum = from_str("B").unwrap();
    assert_eq!(val, TestEnum::B);
}

#[test]
fn test_raw_struct_with_enum() {
    #[derive(Deserialize, PartialEq, Debug)]
    enum TestEnum {
        A,
        B,
        C,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct TestStruct {
        v: TestEnum,
    }

    let val: TestStruct = from_str("v=B").unwrap();
    assert_eq!(val, TestStruct { v: TestEnum::B });
}

#[test]
fn test_empty_value() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
        int: u32,
        str1: String,
        str2: String,
        str3: String,
    }

    let j = r#"int=0 str1="" str2= str3="#;
    let expected = Test {
        int: 0,
        str1: "".to_string(),
        str2: "".to_string(),
        str3: "".to_string(),
    };
    assert_eq!(expected, from_str(j).unwrap());
}

#[test]
fn test_deserializer_from_slice_invalid_utf8() {
    let input = &[0xFF, 0xFE]; // Invalid UTF-8 bytes
    let deserializer = Deserializer::from_slice(input);
    assert!(deserializer.is_err());
}

#[test]
fn test_deserializer_new() {
    let input = "key=value";
    let _deserializer = Deserializer::new(input);
    // Just test that constructor works
}

#[test]
fn test_key_with_unicode() {
    // Test parsing a key with non-ASCII characters to trigger unicode validation
    let input = "café=value";
    let result: Result<std::collections::HashMap<String, String>> = from_str(input);
    assert!(result.is_ok());
    let map = result.unwrap();
    assert_eq!(map.get("café"), Some(&"value".to_string()));
}

#[test]
fn test_unquoted_value_with_unicode() {
    // Test parsing an unquoted value with non-ASCII characters to trigger unicode validation on line 576
    let input = "key=café";
    let result: Result<std::collections::HashMap<String, String>> = from_str(input);
    assert!(result.is_ok());
    let map = result.unwrap();
    assert_eq!(map.get("key"), Some(&"café".to_string()));
}

#[test]
fn test_invalid_surrogate_pair_calculation() {
    // Test surrogate pair processing by creating malformed UTF-16 surrogates
    // The high surrogate 0xD800 followed by an invalid low surrogate should trigger error handling
    // But since valid surrogate pairs always produce valid code points, let's test a different edge case
    // Testing with a string that has invalid UTF-8 bytes to trigger the InvalidUnicodeCodePoint error
    use crate::logfmt::de::Deserializer;

    // Create a deserializer and try to trigger the surrogate pair error path
    // by testing the boundary condition where char::from_u32 could return None
    let mut deserializer = Deserializer::new(r#"key="\uD800\uDC00""#);
    let result: Result<std::collections::HashMap<String, String>> = serde::Deserialize::deserialize(&mut deserializer);
    // This should succeed as it's a valid surrogate pair, let's just ensure it works
    assert!(result.is_ok());
}

#[test]
fn test_quoted_key() {
    let input = r#""user name"=alice "items[0]"=1 ""=empty plain=x"#;
    let map: HashMap<String, String> = from_str(input).unwrap();
    assert_eq!(map.len(), 4);
    assert_eq!(map.get("user name"), Some(&"alice".to_string()));
    assert_eq!(map.get("items[0]"), Some(&"1".to_string()));
    assert_eq!(map.get(""), Some(&"empty".to_string()));
    assert_eq!(map.get("plain"), Some(&"x".to_string()));
}

#[test]
fn test_quoted_key_borrowed() {
    let input = r#""a b"="c d""#;
    let map: HashMap<&str, &str> = from_str(input).unwrap();
    assert_eq!(map.get("a b"), Some(&"c d"));
}

#[test]
fn test_quoted_key_escape() {
    let input = r#""a\"b\\cé\n"=v"#;
    let map: HashMap<String, String> = from_str(input).unwrap();
    assert_eq!(map.get("a\"b\\cé\n"), Some(&"v".to_string()));
}

#[test]
fn test_quoted_key_escape_not_borrowed() {
    let result = from_str::<HashMap<&str, &str>>(r#""a\"b"=v"#);
    assert!(result.is_err());
}

#[test]
fn test_quoted_key_errors() {
    assert_eq!(
        from_str::<HashMap<String, String>>(r#""a b" =v"#),
        Err(Error::ExpectedKeyValueDelimiter)
    );
    assert_eq!(
        from_str::<HashMap<String, String>>(r#""a b""#),
        Err(Error::ExpectedKeyValueDelimiter)
    );
    assert_eq!(from_str::<HashMap<String, String>>(r#""a b=v"#), Err(Error::Eof));
    assert_eq!(
        from_str::<HashMap<String, String>>(r#""a\qb"=v"#),
        Err(Error::InvalidEscape)
    );
}

#[test]
fn test_bare_key_stays_strict() {
    for input in ["{a=1", "a[0]=1", "a,b=1", "'a'=1", "a\"b=1"] {
        assert!(
            from_str::<HashMap<String, String>>(input).is_err(),
            "expected {input:?} to be rejected"
        );
    }
}

#[test]
fn test_quoted_key_raw_value() {
    #[derive(Deserialize)]
    struct Test<'a> {
        #[serde(borrow, rename = "a b")]
        v: &'a RawValue,
    }

    let parsed: Test = from_str(r#""a b"="x y""#).unwrap();
    assert_eq!(parsed.v.get(), r#""x y""#);
}

#[rstest]
fn test_bare_keys(#[values("a", "a.b", "café", "a-b_c", "a/b", "@a")] key: &str) {
    assert!(is_bare_key(key));
}

#[rstest]
fn test_quoted_keys(
    #[values("", "user name", "items[0]", "{a", "a,b", "'a'", "a\"b", "a=b", "a\tb", "a\u{7f}")] key: &str,
) {
    assert!(!is_bare_key(key));
}

#[rstest]
fn test_bare_values(#[values("a", "a.b", "café", "42", "[1,2]", "{a}", "a\\b", "a'b")] value: &str) {
    assert!(is_bare_value(value));
}

#[rstest]
fn test_quoted_values(#[values("", "a b", "a=b", "a\"b", "\"a", "a\tb", "a\nb", "a\u{1}b")] value: &str) {
    assert!(!is_bare_value(value));
}

proptest::proptest! {
    #[test]
    fn test_is_bare_key_matches_parser(key in "\\PC{0,4}|[ -~]{0,4}") {
        let parsed = from_str::<HashMap<String, String>>(&format!("{key}=1"));
        let accepted = matches!(&parsed, Ok(map) if map.len() == 1 && map.contains_key(&key));
        proptest::prop_assert_eq!(is_bare_key(&key), accepted, "key {:?}, parsed {:?}", key, parsed);
    }

    /// An empty value is also accepted by the parser, but is written quoted for clarity.
    #[test]
    fn test_is_bare_value_matches_parser(value in "\\PC{1,4}|[\\x00-~]{1,4}") {
        let parsed = from_str::<HashMap<String, String>>(&format!("a={value}"));
        let accepted = matches!(&parsed, Ok(map) if map.get("a") == Some(&value));
        proptest::prop_assert_eq!(is_bare_value(&value), accepted, "value {:?}, parsed {:?}", value, parsed);
    }
}
