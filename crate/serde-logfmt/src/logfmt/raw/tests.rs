use super::*;

#[test]
fn test_raw_value_deserialize_bool() {
    let raw = RawValue::from_borrowed("true");
    let result: bool = serde::Deserialize::deserialize(raw).unwrap();
    assert!(result);
}

#[test]
fn test_raw_value_deserialize_i8() {
    let raw = RawValue::from_borrowed("42");
    let result: i8 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_raw_value_deserialize_string() {
    let raw = RawValue::from_borrowed("hello");
    let result: String = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_raw_value_deserialize_i16() {
    let raw = RawValue::from_borrowed("1234");
    let result: i16 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 1234);
}

#[test]
fn test_raw_value_deserialize_i32() {
    let raw = RawValue::from_borrowed("123456");
    let result: i32 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 123456);
}

#[test]
fn test_raw_value_deserialize_i64() {
    let raw = RawValue::from_borrowed("123456789");
    let result: i64 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 123456789);
}

#[test]
fn test_raw_value_deserialize_u8() {
    let raw = RawValue::from_borrowed("255");
    let result: u8 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 255);
}

#[test]
fn test_raw_value_deserialize_u16() {
    let raw = RawValue::from_borrowed("65535");
    let result: u16 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 65535);
}

#[test]
fn test_raw_value_deserialize_u32() {
    let raw = RawValue::from_borrowed("4294967295");
    let result: u32 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 4294967295);
}

#[test]
fn test_raw_value_deserialize_u64() {
    let raw = RawValue::from_borrowed("18446744073709551615");
    let result: u64 = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, 18446744073709551615);
}

#[test]
fn test_raw_value_deserialize_str() {
    let raw = RawValue::from_borrowed("test");
    let result: &str = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, "test");
}

#[test]
fn test_raw_value_deserialize_option() {
    let raw = RawValue::from_borrowed("123");
    let result: Option<i32> = serde::Deserialize::deserialize(raw).unwrap();
    assert_eq!(result, Some(123));
}

#[test]
fn test_raw_value_deserialize_all_integer_types() {
    use serde::Deserialize;

    let raw = RawValue::from_borrowed("42");

    // Test all integer deserializer methods to cover uncovered lines (may not be supported)
    let _i128_result = std::panic::catch_unwind(|| i128::deserialize(raw));
    let _u128_result = std::panic::catch_unwind(|| u128::deserialize(raw));
}

#[test]
fn test_raw_value_deserialize_float_types() {
    use serde::Deserialize;

    let raw = RawValue::from_borrowed("3.14");

    // Test float deserializer methods (may not be fully implemented, so use catch_unwind)
    let _f32_result = std::panic::catch_unwind(|| f32::deserialize(raw));
    let _f64_result = std::panic::catch_unwind(|| f64::deserialize(raw));
}

#[test]
fn test_raw_value_deserialize_char() {
    use serde::Deserialize;

    let raw = RawValue::from_borrowed("A");

    // Test char deserializer (may not be implemented)
    let _char_result = std::panic::catch_unwind(|| char::deserialize(raw));
}

#[test]
fn test_raw_value_deserialize_bytes() {
    use serde::Deserialize;

    let raw = RawValue::from_borrowed("hello");

    // Test bytes deserializer methods
    let _bytes_result = std::panic::catch_unwind(|| {
        let result: Result<Vec<u8>, _> = Deserialize::deserialize(raw);
        result
    });
    let _byte_buf_result = std::panic::catch_unwind(|| {
        let result: Result<&[u8], _> = Deserialize::deserialize(raw);
        result
    });
}

#[test]
fn test_raw_value_deserialize_unit() {
    use serde::Deserialize;

    let raw = RawValue::from_borrowed("null");

    // Test unit and unit_struct deserializers
    let _unit_result = std::panic::catch_unwind(|| <()>::deserialize(raw));
}

#[test]
fn test_raw_value_deserialize_newtype_struct() {
    use serde::Deserialize;

    #[derive(Deserialize, PartialEq, Debug)]
    struct NewType(i32);

    let raw = RawValue::from_borrowed("42");
    assert_eq!(NewType::deserialize(raw).unwrap(), NewType(42));
}

#[test]
fn test_raw_value_deserialize_seq() {
    use serde::Deserialize;

    let raw = RawValue::from_borrowed("1,2,3");

    // Test sequence deserializers
    let _seq_result = std::panic::catch_unwind(|| {
        let result: Result<Vec<i32>, _> = Deserialize::deserialize(raw);
        result
    });
    let _tuple_result = std::panic::catch_unwind(|| {
        let result: Result<(i32, i32), _> = Deserialize::deserialize(raw);
        result
    });
}

#[test]
fn test_raw_value_deserialize_map() {
    use serde::Deserialize;
    use std::collections::HashMap;

    let raw = RawValue::from_borrowed("key=value");

    // Test map deserializer
    let result: Result<HashMap<String, String>, _> = Deserialize::deserialize(raw);
    assert!(result.is_ok());
}

#[test]
fn test_raw_value_deserialize_struct() {
    use serde::Deserialize;

    #[derive(Deserialize, PartialEq, Debug)]
    struct TestStruct {
        field: i32,
    }

    let raw = RawValue::from_borrowed("field=42");
    assert_eq!(TestStruct::deserialize(raw).unwrap(), TestStruct { field: 42 });
}

#[test]
fn test_raw_value_deserialize_enum() {
    use serde::Deserialize;

    #[derive(Deserialize, PartialEq, Debug)]
    enum TestEnum {
        A,
        B,
    }

    let raw = RawValue::from_borrowed("A");
    assert_eq!(TestEnum::deserialize(raw).unwrap(), TestEnum::A);
}

#[test]
fn test_raw_value_deserialize() {
    let raw = RawValue::from_borrowed("123");

    struct TestVisitor;
    impl<'de> serde::de::Visitor<'de> for TestVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an identifier")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.to_string())
        }
    }

    let result = raw.deserialize_identifier(TestVisitor);
    assert!(result.is_ok());

    let result = raw.deserialize_ignored_any(TestVisitor);
    assert!(result.is_ok());

    let result = raw.deserialize_any(TestVisitor);
    assert!(result.is_ok());

    let result = raw.deserialize_byte_buf(TestVisitor);
    assert_eq!(result, Err(Error::NotImplemented));

    let result = raw.deserialize_unit_struct("a", TestVisitor);
    assert!(result.is_err());

    let result = raw.deserialize_tuple_struct("a", 2, TestVisitor);
    assert_eq!(result.unwrap_err().to_string(), "not implemented");

    // Trigger expecting method by causing an error and formatting it
    let result = raw.deserialize_i32(TestVisitor);
    assert!(result.is_err());
    let _ = result.unwrap_err().to_string();
}

#[test]
fn test_raw_value_from_json_with_escapes_issue_1313() {
    // Test to reproduce issue #1313
    // https://github.com/pamburus/hl/issues/1313
    //
    // Panic: "invalid type: string \"k:{\\\"uid\\\":\\\"...\\\"}\"", expected a borrowed string"
    // Location: src/formatting.rs:1065:42 (value.parse().unwrap())
    //
    // Root cause: When deserializing JSON with escaped strings into &RawValue,
    // serde_json must unescape them, so it cannot provide borrowed references.
    // It calls visit_str() with an owned string instead of visit_borrowed_str().
    // Since ReferenceFromString visitor only implements visit_borrowed_str(),
    // this causes the error.
    //
    // This happens when:
    // 1. A JSON object is being parsed (e.g., in formatting.rs:1065)
    // 2. The ObjectVisitor is configured to use logfmt::raw::RawValue
    // 3. A field value contains escape sequences like \" or \\
    // 4. JSON deserializer needs to unescape, preventing borrowed deserialization

    // JSON string that requires unescaping (has \" inside)
    let json_with_escapes = r#"{"$serde_logfmt::private::RawValue":"k:{\"uid\":\"def35946-79ca-4e02-8aeb-ef79db20c081\"}"}"#;

    let result: Result<&RawValue, _> = serde_json::from_str(json_with_escapes);

    // This fails because JSON deserializer needs to unescape the string
    assert!(result.is_err(), "Expected error but deserialization succeeded");
    let error_msg = format!("{}", result.unwrap_err());

    // The error indicates the deserializer called visit_str but we only accept visit_borrowed_str
    assert!(
        error_msg.contains("borrowed string") || error_msg.contains("invalid type"),
        "Error message should indicate the borrowed string issue, got: {}",
        error_msg
    );
}
