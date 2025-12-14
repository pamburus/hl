use super::{string::new_message_format, *};
use crate::{
    datefmt::LinuxDateFormat,
    model::{Caller, RawObject, Record, RecordFields, RecordWithSourceConstructor},
    settings::{AsciiMode, MessageFormat, MessageFormatting},
    testing::Sample,
    timestamp::Timestamp,
    timezone::Tz,
};
use chrono::{Offset, Utc};
use encstr::EncodedString;
use serde_json as json;

trait FormatToVec {
    fn format_to_vec(&self, rec: &Record) -> Vec<u8>;
}

trait FormatToString {
    fn format_to_string(&self, rec: &Record) -> String;
}

impl FormatToVec for RecordFormatter {
    fn format_to_vec(&self, rec: &Record) -> Vec<u8> {
        let mut buf = Vec::new();
        self.format_record(&mut buf, rec);
        buf
    }
}

impl FormatToString for RecordFormatter {
    fn format_to_string(&self, rec: &Record) -> String {
        String::from_utf8(self.format_to_vec(rec)).unwrap()
    }
}

fn formatter() -> RecordFormatterBuilder {
    RecordFormatterBuilder::sample()
        .with_theme(Sample::sample())
        .with_timestamp_formatter(DateTimeFormatter::new(
            LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
            Tz::FixedOffset(Utc.fix()),
        ))
        .with_options(Formatting {
            flatten: None,
            message: MessageFormatting {
                format: MessageFormat::AutoQuoted,
            },
            punctuation: Sample::sample(),
        })
}

fn format(rec: &Record) -> String {
    formatter().build().format_to_string(rec)
}

fn format_no_color(rec: &Record) -> String {
    formatter().with_theme(Default::default()).build().format_to_string(rec)
}

fn json_raw_value(s: &str) -> Box<json::value::RawValue> {
    json::value::RawValue::from_string(s.into()).unwrap()
}

trait RecordExt<'a> {
    fn from_fields(fields: &[(&'a str, RawValue<'a>)]) -> Record<'a>;
}

impl<'a> RecordExt<'a> for Record<'a> {
    fn from_fields(fields: &[(&'a str, RawValue<'a>)]) -> Record<'a> {
        Record {
            fields: RecordFields::from_slice(fields),
            ..Default::default()
        }
    }
}

#[test]
fn test_nested_objects() {
    let ka = json_raw_value(r#"{"va":{"kb":42,"kc":43}}"#);
    let rec = Record {
        ts: Some(Timestamp::new("2000-01-02T03:04:05.123Z")),
        message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
        level: Some(Level::Debug),
        logger: Some("tl"),
        caller: Caller::with_name("tc"),
        fields: RecordFields::from_slice(&[("k_a", RawValue::from(RawObject::Json(&ka)))]),
        ..Default::default()
    };

    assert_eq!(
        &format(&rec),
        "\u{1b}[0;2;3m00-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0m \u{1b}[0;1;39mtm \u{1b}[0;32mk-a\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mva\u{1b}[0;2m=\u{1b}[0;33m{ \u{1b}[0;32mkb\u{1b}[0;2m=\u{1b}[0;94m42 \u{1b}[0;32mkc\u{1b}[0;2m=\u{1b}[0;94m43\u{1b}[0;33m } }\u{1b}[0;2;3m -> tc\u{1b}[0m",
    );

    assert_eq!(
        &formatter().with_flatten(true).build().format_to_string(&rec),
        "\u{1b}[0;2;3m00-01-02 03:04:05.123 \u{1b}[0;36m|\u{1b}[0;95mDBG\u{1b}[0;36m|\u{1b}[0;2;3m \u{1b}[0;2;4mtl:\u{1b}[0m \u{1b}[0;1;39mtm \u{1b}[0;32mk-a.va.kb\u{1b}[0;2m=\u{1b}[0;94m42 \u{1b}[0;32mk-a.va.kc\u{1b}[0;2m=\u{1b}[0;94m43\u{1b}[0;2;3m -> tc\u{1b}[0m",
    );
}

#[test]
fn test_timestamp_none() {
    let rec = Record {
        message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
        level: Some(Level::Error),
        ..Default::default()
    };

    assert_eq!(&format(&rec), "\u{1b}[0;7;91m|ERR|\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m");
}

#[test]
fn test_level_trace() {
    let rec = Record {
        message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
        level: Some(Level::Trace),
        ..Default::default()
    };

    assert_eq!(
        &format(&rec),
        "\u{1b}[0;36m|\u{1b}[0;2mTRC\u{1b}[0;36m|\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m"
    );
}

#[test]
fn test_timestamp_none_always_show() {
    let rec = Record {
        message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
        ..Default::default()
    };

    assert_eq!(
        &formatter().with_always_show_time(true).build().format_to_string(&rec),
        "\u{1b}[0;2;3m---------------------\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m",
    );
}

#[test]
fn test_level_none() {
    let rec = Record {
        message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
        ..Default::default()
    };

    assert_eq!(&format(&rec), "\u{1b}[0;1;39mtm\u{1b}[0m",);
}

#[test]
fn test_level_none_always_show() {
    let rec = Record {
        message: Some(RawValue::String(EncodedString::json(r#""tm""#))),
        ..Default::default()
    };

    assert_eq!(
        &formatter().with_always_show_level(true).build().format_to_string(&rec),
        "\u{1b}[0;36m|(?)|\u{1b}[0m \u{1b}[0;1;39mtm\u{1b}[0m"
    );
}

#[test]
fn test_string_value_raw() {
    let v = "v";
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), "k=v");
}

#[test]
fn test_string_value_json_simple() {
    let v = r#""some-value""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k=some-value"#);
}

#[test]
fn test_string_value_json_space() {
    let v = r#""some value""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k="some value""#);
}

#[test]
fn test_string_value_raw_space() {
    let v = r#"some value"#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k="some value""#);
}

#[test]
fn test_string_value_json_space_and_double_quotes() {
    let v = r#""some \"value\"""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k='some "value"'"#);
}

#[test]
fn test_string_value_raw_space_and_double_quotes() {
    let v = r#"some "value""#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k='some "value"'"#);
}

#[test]
fn test_string_value_json_space_and_single_quotes() {
    let v = r#""some 'value'""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k="some 'value'""#);
}

#[test]
fn test_string_value_raw_space_and_single_quotes() {
    let v = r#"some 'value'"#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k="some 'value'""#);
}

#[test]
fn test_string_value_json_space_and_backticks() {
    let v = r#""some `value`""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k="some `value`""#);
}

#[test]
fn test_string_value_raw_space_and_backticks() {
    let v = r#"some `value`"#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k="some `value`""#);
}

#[test]
fn test_string_value_json_space_and_double_and_single_quotes() {
    let v = r#""some \"value\" from 'source'""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k=`some "value" from 'source'`"#);
}

#[test]
fn test_string_value_raw_space_and_double_and_single_quotes() {
    let v = r#"some "value" from 'source'"#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k=`some "value" from 'source'`"#);
}

#[test]
fn test_string_value_json_backslash() {
    let v = r#""some-\\\"value\\\"""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k=`some-\"value\"`"#);
}

#[test]
fn test_string_value_raw_backslash() {
    let v = r#"some-\"value\""#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), r#"k=`some-\"value\"`"#);
}

#[test]
fn test_string_value_json_space_and_double_and_single_quotes_and_backticks() {
    let v = r#""some \"value\" from 'source' with `sauce`""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(
        &format_no_color(&rec),
        r#"k="some \"value\" from 'source' with `sauce`""#
    );
}

#[test]
fn test_string_value_raw_space_and_double_and_single_quotes_and_backticks() {
    let v = r#"some "value" from 'source' with `sauce`"#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(
        &format_no_color(&rec),
        r#"k="some \"value\" from 'source' with `sauce`""#
    );
}

#[test]
fn test_string_value_json_tabs() {
    let v = r#""some\tvalue""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), "k=`some\tvalue`");
}

#[test]
fn test_string_value_raw_tabs() {
    let v = "some\tvalue";
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), "k=`some\tvalue`");
}

#[test]
fn test_string_value_json_control_chars() {
    let v = r#""some-\u001b[1mvalue\u001b[0m""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
}

#[test]
fn test_string_value_raw_control_chars() {
    let rec = Record::from_fields(&[("k", EncodedString::raw("some-\x1b[1mvalue\x1b[0m").into())]);

    let result = format_no_color(&rec);
    assert_eq!(&result, r#"k="some-\u001b[1mvalue\u001b[0m""#, "{}", result);
}

#[test]
fn test_string_value_json_control_chars_and_quotes() {
    let v = r#""some-\u001b[1m\"value\"\u001b[0m""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
}

#[test]
fn test_string_value_raw_control_chars_and_quotes() {
    let v = "some-\x1b[1m\"value\"\x1b[0m";
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(format_no_color(&rec), r#"k="some-\u001b[1m\"value\"\u001b[0m""#);
}

#[test]
fn test_string_value_json_ambiguous() {
    for v in ["true", "false", "null", "{}", "[]"] {
        let v = format!(r#""{}""#, v);
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }
}

#[test]
fn test_string_value_raw_ambiguous() {
    for v in ["true", "false", "null"] {
        let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }
    for v in ["{}", "[]"] {
        let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k="{}""#, v));
    }
}

#[test]
fn test_string_value_json_number() {
    for v in ["42", "42.42", "-42", "-42.42"] {
        let v = format!(r#""{}""#, v);
        let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }
    for v in [
        "42128731867381927389172983718293789127389172938712983718927",
        "42.128731867381927389172983718293789127389172938712983718927",
    ] {
        let qv = format!(r#""{}""#, v);
        let rec = Record::from_fields(&[("k", EncodedString::json(&qv).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }
}

#[test]
fn test_string_value_raw_number() {
    for v in ["42", "42.42", "-42", "-42.42"] {
        let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }
    for v in [
        "42128731867381927389172983718293789127389172938712983718927",
        "42.128731867381927389172983718293789127389172938712983718927",
    ] {
        let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
        assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
    }
}

#[test]
fn test_string_value_json_version() {
    let v = "1.1.0";
    let qv = format!(r#""{}""#, v);
    let rec = Record::from_fields(&[("k", EncodedString::json(&qv).into())]);
    assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
}

#[test]
fn test_string_value_raw_version() {
    let v = "1.1.0";
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
}

#[test]
fn test_string_value_json_hyphen() {
    let v = "-";
    let qv = format!(r#""{}""#, v);
    let rec = Record::from_fields(&[("k", EncodedString::json(&qv).into())]);
    assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
}

#[test]
fn test_string_value_raw_hyphen() {
    let v = "-";
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(format_no_color(&rec), format!(r#"k={}"#, v));
}

#[test]
fn test_string_value_trailing_space() {
    let input = "test message\n";
    let golden = r#""test message""#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(input).into())]);
    assert_eq!(format_no_color(&rec), format!(r#"k={}"#, golden));
}

#[test]
fn test_message_empty() {
    let rec = Record {
        message: Some(EncodedString::raw("").into()),
        ..Default::default()
    };

    let result = format_no_color(&rec);
    assert_eq!(&result, "", "{}", result);
}

#[test]
fn test_message_double_quoted() {
    let rec = Record {
        message: Some(EncodedString::raw(r#""hello, world""#).into()),
        ..Default::default()
    };

    let result = format_no_color(&rec);
    assert_eq!(&result, r#"'"hello, world"'"#, "{}", result);
}

#[test]
fn test_message_single_quoted() {
    let rec = Record {
        message: Some(EncodedString::raw(r#"'hello, world'"#).into()),
        ..Default::default()
    };

    let result = format_no_color(&rec);
    assert_eq!(&result, r#""'hello, world'""#, "{}", result);
}

#[test]
fn test_message_single_and_double_quoted() {
    let rec = Record {
        message: Some(EncodedString::raw(r#"'hello, "world"'"#).into()),
        ..Default::default()
    };

    let result = format_no_color(&rec);
    assert_eq!(&result, r#"`'hello, "world"'`"#, "{}", result);
}

#[test]
fn test_message_control_chars() {
    let rec = Record {
        message: Some(EncodedString::raw("hello, \x1b[33mworld\x1b[0m").into()),
        ..Default::default()
    };

    let result = format_no_color(&rec);
    assert_eq!(&result, r#""hello, \u001b[33mworld\u001b[0m""#, "{}", result);
}

#[test]
fn test_message_spaces_only() {
    let rec = Record {
        message: Some(EncodedString::raw("    ").into()),
        ..Default::default()
    };

    let result = format_no_color(&rec);
    assert_eq!(&result, r#""#, "{}", result);
}

#[test]
fn test_nested_hidden_fields_flatten() {
    let val = json_raw_value(r#"{"b":{"c":{"d":1,"e":2},"f":3}}"#);
    let rec = Record::from_fields(&[("a", RawObject::Json(&val).into())]);
    let mut fields = IncludeExcludeKeyFilter::default();
    let b = fields.entry("a").entry("b");
    b.exclude();
    b.entry("c").entry("d").include();
    let formatter = RecordFormatterBuilder {
        flatten: true,
        theme: Some(Default::default()), // No theme for consistent test output
        fields: Some(fields.into()),
        ..formatter()
    }
    .build();

    assert_eq!(&formatter.format_to_string(&rec), "a.b.c.d=1 ...");
}

#[test]
fn test_nested_hidden_fields_group_unhide() {
    let val = json_raw_value(r#"{"b":{"c":{"d":1,"e":2},"f":3}}"#);
    let rec = Record::from_fields(&[("a", RawObject::Json(&val).into())]);
    let mut fields = IncludeExcludeKeyFilter::default();
    fields.entry("a.b.c").exclude();
    fields.entry("a.b.c.e").include();
    fields.entry("a.b.c").exclude();
    let formatter = RecordFormatterBuilder {
        flatten: true,
        theme: Some(Default::default()), // No theme for consistent test output
        fields: Some(fields.into()),
        ..formatter()
    }
    .build();

    assert_eq!(&formatter.format_to_string(&rec), "a.b.f=3 ...");
}

#[test]
fn test_nested_hidden_fields_no_flatten() {
    let val = json_raw_value(r#"{"b":{"c":{"d":1,"e":2},"f":3}}"#);
    let rec = Record::from_fields(&[("a", RawObject::Json(&val).into())]);
    let mut fields = IncludeExcludeKeyFilter::default();
    let b = fields.entry("a").entry("b");
    b.exclude();
    b.entry("c").entry("d").include();
    let formatter = RecordFormatterBuilder {
        flatten: false,
        theme: Some(Default::default()), // No theme for consistent test output
        fields: Some(fields.into()),
        ..formatter()
    }
    .build();

    assert_eq!(&formatter.format_to_string(&rec), "a={ b={ c={ d=1 ... } ... } }");
}

#[test]
fn test_caller() {
    let rec = Record {
        caller: Caller {
            name: "test_function",
            file: "test_file.rs",
            line: "42",
        },
        ..Default::default()
    };

    let result = format_no_color(&rec);
    assert_eq!(&result, " -> test_function @ test_file.rs:42", "{}", result);
}

#[test]
fn test_no_op_record_with_source_formatter() {
    let formatter = NoOpRecordWithSourceFormatter;
    let rec = Record::default();
    let rec = rec.with_source(b"src");
    formatter.format_record(&mut Buf::default(), rec);
}

#[test]
fn test_delimited_message_with_colors() {
    let formatter = formatter()
        .with_message_format(new_message_format(MessageFormat::Delimited, "::"))
        .build();

    let rec = Record {
        ts: Some(Timestamp::new("2000-01-02T03:04:05.123Z")),
        fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
        ..Default::default()
    };
    assert_eq!(
        formatter.format_to_string(&rec),
        "\u{1b}[0;2;3m00-01-02 03:04:05.123\u{1b}[0m \u{1b}[0;2;3m:: \u{1b}[0;32ma\u{1b}[0;2m=\u{1b}[0;94m42\u{1b}[0m"
    );
}

#[test]
fn test_auto_quoted_message() {
    let formatter = formatter()
        .with_theme(Default::default())
        .with_message_format(new_message_format(MessageFormat::AutoQuoted, ""))
        .build();

    let mut rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
        ..Default::default()
    };
    assert_eq!(formatter.format_to_string(&rec), "m a=42");

    rec.fields = Default::default();
    assert_eq!(formatter.format_to_string(&rec), "m");

    rec.message = Some(EncodedString::raw("m x=1").into());
    assert_eq!(formatter.format_to_string(&rec), r#""m x=1""#);

    rec.message = Some(EncodedString::raw("m '1'").into());
    assert_eq!(formatter.format_to_string(&rec), r#"m '1'"#);

    rec.message = Some(EncodedString::raw(r#"m '1' and "2""#).into());
    assert_eq!(formatter.format_to_string(&rec), r#"m '1' and "2""#);

    rec.message = Some(EncodedString::raw(r#"m x='1' and y="2""#).into());
    assert_eq!(formatter.format_to_string(&rec), r#"`m x='1' and y="2"`"#);

    rec.message = Some(EncodedString::raw("'m' `1`").into());
    assert_eq!(formatter.format_to_string(&rec), r#""'m' `1`""#);

    rec.message = Some(EncodedString::raw("").into());
    assert_eq!(formatter.format_to_string(&rec), r#""#);

    rec.ts = Some(Timestamp::new("2000-01-02T03:04:05.123Z"));
    assert_eq!(formatter.format_to_string(&rec), r#"00-01-02 03:04:05.123"#);
}

#[test]
fn test_always_quoted_message() {
    let formatter = formatter()
        .with_theme(Default::default())
        .with_message_format(new_message_format(MessageFormat::AlwaysQuoted, ""))
        .build();

    let mut rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
        ..Default::default()
    };
    assert_eq!(formatter.format_to_string(&rec), r#""m" a=42"#);

    rec.message = Some(EncodedString::raw("m x='1'").into());
    assert_eq!(formatter.format_to_string(&rec), r#""m x='1'" a=42"#);

    rec.message = Some(EncodedString::raw(r#""m" x='1'"#).into());
    assert_eq!(formatter.format_to_string(&rec), r#"`"m" x='1'` a=42"#);

    rec.message = Some(EncodedString::raw(r#"m x="1""#).into());
    assert_eq!(formatter.format_to_string(&rec), r#"'m x="1"' a=42"#);

    rec.message = Some(EncodedString::raw(r#"m `x`="1"|'2'"#).into());
    assert_eq!(formatter.format_to_string(&rec), r#""m `x`=\"1\"|'2'" a=42"#);

    rec.fields = Default::default();
    rec.message = Some(EncodedString::raw("m").into());
    assert_eq!(formatter.format_to_string(&rec), r#""m""#);
}

#[test]
fn test_always_double_quoted_message() {
    let formatter = formatter()
        .with_theme(Default::default())
        .with_message_format(new_message_format(MessageFormat::AlwaysDoubleQuoted, ""))
        .build();

    let mut rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
        ..Default::default()
    };
    assert_eq!(formatter.format_to_string(&rec), r#""m" a=42"#);

    rec.fields = Default::default();
    assert_eq!(formatter.format_to_string(&rec), r#""m""#);
}

#[test]
fn test_raw_message() {
    let formatter = formatter()
        .with_theme(Default::default())
        .with_message_format(new_message_format(MessageFormat::Raw, ""))
        .build();

    let mut rec = Record {
        message: Some(EncodedString::raw("m 1").into()),
        fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
        ..Default::default()
    };
    assert_eq!(formatter.format_to_string(&rec), r#"m 1 a=42"#);

    rec.fields = Default::default();
    assert_eq!(formatter.format_to_string(&rec), r#"m 1"#);
}

#[test]
fn test_delimited_message() {
    let formatter = formatter()
        .with_theme(Default::default())
        .with_message_format(new_message_format(MessageFormat::Delimited, "::"))
        .build();

    let mut rec = Record {
        message: Some(EncodedString::raw("'message' 1").into()),
        fields: RecordFields::from_slice(&[("a", RawValue::Number("42"))]),
        ..Default::default()
    };
    assert_eq!(formatter.format_to_string(&rec), r#""'message' 1" :: a=42"#);

    rec.message = Some(EncodedString::raw(r#"`'message' "1"`"#).into());
    assert_eq!(formatter.format_to_string(&rec), r#""`'message' \"1\"`" :: a=42"#);

    rec.fields = Default::default();
    assert_eq!(formatter.format_to_string(&rec), r#""`'message' \"1\"`""#);

    rec.message = Some(EncodedString::raw("'message' 1").into());
    assert_eq!(formatter.format_to_string(&rec), r#""'message' 1""#);

    rec.message = Some(EncodedString::raw(r#""message" 1"#).into());
    assert_eq!(formatter.format_to_string(&rec), r#"'"message" 1'"#);

    rec.message = Some(EncodedString::raw(r#""message" '1'"#).into());
    assert_eq!(formatter.format_to_string(&rec), r#"`"message" '1'`"#);

    rec.message = Some(EncodedString::raw(r#"message\twith\ttabs"#).into());
    assert_eq!(formatter.format_to_string(&rec), r#"message\twith\ttabs"#);
}

#[test]
fn test_ascii_mode() {
    // Use testing samples for record and formatting
    let (rec, formatting) = (Sample::sample(), Formatting::sample());

    // Create formatters with each ASCII mode but no theme (for no-color output)
    let formatter_ascii = RecordFormatterBuilder::new()
        .with_timestamp_formatter(DateTimeFormatter::new(
            LinuxDateFormat::new("%b %d %T.%3N").compile(),
            Tz::FixedOffset(Utc.fix()),
        ))
        .with_options(formatting.clone())
        .with_ascii(AsciiMode::On)
        .build();

    let formatter_utf8 = RecordFormatterBuilder::new()
        .with_timestamp_formatter(DateTimeFormatter::new(
            LinuxDateFormat::new("%b %d %T.%3N").compile(),
            Tz::FixedOffset(Utc.fix()),
        ))
        .with_options(formatting)
        .with_ascii(AsciiMode::Off)
        .build();

    // Get formatted output from both formatters (already without ANSI codes)
    let ascii_result = formatter_ascii.format_to_string(&rec);
    let utf8_result = formatter_utf8.format_to_string(&rec);

    // Verify ASCII mode uses ASCII arrow
    assert!(ascii_result.contains("-> "), "ASCII mode should use ASCII arrow");
    // Also verify that it doesn't contain the Unicode arrow
    assert!(!ascii_result.contains("→ "), "ASCII mode should not use Unicode arrow");

    // The ASCII and Unicode outputs should be different
    assert_ne!(ascii_result, utf8_result);

    // Unicode mode should use Unicode arrow
    assert!(utf8_result.contains("→ "), "Unicode mode should use Unicode arrow");
    // Also verify that it doesn't contain the ASCII arrow
    assert!(!utf8_result.contains("@ "), "Unicode mode should not use ASCII arrow");
}

#[test]
fn test_punctuation_with_ascii_mode() {
    // Use testing samples for formatting
    let formatting = Formatting::sample();

    // Create formatters with different ASCII modes but no theme
    let ascii_formatter = RecordFormatterBuilder::new()
        .with_timestamp_formatter(DateTimeFormatter::new(
            LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
            Tz::FixedOffset(Utc.fix()),
        ))
        .with_options(formatting.clone())
        .with_ascii(AsciiMode::On)
        .build();

    let utf8_formatter = RecordFormatterBuilder::new()
        .with_timestamp_formatter(DateTimeFormatter::new(
            LinuxDateFormat::new("%y-%m-%d %T.%3N").compile(),
            Tz::FixedOffset(Utc.fix()),
        ))
        .with_options(formatting)
        .with_ascii(AsciiMode::Off)
        .build();

    // Use test record with source location for testing source_location_separator
    let rec = Record::sample();

    // Format the record with both formatters
    let ascii_result = ascii_formatter.format_to_string(&rec);
    let utf8_result = utf8_formatter.format_to_string(&rec);

    // ASCII result should contain the ASCII arrow
    assert!(ascii_result.contains("-> "), "ASCII result missing expected arrow");

    // Unicode result should contain the Unicode arrow
    assert!(utf8_result.contains("→ "), "Unicode result missing expected arrow");

    // The outputs should be different
    assert_ne!(ascii_result, utf8_result);
}

#[test]
fn test_hide_empty_fields_nested_flatten() {
    let val = json_raw_value(r#"{"nested":{"empty":"","nonempty":"value"},"top_empty":""}"#);
    let rec = Record::from_fields(&[("data", RawObject::Json(&val).into())]);

    // With hide_empty_fields enabled
    let formatter_hide = RecordFormatterBuilder {
        flatten: true,
        hide_empty_fields: true,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    // With hide_empty_fields disabled
    let formatter_show = RecordFormatterBuilder {
        flatten: true,
        hide_empty_fields: false,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    let result_hide = formatter_hide.format_to_string(&rec);
    let result_show = formatter_show.format_to_string(&rec);

    // When hiding empty fields, should only show non-empty nested field and ellipsis
    assert_eq!(&result_hide, "data.nested.nonempty=value ...");

    // When showing empty fields, should show all fields including empty ones
    assert_eq!(
        &result_show,
        r#"data.nested.empty="" data.nested.nonempty=value data.top-empty="""#
    );
}

#[test]
fn test_hide_empty_fields_nested_no_flatten() {
    let val = json_raw_value(r#"{"nested":{"empty":"","nonempty":"value"},"top_empty":""}"#);
    let rec = Record::from_fields(&[("data", RawObject::Json(&val).into())]);

    // With hide_empty_fields enabled
    let formatter_hide = RecordFormatterBuilder {
        flatten: false,
        hide_empty_fields: true,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    // With hide_empty_fields disabled
    let formatter_show = RecordFormatterBuilder {
        flatten: false,
        hide_empty_fields: false,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    let result_hide = formatter_hide.format_to_string(&rec);
    let result_show = formatter_show.format_to_string(&rec);

    // When hiding empty fields, should only show non-empty nested field and ellipsis
    assert_eq!(&result_hide, "data={ nested={ nonempty=value ... } ... }");

    // When showing empty fields, should show all fields including empty ones
    assert_eq!(
        &result_show,
        r#"data={ nested={ empty="" nonempty=value } top-empty="" }"#
    );
}

#[test]
fn test_hide_empty_fields_no_ellipsis_when_no_empty_fields() {
    let val = json_raw_value(r#"{"nested":{"nonempty1":"value1","nonempty2":"value2"}}"#);
    let rec = Record::from_fields(&[("data", RawObject::Json(&val).into())]);

    // With hide_empty_fields enabled
    let formatter_hide = RecordFormatterBuilder {
        flatten: true,
        hide_empty_fields: true,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    let result_hide = formatter_hide.format_to_string(&rec);

    // When no empty fields exist, should not show ellipsis
    assert_eq!(
        &result_hide,
        "data.nested.nonempty1=value1 data.nested.nonempty2=value2"
    );
}

#[test]
fn test_hide_empty_objects_flatten() {
    let val = json_raw_value(r#"{"empty_obj":{},"all_empty":{"a":"","b":""},"has_value":{"a":"","b":"value"}}"#);
    let rec = Record::from_fields(&[("data", RawObject::Json(&val).into())]);

    // With hide_empty_fields enabled
    let formatter_hide = RecordFormatterBuilder {
        flatten: true,
        hide_empty_fields: true,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    // With hide_empty_fields disabled
    let formatter_show = RecordFormatterBuilder {
        flatten: true,
        hide_empty_fields: false,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    let result_hide = formatter_hide.format_to_string(&rec);
    let result_show = formatter_show.format_to_string(&rec);

    // When hiding empty fields, empty objects and objects with all empty fields should be hidden
    assert_eq!(&result_hide, "data.has-value.b=value ...");

    // When showing empty fields, all objects should show
    assert_eq!(
        &result_show,
        r#"data.all-empty.a="" data.all-empty.b="" data.has-value.a="" data.has-value.b=value"#
    );
}

#[test]
fn test_hide_empty_objects_no_flatten() {
    let val = json_raw_value(r#"{"empty_obj":{},"all_empty":{"a":"","b":""},"has_value":{"a":"","b":"value"}}"#);
    let rec = Record::from_fields(&[("data", RawObject::Json(&val).into())]);

    // With hide_empty_fields enabled
    let formatter_hide = RecordFormatterBuilder {
        flatten: false,
        hide_empty_fields: true,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    // With hide_empty_fields disabled
    let formatter_show = RecordFormatterBuilder {
        flatten: false,
        hide_empty_fields: false,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    let result_hide = formatter_hide.format_to_string(&rec);
    let result_show = formatter_show.format_to_string(&rec);

    // When hiding empty fields, empty objects and objects with all empty fields should be hidden
    assert_eq!(&result_hide, "data={ has-value={ b=value ... } ... }");

    // When showing empty fields, all objects should show
    assert_eq!(
        &result_show,
        r#"data={ empty-obj={} all-empty={ a="" b="" } has-value={ a="" b=value } }"#
    );
}

#[test]
fn test_hide_deeply_nested_empty_objects() {
    let val = json_raw_value(r#"{"deep":{"level1":{"level2":{"empty":""}}}}"#);
    let rec = Record::from_fields(&[("data", RawObject::Json(&val).into())]);

    let formatter_hide = RecordFormatterBuilder {
        flatten: true,
        hide_empty_fields: true,
        theme: Some(Default::default()),
        ..formatter()
    }
    .build();

    let result_hide = formatter_hide.format_to_string(&rec);

    // Deeply nested objects with only empty fields should be completely hidden
    assert_eq!(&result_hide, " ...");
}
