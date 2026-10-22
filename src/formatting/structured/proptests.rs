//! Property tests for structured output.

// std imports
use std::{collections::HashSet, fmt};

// third-party imports
use proptest::prelude::*;
use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

// local imports
use super::*;
use crate::{
    input::InputDescriptor,
    model::{Origin, Parser, ParserSettings, RawRecord, RecordWithSourceConstructor},
    settings::Settings,
};
use ::json;

// ---

/// Formats the first record of `input`, or returns `None` if there is no record in it.
fn format<E: Encoder>(formatter: &StructuredRecordFormatter<E>, input: &str) -> Option<String> {
    let predefined = Settings::default().fields.predefined;
    let parser = Parser::new(ParserSettings::new(&predefined, [], None));
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

fn formatter<E: Encoder>(flatten: bool) -> StructuredRecordFormatter<E> {
    StructuredRecordFormatterBuilder::new()
        .with_predefined_fields(&Settings::default().fields.predefined)
        .with_flatten(flatten)
        .build()
}

// ---

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

fn json_value() -> impl Strategy<Value = json::Value> {
    let leaf = prop_oneof![
        Just(json::Value::Null),
        any::<bool>().prop_map(json::Value::Bool),
        any::<i64>().prop_map(json::Value::from),
        (-1e10f64..1e10).prop_map(json::Value::from),
        "\\PC{0,8}".prop_map(json::Value::String),
        Just(json::Value::String("2026-07-06T10:00:00.5Z".into())),
        Just(json::Value::String("info".into())),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(json::Value::Array),
            prop::collection::vec((key(), inner), 0..4).prop_map(|entries| {
                // Repeated keys are written as is, since serde_json's map would merge them.
                let body = entries
                    .iter()
                    .map(|(k, v)| format!("{}:{}", json::to_string(k).unwrap(), v))
                    .collect::<Vec<_>>()
                    .join(",");
                json::from_str::<json::Value>(&format!("{{{body}}}")).unwrap_or(json::Value::Null)
            }),
        ]
    })
}

/// A JSON record, possibly with repeated keys.
fn json_record() -> impl Strategy<Value = String> {
    prop::collection::vec((key(), json_value()), 0..8).prop_map(|entries| {
        let body = entries
            .iter()
            .map(|(k, v)| format!("{}:{}", json::to_string(k).unwrap(), v))
            .collect::<Vec<_>>()
            .join(",");
        format!("{{{body}}}")
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

// ---

proptest! {
    #[test]
    fn json_output_is_valid(input in json_record(), flatten in any::<bool>()) {
        if let Some(output) = format(&formatter::<JsonEncoder>(flatten), &input) {
            assert_json_object_with_unique_keys(&output);
        }
    }

    #[test]
    fn json_output_from_logfmt_is_valid(input in logfmt_record(), flatten in any::<bool>()) {
        if let Some(output) = format(&formatter::<JsonEncoder>(flatten), &input) {
            assert_json_object_with_unique_keys(&output);
        }
    }
}
