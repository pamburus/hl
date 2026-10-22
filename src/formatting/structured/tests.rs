use super::*;
use crate::{
    IncludeExcludeKeyFilter, KeyMatchOptions,
    input::InputDescriptor,
    model::{Origin, Parser, ParserSettings, RawRecord, RecordWithSourceConstructor},
    settings::{Field, PredefinedFields, Settings},
};

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
    let parser = Parser::new(ParserSettings::new(predefined, [], None));
    let raw = RawRecord::parser().parse(input.as_bytes()).next().unwrap().unwrap();
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
    String::from_utf8(buf).unwrap()
}

fn json(builder: StructuredRecordFormatterBuilder, input: &str) -> String {
    format_with(&builder.build::<JsonEncoder>(), &predefined(), input)
}

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
        let formatter = formatter().with_flatten(flatten).build::<JsonEncoder>();
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
            assert_eq!(allocations, 0, "flatten={flatten}: {input}");
        }
    }
}
