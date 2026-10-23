// std imports
use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::LazyLock,
};

// third-party imports
use proptest::prelude::*;
use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

// local imports
use super::*;
use crate::{
    IncludeExcludeKeyFilter, KeyMatchOptions,
    input::InputDescriptor,
    model::{Origin, Parser, ParserSettings, RawRecord, RecordWithSourceConstructor},
    settings::{Field, PredefinedFields, Settings},
};
use ::json;

fn predefined() -> PredefinedFields {
    Settings::default().fields.predefined
}

fn formatter() -> StructuredRecordFormatterBuilder {
    StructuredRecordFormatterBuilder::new().with_predefined_fields(&predefined())
}

fn hide(keys: &[&str]) -> Arc<IncludeExcludeKeyFilter> {
    let mut filter = IncludeExcludeKeyFilter::new(KeyMatchOptions::default());
    for key in keys {
        if let Some(key) = key.strip_prefix('!') {
            filter.entry(key).include();
        } else if *key == "*" {
            filter = filter.excluded();
        } else {
            filter.entry(key).exclude();
        }
    }
    Arc::new(filter)
}

fn format_with<E: Encoder>(
    formatter: &StructuredRecordFormatter<E>,
    predefined: &PredefinedFields,
    input: &str,
) -> String {
    try_format_with(formatter, predefined, input).unwrap()
}

/// Formats the first record of `input`, or returns `None` if there is no record in it.
fn try_format_with<E: Encoder>(
    formatter: &StructuredRecordFormatter<E>,
    predefined: &PredefinedFields,
    input: &str,
) -> Option<String> {
    let parser = Parser::new(ParserSettings::new(predefined, [], None));
    let raw = RawRecord::parser().parse(input.as_bytes()).next()?.ok()?;
    let record = parser.parse(&raw.record);
    let descriptor = InputDescriptor::new(0, "test");
    let origin = Origin::new(&descriptor, 0, input.len());
    let mut buf = Vec::new();
    RecordWithSourceFormatter::format_record(
        formatter,
        &mut buf,
        0..0,
        record.with_source(&input.as_bytes()[raw.offsets], origin),
    );
    Some(String::from_utf8(buf).unwrap())
}

fn json(builder: StructuredRecordFormatterBuilder, input: &str) -> String {
    format_with(&builder.build::<JsonEncoder>(), &predefined(), input)
}

fn no_allocations_after_warm_up<E: Encoder>() {
    use crate::testing::alloc::count_allocations;

    // Values nested two levels deep within an object are left out here: parsing such an object
    // allocates in the JSON parser, which the pretty formatter does as well.
    let inputs = [
        r#"{"ts":"2026-07-06T10:00:00.5+02:00","level":"warn","logger":"l","msg":"m","caller":"c","a":{"b":[1,2],"d":"x\/y"},"f":[1,{"c":2}],"e":1}"#,
        r#"{"ts":1700000000.25,"msg":{"k":1,"k":2},"r":1,"r":2,"a.b":3,"a":{"b":4,"c":{"d":5}}}"#,
        r#"ts=yesterday level=info msg="hello world" n=+5 t=true tag=a tag=b"#,
    ];
    let predefined = predefined();
    let parser = Parser::new(ParserSettings::new(&predefined, [], None));
    let descriptor = InputDescriptor::new(0, "test");

    for flatten in [false, true] {
        let formatter = formatter().with_flatten(flatten).build::<E>();
        for input in inputs {
            let raw = RawRecord::parser().parse(input.as_bytes()).next().unwrap().unwrap();
            let record = parser.parse(&raw.record);
            let mut buf = Vec::with_capacity(1024);
            let mut format = || {
                buf.clear();
                let origin = Origin::new(&descriptor, 0, input.len());
                RecordWithSourceFormatter::format_record(
                    &formatter,
                    &mut buf,
                    0..0,
                    record.with_source(input.as_bytes(), origin),
                );
            };
            // The first run may grow the per-thread scratch.
            format();
            let ((), allocations) = count_allocations(&mut format);
            assert_eq!(
                allocations,
                0,
                "{}, flatten={flatten}: {input}",
                std::any::type_name::<E>()
            );
        }
    }
}

// ---
// Property test helpers.

/// Formats the first record of `input` with the default settings and `flatten`.
fn format<E: Encoder>(input: &str, flatten: bool) -> Option<String> {
    try_format_with(&formatter().with_flatten(flatten).build::<E>(), &predefined(), input)
}

/// A JSON value that checks that every object in it has unique keys.
struct UniqueKeys;

impl<'de> Deserialize<'de> for UniqueKeys {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueKeysVisitor)
    }
}

struct UniqueKeysVisitor;

impl<'de> Visitor<'de> for UniqueKeysVisitor {
    type Value = UniqueKeys;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<UniqueKeys, A::Error> {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(serde::de::Error::custom(format!("duplicate key {key:?}")));
            }
            map.next_value::<UniqueKeys>()?;
        }
        Ok(UniqueKeys)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<UniqueKeys, A::Error> {
        while seq.next_element::<UniqueKeys>()?.is_some() {}
        Ok(UniqueKeys)
    }

    fn visit_bool<E>(self, _: bool) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }

    fn visit_i64<E>(self, _: i64) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }

    fn visit_u64<E>(self, _: u64) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }

    fn visit_f64<E>(self, _: f64) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }

    fn visit_str<E>(self, _: &str) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }

    fn visit_unit<E>(self) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
}

fn assert_json_object_with_unique_keys(line: &str) {
    assert!(line.starts_with('{'), "not an object: {line}");
    assert!(!line.contains('\n'), "not a single line: {line}");
    if let Err(err) = json::from_str::<UniqueKeys>(line) {
        panic!("invalid output {line}: {err}");
    }
    json::from_str::<IgnoredAny>(line).unwrap();
}

// ---

/// Keys, including the names of predefined fields and keys with dots.
fn key() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("ts".to_owned()),
        Just("timestamp".to_owned()),
        Just("level".to_owned()),
        Just("msg".to_owned()),
        Just("message".to_owned()),
        Just("logger".to_owned()),
        Just("caller".to_owned()),
        Just("a".to_owned()),
        Just("b".to_owned()),
        Just("a.b".to_owned()),
        "[a-z]{1,3}(\\.[a-z]{1,2})?",
    ]
}

fn json_leaf() -> impl Strategy<Value = json::Value> {
    prop_oneof![
        Just(json::Value::Null),
        any::<bool>().prop_map(json::Value::Bool),
        any::<i64>().prop_map(json::Value::from),
        (-1e10f64..1e10).prop_map(json::Value::from),
        "\\PC{0,8}".prop_map(json::Value::String),
        Just(json::Value::String("2026-07-06T10:00:00.5Z".into())),
        Just(json::Value::String("info".into())),
    ]
}

/// A JSON object with the given entries, possibly with repeated keys.
fn json_object(entries: &[(String, impl fmt::Display)]) -> String {
    // Repeated keys are written as is, since serde_json's map would merge them.
    let body = entries
        .iter()
        .map(|(k, v)| format!("{}:{}", json::to_string(k).unwrap(), v))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{body}}}")
}

fn json_value() -> impl Strategy<Value = json::Value> {
    json_leaf().prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(json::Value::Array),
            prop::collection::vec((key(), inner), 0..4)
                .prop_map(|entries| json::from_str(&json_object(&entries)).unwrap_or(json::Value::Null)),
        ]
    })
}

/// A JSON value that logfmt output keeps as is when flattening: no arrays and no empty objects.
fn json_flat_value() -> impl Strategy<Value = json::Value> {
    json_leaf().prop_recursive(3, 24, 4, |inner| {
        prop::collection::vec((key(), inner), 1..4)
            .prop_map(|entries| json::from_str(&json_object(&entries)).unwrap_or(json::Value::Null))
    })
}

/// A JSON record, possibly with repeated keys.
fn json_record() -> impl Strategy<Value = String> {
    prop::collection::vec((key(), json_value()), 0..8).prop_map(|entries| json_object(&entries))
}

/// Source keys of the predefined fields with the default settings.
static PREDEFINED: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let p = Settings::default().fields.predefined;
    let names = [&p.time.names, &p.message.names, &p.logger.names, &p.caller.names];
    let levels = p.level.variants.iter().map(|v| &v.names);
    names.into_iter().chain(levels).flatten().cloned().collect()
});

/// A JSON record with values from [`json_flat_value`], possibly with repeated keys.
///
/// Predefined fields hold no objects, since they are written as values and not flattened.
fn json_flat_record() -> impl Strategy<Value = String> {
    prop::collection::vec((key(), json_flat_value()), 0..8).prop_map(|mut entries| {
        entries.retain(|(k, v)| !(v.is_object() && PREDEFINED.contains(k)));
        json_object(&entries)
    })
}

fn logfmt_value() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9_.:+-]{1,8}",
        Just("true".to_owned()),
        Just("null".to_owned()),
        Just("+5".to_owned()),
        Just(".5".to_owned()),
        Just("007".to_owned()),
        Just("5.e3".to_owned()),
        "\\PC{0,8}".prop_map(|s| json::to_string(&s).unwrap()),
    ]
}

/// A logfmt record, possibly with repeated keys.
fn logfmt_record() -> impl Strategy<Value = String> {
    prop::collection::vec(("[a-z]{1,3}(\\.[a-z]{1,2})?", logfmt_value()), 1..8).prop_map(|entries| {
        entries
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn assert_logfmt_record(line: &str) {
    assert!(!line.contains('\n'), "not a single line: {line}");
    if let Err(err) = serde_logfmt::logfmt::from_str::<HashMap<String, String>>(line) {
        panic!("invalid output {line}: {err:?}");
    }
}

/// Formats `input` as JSON, both directly and through logfmt output read back.
fn json_directly_and_through_logfmt(input: &str, flatten: bool) -> Option<(String, String)> {
    let json = format::<JsonEncoder>(input, flatten)?;
    let logfmt = format::<LogfmtEncoder>(input, flatten)?;
    assert_logfmt_record(&logfmt);
    let through = format::<JsonEncoder>(&logfmt, flatten)?;
    Some((json, through))
}

// ---

#[test]
fn test_predefined_order_and_presence() {
    assert_eq!(
        json(
            formatter(),
            r#"{"x":1,"caller":"c.go:1","msg":"m","logger":"l","level":"info","ts":"2026-07-06T10:00:00Z","y":2}"#
        ),
        r#"{"timestamp":"2026-07-06T10:00:00Z","level":"info","logger":"l","message":"m","caller":"c.go:1","x":1,"y":2}"#
    );
    assert_eq!(json(formatter(), r#"{"msg":"m"}"#), r#"{"message":"m"}"#);
    assert_eq!(json(formatter(), r#"{"x":"y"}"#), r#"{"x":"y"}"#);
    assert_eq!(json(formatter(), r#"{}"#), r#"{}"#);
}

#[test]
fn test_predefined_values() {
    // Levels use the canonical names.
    assert_eq!(
        json(formatter(), r#"{"level":"WARN","msg":"m"}"#),
        r#"{"level":"warning","message":"m"}"#
    );
    // An unrecognized level stays among the regular fields.
    assert_eq!(
        json(formatter(), r#"{"level":"loud","msg":"m"}"#),
        r#"{"message":"m","level":"loud"}"#
    );
    // Messages keep their source type.
    assert_eq!(
        json(formatter(), r#"{"msg":{"a":1, "b":[1, 2]}}"#),
        r#"{"message":{"a":1,"b":[1,2]}}"#
    );
    assert_eq!(json(formatter(), r#"{"msg":42}"#), r#"{"message":42}"#);
    // Timestamps are normalized, or kept with their source type when they can't be parsed.
    assert_eq!(
        json(formatter(), r#"{"ts":1700000000000}"#),
        r#"{"timestamp":"2023-11-14T22:13:20.000Z"}"#
    );
    assert_eq!(json(formatter(), r#"{"ts":-1e300}"#), r#"{"timestamp":-1e300}"#);
    assert_eq!(
        json(formatter(), r#"{"ts":"yesterday"}"#),
        r#"{"timestamp":"yesterday"}"#
    );
    assert_eq!(
        json(formatter(), r#"{"ts":"tomorrow \"maybe\""}"#),
        r#"{"timestamp":"tomorrow \"maybe\""}"#
    );
}

#[test]
fn test_predefined_values_logfmt_source() {
    assert_eq!(
        json(formatter(), r#"ts=yesterday level=info msg=42 x=y"#),
        r#"{"timestamp":"yesterday","level":"info","message":42,"x":"y"}"#
    );
    assert_eq!(
        json(formatter(), r#"ts=-1e300 msg="hello world""#),
        r#"{"timestamp":-1e300,"message":"hello world"}"#
    );
}

#[test]
fn test_combined_caller() {
    let mut predefined = predefined();
    predefined.caller_file = Field::new(vec!["file".into()]).into();
    predefined.caller_line = Field::new(vec!["line".into()]).into();
    let formatter = StructuredRecordFormatterBuilder::new()
        .with_predefined_fields(&predefined)
        .build::<JsonEncoder>();
    let test = |input, expected| assert_eq!(format_with(&formatter, &predefined, input), expected, "{input}");

    test(r#"{"caller":"main"}"#, r#"{"caller":"main"}"#);
    test(r#"{"file":"f.go","line":42}"#, r#"{"caller":"f.go:42"}"#);
    test(r#"{"file":"f.go"}"#, r#"{"caller":"f.go"}"#);
    test(
        r#"{"caller":"fn","file":"f.go","line":"7"}"#,
        r#"{"caller":"fn @ f.go:7"}"#,
    );
}

#[test]
fn test_repeated_keys() {
    assert_eq!(json(formatter(), r#"tag=a x=1 tag=b"#), r#"{"tag":"b","x":1}"#);
    assert_eq!(
        json(formatter(), r#"{"o":{"k":1,"j":2,"k":3},"p":[{"a":1,"a":2}]}"#),
        r#"{"o":{"k":3,"j":2},"p":[{"a":2}]}"#
    );
    // A repeated object is replaced as a whole.
    assert_eq!(
        json(formatter().with_flatten(true), r#"{"a":{"b":1},"x":0,"a":{"c":2}}"#),
        r#"{"a.c":2,"x":0}"#
    );
}

#[test]
fn test_collisions() {
    // A raw field with a predefined output key is dropped.
    assert_eq!(
        json(formatter(), r#"{"msg":"m","message":"raw"}"#),
        r#"{"message":"m"}"#
    );
    assert_eq!(
        json(formatter(), r#"{"message":"raw","msg":"m"}"#),
        r#"{"message":"m"}"#
    );
    assert_eq!(
        json(
            formatter(),
            r#"{"ts":"2026-07-06T10:00:00Z","timestamp":"x","caller":"c","level":"info"}"#
        ),
        r#"{"timestamp":"2026-07-06T10:00:00Z","level":"info","caller":"c"}"#
    );
    // Nested source fields of predefined fields are not repeated.
    assert_eq!(
        json(formatter(), r#"{"span":{"name":"s","id":1},"msg":"m"}"#),
        r#"{"logger":"s","message":"m","span":{"id":1}}"#
    );
    assert_eq!(
        json(formatter(), r#"{"span":{"name":"s"},"msg":"m"}"#),
        r#"{"logger":"s","message":"m"}"#
    );
}

#[test]
fn test_flattening() {
    let input = r#"{"a":{"b":1,"c":{"d":2}},"items":[{"id":1,"tags":{"a":1}}],"e":{}}"#;
    assert_eq!(
        json(formatter().with_flatten(false), input),
        r#"{"a":{"b":1,"c":{"d":2}},"items":[{"id":1,"tags":{"a":1}}],"e":{}}"#
    );
    assert_eq!(
        json(formatter().with_flatten(true), input),
        r#"{"a.b":1,"a.c.d":2,"items":[{"id":1,"tags":{"a":1}}],"e":{}}"#
    );
    // Keys that become equal after flattening are repeats.
    assert_eq!(
        json(formatter().with_flatten(true), r#"{"a":{"b":1},"x":0,"a.b":2}"#),
        r#"{"a.b":2,"x":0}"#
    );
    assert_eq!(
        json(formatter().with_flatten(true), r#"{"a.b":1,"x":0,"a":{"b":2,"c":3}}"#),
        r#"{"a.b":2,"x":0,"a.c":3}"#
    );
    assert_eq!(
        json(formatter().with_flatten(true), r#"{"a":{"b.c":1,"b":{"c":2,"d":3}}}"#),
        r#"{"a.b.c":2,"a.b.d":3}"#
    );
    // Without flattening, they are distinct keys.
    assert_eq!(
        json(formatter().with_flatten(false), r#"{"a":{"b":1},"a.b":2}"#),
        r#"{"a":{"b":1},"a.b":2}"#
    );
}

#[test]
fn test_hiding() {
    let input = r#"{"msg":"m","caller":"c","secret":"s","o":{"k":1,"j":2},"arr":[{"secret":1}]}"#;
    assert_eq!(
        json(formatter().with_field_filter(hide(&["secret"])), input),
        r#"{"message":"m","caller":"c","o":{"k":1,"j":2},"arr":[{"secret":1}]}"#
    );
    assert_eq!(
        json(formatter().with_field_filter(hide(&["o.k"])), input),
        r#"{"message":"m","caller":"c","secret":"s","o":{"j":2},"arr":[{"secret":1}]}"#
    );
    assert_eq!(
        json(formatter().with_field_filter(hide(&["o.k"])).with_flatten(true), input),
        r#"{"message":"m","caller":"c","secret":"s","o.j":2,"arr":[{"secret":1}]}"#
    );
    // Predefined fields are matched by their output keys.
    assert_eq!(
        json(formatter().with_field_filter(hide(&["caller", "message"])), input),
        r#"{"secret":"s","o":{"k":1,"j":2},"arr":[{"secret":1}]}"#
    );
    assert_eq!(
        json(formatter().with_field_filter(hide(&["*", "!message", "!o"])), input),
        r#"{"message":"m","o":{"k":1,"j":2}}"#
    );
    // An object with all its fields hidden is kept, as in pretty output.
    assert_eq!(
        json(
            formatter().with_field_filter(hide(&["o.k", "o.j"])),
            r#"{"o":{"k":1,"j":2}}"#
        ),
        r#"{"o":{}}"#
    );
}

#[test]
fn test_hiding_empty_fields() {
    let input = r#"{"msg":"","a":"","b":null,"c":{},"d":[],"e":{"f":""},"g":[""],"h":0}"#;
    assert_eq!(
        json(formatter(), input),
        r#"{"message":"","a":"","b":null,"c":{},"d":[],"e":{"f":""},"g":[""],"h":0}"#
    );
    assert_eq!(
        json(formatter().with_empty_fields_hiding(true), input),
        r#"{"g":[""],"h":0}"#
    );
}

#[test]
fn test_compaction() {
    let input = "{\n  \"msg\": \"m\",\n  \"o\": { \"a\": [ 1, 2 ] },\n  \"s\": \"a\\/b\"\n}";
    assert_eq!(json(formatter(), input), r#"{"message":"m","o":{"a":[1,2]},"s":"a/b"}"#);
}

#[test]
fn test_no_allocations_after_warm_up() {
    no_allocations_after_warm_up::<JsonEncoder>();
    no_allocations_after_warm_up::<LogfmtEncoder>();
}

// ---
// Property tests.

proptest! {
    #[test]
    fn json_output_is_valid(input in json_record(), flatten in any::<bool>()) {
        if let Some(output) = format::<JsonEncoder>(&input, flatten) {
            assert_json_object_with_unique_keys(&output);
        }
    }

    #[test]
    fn json_output_from_logfmt_is_valid(input in logfmt_record(), flatten in any::<bool>()) {
        if let Some(output) = format::<JsonEncoder>(&input, flatten) {
            assert_json_object_with_unique_keys(&output);
        }
    }
}

proptest! {
    #[test]
    fn logfmt_output_is_valid(input in json_record(), flatten in any::<bool>()) {
        if let Some(output) = format::<LogfmtEncoder>(&input, flatten) {
            assert_logfmt_record(&output);
        }
    }

    /// Logfmt converted to JSON and back to logfmt keeps keys, values and types (SC-003).
    #[test]
    fn logfmt_round_trip(input in logfmt_record(), flatten in any::<bool>()) {
        let Some(json) = format::<JsonEncoder>(&input, flatten) else {
            return Ok(());
        };
        let logfmt = format::<LogfmtEncoder>(&json, flatten).unwrap();
        assert_logfmt_record(&logfmt);
        let direct = format::<LogfmtEncoder>(&input, flatten).unwrap();
        // Both read back as the same record, though numbers may be written differently.
        prop_assert_eq!(
            format::<JsonEncoder>(&logfmt, flatten),
            format::<JsonEncoder>(&direct, flatten)
        );
        prop_assert_eq!(format::<JsonEncoder>(&logfmt, flatten).unwrap(), json);
    }

    /// JSON converted to logfmt and back keeps keys, values and types, except for arrays and
    /// empty objects, which logfmt holds as strings.
    #[test]
    fn json_round_trip(input in json_flat_record()) {
        if let Some((json, through)) = json_directly_and_through_logfmt(&input, true) {
            prop_assert_eq!(through, json);
        }
    }
}
