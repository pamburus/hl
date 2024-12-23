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
use itertools::Itertools;
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
        self.format_record(&mut buf, 0..0, rec);
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
            expansion: Default::default(),
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

fn format_no_color_inline(rec: &Record) -> String {
    formatter()
        .with_theme(Default::default())
        .with_expansion(Expansion {
            mode: ExpansionMode::Inline,
            ..Default::default()
        })
        .build()
        .format_to_string(rec)
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
        &format_no_color_inline(&rec),
        r#"k="some \"value\" from 'source' with `sauce`""#
    );
}

#[test]
fn test_string_value_raw_space_and_double_and_single_quotes_and_backticks() {
    let v = r#"some "value" from 'source' with `sauce`"#;
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(
        &format_no_color_inline(&rec),
        r#"k="some \"value\" from 'source' with `sauce`""#
    );
}

#[test]
fn test_string_value_json_tabs() {
    let v = r#""some\tvalue""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color_inline(&rec), "k=`some\tvalue`");
}

#[test]
fn test_string_value_json_tabs_expand() {
    let v = r#""some\tvalue""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(v).into())]);
    assert_eq!(&format_no_color(&rec), "~\n  > k=|=>\n     \tsome\tvalue");
}

#[test]
fn test_string_value_raw_tabs() {
    let v = "some\tvalue";
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color_inline(&rec), "k=`some\tvalue`");
}

#[test]
fn test_string_value_raw_tabs_expand() {
    let v = "some\tvalue";
    let rec = Record::from_fields(&[("k", EncodedString::raw(v).into())]);
    assert_eq!(&format_no_color(&rec), "~\n  > k=|=>\n     \tsome\tvalue");
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
        expansion: Some(ExpansionMode::Never.into()),
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
    formatter.format_record(&mut Buf::default(), 0..0, rec);
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
fn test_string_value_json_extended_space() {
    let v = r#""some\tvalue""#;
    let rec = Record::from_fields(&[("k", EncodedString::json(&v).into())]);
    assert_eq!(
        format_no_color(&rec),
        format!(
            "{mh}\n  > k={vh}\n    {vi}some\tvalue",
            mh = EXPANDED_MESSAGE_HEADER,
            vh = EXPANDED_VALUE_HEADER,
            vi = EXPANDED_VALUE_INDENT,
        )
    );
}

#[test]
fn test_string_value_raw_extended_space() {
    let v = "some\tvalue";
    let rec = Record::from_fields(&[("k", EncodedString::raw(&v).into())]);
    assert_eq!(
        format_no_color(&rec),
        format!(
            "{mh}\n  > k={vh}\n    {vi}some\tvalue",
            mh = EXPANDED_MESSAGE_HEADER,
            vh = EXPANDED_VALUE_HEADER,
            vi = EXPANDED_VALUE_INDENT,
        )
    );
}

#[test]
fn test_expand_with_hidden() {
    let mut fields = IncludeExcludeKeyFilter::default();
    fields.entry("b").exclude();
    fields.entry("c").entry("z").exclude();
    let formatter = RecordFormatterBuilder {
        theme: Default::default(),
        flatten: false,
        expansion: Some(ExpansionMode::Always.into()),
        fields: Some(fields.into()),
        ..formatter()
    }
    .build();

    let obj = json_raw_value(r#"{"x":10,"y":20,"z":30}"#);
    let rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[
            ("a", EncodedString::raw("1").into()),
            ("b", EncodedString::raw("2").into()),
            ("c", RawObject::Json(&obj).into()),
            ("d", EncodedString::raw("4").into()),
        ]),
        ..Default::default()
    };

    let result = formatter.format_to_string(&rec);
    assert_eq!(
        &result,
        "m\n  > a=1\n  > c:\n    > x=10\n    > y=20\n    > ...\n  > d=4\n  > ..."
    );
}

#[test]
fn test_expand_with_hidden_and_flatten() {
    let mut fields = IncludeExcludeKeyFilter::default();
    fields.entry("c").entry("z").exclude();

    let formatter = RecordFormatterBuilder {
        theme: Default::default(),
        flatten: true,
        expansion: Some(ExpansionMode::Always.into()),
        fields: Some(fields.into()),
        ..formatter()
    }
    .build();

    let obj = json_raw_value(r#"{"x":10,"y":20,"z":30}"#);
    let rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[
            ("a", EncodedString::raw("1").into()),
            ("b", EncodedString::raw("2").into()),
            ("c", RawObject::Json(&obj).into()),
            ("d", EncodedString::raw("4").into()),
        ]),
        ..Default::default()
    };

    let result = formatter.format_to_string(&rec);
    assert_eq!(&result, "m\n  > a=1\n  > b=2\n  > c.x=10\n  > c.y=20\n  > d=4\n  > ...");
}

#[test]
fn test_expand_object() {
    let formatter = RecordFormatterBuilder {
        theme: Default::default(),
        flatten: false,
        expansion: Some(ExpansionMode::default().into()),
        ..formatter()
    }
    .build();

    let obj = json_raw_value(r#"{"x":10,"y":"some\nmultiline\nvalue","z":30}"#);
    let rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[
            ("a", EncodedString::raw("1").into()),
            ("b", EncodedString::raw("2").into()),
            ("c", RawObject::Json(&obj).into()),
            ("d", EncodedString::raw("4").into()),
        ]),
        ..Default::default()
    };

    let result = formatter.format_to_string(&rec);
    assert_eq!(
        &result,
        "m a=1 b=2 d=4\n  > c:\n    > x=10\n    > y=|=>\n       \tsome\n       \tmultiline\n       \tvalue\n    > z=30"
    );
}

#[test]
fn test_expand_global_threshold() {
    let mut expansion = Expansion::from(ExpansionMode::High);
    expansion.profiles.high.thresholds.global = 2;

    let formatter = RecordFormatterBuilder {
        theme: Default::default(),
        expansion: Some(expansion),
        ..formatter()
    }
    .build();

    let rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[
            ("a", EncodedString::raw("1").into()),
            ("b", EncodedString::raw("2").into()),
            ("c", EncodedString::raw("3").into()),
        ]),
        ..Default::default()
    };

    let result = formatter.format_to_string(&rec);
    assert_eq!(&result, "m\n  > a=1\n  > b=2\n  > c=3", "{}", result);
}

#[test]
fn test_caller_file_line() {
    let format = |file, line| {
        let rec = Record {
            message: Some(EncodedString::raw("m").into()),
            caller: Caller { file, line, name: "" },
            ..Default::default()
        };

        format_no_color(&rec)
    };

    assert_eq!(format("f", "42"), r#"m -> f:42"#);
    assert_eq!(format("f", ""), r#"m -> f"#);
    assert_eq!(format("", "42"), r#"m -> :42"#);
}

#[test]
fn test_expand_no_filter() {
    let rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_slice(&[
            ("a", EncodedString::raw("1").into()),
            ("b", EncodedString::raw("2").into()),
            ("c", EncodedString::raw("3").into()),
        ]),
        ..Default::default()
    };

    let formatter = RecordFormatterBuilder {
        theme: Default::default(),
        expansion: Some(ExpansionMode::default().into()),
        ..formatter()
    }
    .build();

    assert_eq!(formatter.format_to_string(&rec), r#"m a=1 b=2 c=3"#);
}

#[test]
fn test_expand_message() {
    let rec = |m, f| Record {
        message: Some(EncodedString::raw(m).into()),
        fields: RecordFields::from_slice(&[("a", EncodedString::raw(f).into())]),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::Medium);
    expansion.profiles.medium.thresholds.message = 64;

    let default_theme = formatter().theme;

    let mut formatter = RecordFormatterBuilder {
        theme: Default::default(),
        expansion: Some(expansion),
        ..formatter()
    }
    .build();

    let lorem_ipsum = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";

    assert_eq!(
        formatter.format_to_string(&rec(lorem_ipsum, "1")),
        format!("a=1\n  > msg=\"{}\"", lorem_ipsum)
    );
    assert_eq!(
        formatter.format_to_string(&rec("", "some\nmultiline\ntext")),
        format!(
            concat!(
                "{mh}\n",
                "  > a={header}\n",
                "    {indent}some\n",
                "    {indent}multiline\n",
                "    {indent}text"
            ),
            mh = EXPANDED_MESSAGE_HEADER,
            header = EXPANDED_VALUE_HEADER,
            indent = EXPANDED_VALUE_INDENT
        )
    );

    assert_eq!(
        formatter.format_to_string(&rec("some\nmultiline\ntext", "1")),
        format!(
            concat!(
                "a=1\n",
                "  > msg={vh}\n",
                "    {vi}some\n",
                "    {vi}multiline\n",
                "    {vi}text",
            ),
            vh = EXPANDED_VALUE_HEADER,
            vi = EXPANDED_VALUE_INDENT,
        )
    );

    formatter.theme = default_theme.unwrap_or_default();

    assert_eq!(
        formatter.format_to_string(&rec("some\nmultiline\ntext", "1")),
        format!(
            concat!(
                "\u{1b}[0;32ma\u{1b}[0;2m=\u{1b}[0;94m1\u{1b}[0;32m\u{1b}[0m\n",
                "  \u{1b}[0;2m> \u{1b}[0;32mmsg\u{1b}[0;2m=\u{1b}[0;39m\u{1b}[0;2m{vh}\u{1b}[0m\n",
                "  \u{1b}[0;2m  {vi}\u{1b}[0msome\n",
                "  \u{1b}[0;2m  {vi}\u{1b}[0mmultiline\n",
                "  \u{1b}[0;2m  {vi}\u{1b}[0mtext\u{1b}[0m",
            ),
            vh = EXPANDED_VALUE_HEADER,
            vi = EXPANDED_VALUE_INDENT,
        )
    );
}

#[test]
fn test_expand_without_message() {
    let rec = |f, ts| Record {
        ts,
        fields: RecordFields::from_slice(&[("a", EncodedString::raw(f).into())]),
        ..Default::default()
    };

    let ts = Timestamp::new("2000-01-02T03:04:05.123Z");

    let formatter = RecordFormatterBuilder {
        theme: Default::default(),
        expansion: Some(ExpansionMode::Always.into()),
        ..formatter()
    }
    .build();

    assert_eq!(
        formatter.format_to_string(&rec("1", None)),
        format!("{mh}\n  > a=1", mh = EXPANDED_MESSAGE_HEADER)
    );
    assert_eq!(
        formatter.format_to_string(&rec("1", Some(ts))),
        format!(
            concat!("00-01-02 03:04:05.123 {mh}\n", "                        > a=1"),
            mh = EXPANDED_MESSAGE_HEADER
        )
    );
}

#[test]
fn test_format_uuid() {
    let rec = |value| Record {
        fields: RecordFields::from_slice(&[("a", EncodedString::raw(value).into())]),
        ..Default::default()
    };

    assert_eq!(
        format_no_color(&rec("243e020d-11d6-42f6-b4cd-b4586057b9a2")),
        "a=243e020d-11d6-42f6-b4cd-b4586057b9a2"
    );
}

#[test]
fn test_format_int_string() {
    let rec = |value| Record {
        fields: RecordFields::from_slice(&[("a", EncodedString::json(value).into())]),
        ..Default::default()
    };

    assert_eq!(format_no_color(&rec(r#""243""#)), r#"a="243""#);
}

#[test]
fn test_format_unparsable_time() {
    let rec = |ts, msg| Record {
        ts: Some(Timestamp::new(ts)),
        level: Some(Level::Info),
        message: Some(EncodedString::raw(msg).into()),
        ..Default::default()
    };

    assert_eq!(
        format_no_color(&rec("some-unparsable-time", "some-msg")),
        "|INF| some-msg ts=some-unparsable-time"
    );
}

#[test]
fn test_format_value_with_eq() {
    let rec = |value| Record {
        fields: RecordFields::from_slice(&[("a", EncodedString::raw(value).into())]),
        ..Default::default()
    };

    assert_eq!(format_no_color(&rec("x=y")), r#"a="x=y""#);
    assert_eq!(format_no_color(&rec("|=>")), r#"a="|=>""#);
}

#[test]
fn test_value_format_auto() {
    let vf = string::ValueFormatAuto::default();
    let mut buf = Vec::new();
    let result = vf
        .format(EncodedString::raw("test"), &mut buf, ExtendedSpaceAction::Inline)
        .unwrap();
    assert_eq!(buf, b"test");
    assert_eq!(result.is_ok(), true);
}

#[test]
fn test_message_format_auto_empty() {
    let vf = string::MessageFormatAutoQuoted;
    let mut buf = Vec::new();
    let result = vf
        .format(EncodedString::raw(""), &mut buf, ExtendedSpaceAction::Abort)
        .unwrap();
    assert_eq!(buf, br#""""#);
    assert_eq!(result.is_ok(), true);
}

#[test]
fn test_expand_mode_inline() {
    let rec = |value| Record {
        fields: RecordFields::from_slice(&[("a", EncodedString::raw(value).into())]),
        ..Default::default()
    };

    let formatter = formatter()
        .with_theme(Default::default())
        .with_expansion(ExpansionMode::Inline.into())
        .build();

    assert_eq!(
        formatter.format_to_string(&rec("some single-line message")),
        r#"a="some single-line message""#
    );
    assert_eq!(
        formatter.format_to_string(&rec("some\nmultiline\nmessage")),
        "a=`some\nmultiline\nmessage`"
    );
}

#[test]
fn test_expand_mode_low() {
    let rec = |value| Record {
        fields: RecordFields::from_slice(&[("a", EncodedString::raw(value).into())]),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::Low);
    expansion.profiles.low.thresholds.global = 1024;
    expansion.profiles.low.thresholds.cumulative = 1024;
    expansion.profiles.low.thresholds.field = 1024;
    expansion.profiles.low.thresholds.message = 1024;

    let formatter = formatter()
        .with_theme(Default::default())
        .with_expansion(expansion)
        .build();

    assert_eq!(
        formatter.format_to_string(&rec("some single-line message")),
        r#"a="some single-line message""#
    );
    assert_eq!(
        formatter.format_to_string(&rec("some\nmultiline\nmessage")),
        "~\n  > a=|=>\n     \tsome\n     \tmultiline\n     \tmessage"
    );
}

#[test]
fn test_expansion_threshold_cumulative() {
    let rec = |msg, v1, v2, v3| Record {
        message: Some(EncodedString::raw(msg).into()),
        fields: RecordFields::from_slice(&[
            ("a", EncodedString::raw(v1).into()),
            ("b", EncodedString::raw(v2).into()),
            ("c", EncodedString::raw(v3).into()),
        ]),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::High);
    expansion.profiles.high.thresholds.global = 1024;
    expansion.profiles.high.thresholds.cumulative = 32;
    expansion.profiles.high.thresholds.field = 1024;
    expansion.profiles.high.thresholds.message = 1024;

    let formatter = formatter()
        .with_theme(Default::default())
        .with_expansion(expansion)
        .build();

    assert_eq!(
        formatter.format_to_string(&rec("", "v1", "v2", "v3")),
        r#"a=v1 b=v2 c=v3"#
    );
    assert_eq!(
        formatter.format_to_string(&rec("m", "v1", "v2", "v3")),
        r#"m a=v1 b=v2 c=v3"#
    );
    assert_eq!(
        formatter.format_to_string(&rec("", "long-v1", "long-v2", "long-v3")),
        "a=long-v1 b=long-v2 c=long-v3"
    );
    assert_eq!(
        formatter.format_to_string(&rec("m", "long-v1", "long-v2", "long-v3")),
        "m a=long-v1 b=long-v2\n  > c=long-v3"
    );
    assert_eq!(
        formatter.format_to_string(&rec(
            "some long long long long long long message",
            "long-v1",
            "long-v2",
            "long-v3"
        )),
        "some long long long long long long message\n  > a=long-v1\n  > b=long-v2\n  > c=long-v3"
    );
}

#[test]
fn test_expansion_threshold_global() {
    let rec = |msg, v1, v2, v3| Record {
        message: Some(EncodedString::raw(msg).into()),
        fields: RecordFields::from_slice(&[
            ("a", EncodedString::raw(v1).into()),
            ("b", EncodedString::raw(v2).into()),
            ("c", EncodedString::raw(v3).into()),
        ]),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::High);
    expansion.profiles.high.thresholds.global = 28;
    expansion.profiles.high.thresholds.cumulative = 1024;
    expansion.profiles.high.thresholds.field = 1024;
    expansion.profiles.high.thresholds.message = 1024;

    let formatter = formatter()
        .with_theme(Default::default())
        .with_expansion(expansion)
        .build();

    assert_eq!(
        formatter.format_to_string(&rec("", "v1", "v2", "v3")),
        r#"a=v1 b=v2 c=v3"#
    );
    assert_eq!(
        formatter.format_to_string(&rec("m", "v1", "v2", "v3")),
        r#"m a=v1 b=v2 c=v3"#
    );
    assert_eq!(
        formatter.format_to_string(&rec("", "long-v1", "long-v2", "long-v3")),
        "~\n  > a=long-v1\n  > b=long-v2\n  > c=long-v3"
    );
    assert_eq!(
        formatter.format_to_string(&rec("m", "long-v1", "long-v2", "long-v3")),
        "m\n  > a=long-v1\n  > b=long-v2\n  > c=long-v3"
    );
    assert_eq!(
        formatter.format_to_string(&rec(
            "some long long long long long long message",
            "long-v1",
            "long-v2",
            "long-v3"
        )),
        "some long long long long long long message\n  > a=long-v1\n  > b=long-v2\n  > c=long-v3"
    );
}

#[test]
fn test_expansion_threshold_field() {
    let rec = |value| Record {
        fields: RecordFields::from_slice(&[("a", value)]),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::High);
    expansion.profiles.high.thresholds.global = 1024;
    expansion.profiles.high.thresholds.cumulative = 1024;
    expansion.profiles.high.thresholds.field = 48;
    expansion.profiles.high.thresholds.message = 1024;

    let formatter = formatter()
        .with_theme(Default::default())
        .with_expansion(expansion)
        .with_flatten(false)
        .build();

    let array = json_raw_value("[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]");
    let object = json_raw_value(r#"{"a":"v1","b":"v2","c":"v3","d":"v4","e":"v5","f":"v6"}"#);

    assert_eq!(
        formatter.format_to_string(&rec(EncodedString::raw("v").into())),
        r#"a=v"#
    );
    assert_eq!(
        formatter.format_to_string(&rec(RawValue::Array(array.as_ref().into()))),
        "~\n  > a=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]"
    );
    assert_eq!(
        formatter.format_to_string(&rec(RawValue::Object(object.as_ref().into()))),
        "~\n  > a:\n    > a=v1\n    > b=v2\n    > c=v3\n    > d=v4\n    > e=v5\n    > f=v6"
    );
}

#[test]
fn test_expansion_nested_field() {
    let rec = |value| Record {
        fields: RecordFields::from_slice(&[("a", value)]),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::High);
    expansion.profiles.high.thresholds.global = 1024;
    expansion.profiles.high.thresholds.cumulative = 1024;
    expansion.profiles.high.thresholds.field = 1024;
    expansion.profiles.high.thresholds.message = 1024;

    let formatter = formatter()
        .with_theme(Default::default())
        .with_expansion(expansion)
        .with_empty_fields_hiding(true)
        .with_flatten(false)
        .build();

    let object =
        json_raw_value(r#"{"a":"v1","b":"v2","c":{"c":"v3","d":"v4\nwith second line","e":"v5","f":"v6","g":""}}"#);

    assert_eq!(
        formatter.format_to_string(&rec(RawValue::Object(object.as_ref().into()))),
        "~\n  > a:\n    > a=v1\n    > b=v2\n    > c:\n      > c=v3\n      > d=|=>\n         \tv4\n         \twith second line\n      > e=v5\n      > f=v6"
    );
}

#[test]
fn test_add_field_to_expand() {
    const M: usize = MAX_FIELDS_TO_EXPAND_ON_HOLD + 2;
    let kvs = (0..M)
        .map(|i| (format!("k{}", i).to_owned(), format!("some\nvalue #{}", i).to_owned()))
        .collect_vec();
    let rec = Record {
        message: Some(EncodedString::raw("m").into()),
        fields: RecordFields::from_iter(
            kvs.iter()
                .map(|(k, v)| (k.as_str(), EncodedString::raw(v.as_str()).into())),
        ),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::Medium);
    expansion.profiles.medium.thresholds.global = 1024;
    expansion.profiles.medium.thresholds.cumulative = 320;
    expansion.profiles.medium.thresholds.field = 1024;
    expansion.profiles.medium.thresholds.message = 1024;

    let formatter = formatter()
        .with_theme(Default::default())
        .with_expansion(expansion)
        .build();

    assert_eq!(
        formatter.format_to_string(&rec),
        "m\n  > k0=|=>\n     \tsome\n     \tvalue #0\n  > k1=|=>\n     \tsome\n     \tvalue #1\n  > k2=|=>\n     \tsome\n     \tvalue #2\n  > k3=|=>\n     \tsome\n     \tvalue #3\n  > k4=|=>\n     \tsome\n     \tvalue #4\n  > k5=|=>\n     \tsome\n     \tvalue #5\n  > k6=|=>\n     \tsome\n     \tvalue #6\n  > k7=|=>\n     \tsome\n     \tvalue #7\n  > k8=|=>\n     \tsome\n     \tvalue #8\n  > k9=|=>\n     \tsome\n     \tvalue #9\n  > k10=|=>\n     \tsome\n     \tvalue #10\n  > k11=|=>\n     \tsome\n     \tvalue #11\n  > k12=|=>\n     \tsome\n     \tvalue #12\n  > k13=|=>\n     \tsome\n     \tvalue #13\n  > k14=|=>\n     \tsome\n     \tvalue #14\n  > k15=|=>\n     \tsome\n     \tvalue #15\n  > k16=|=>\n     \tsome\n     \tvalue #16\n  > k17=|=>\n     \tsome\n     \tvalue #17\n  > k18=|=>\n     \tsome\n     \tvalue #18\n  > k19=|=>\n     \tsome\n     \tvalue #19\n  > k20=|=>\n     \tsome\n     \tvalue #20\n  > k21=|=>\n     \tsome\n     \tvalue #21\n  > k22=|=>\n     \tsome\n     \tvalue #22\n  > k23=|=>\n     \tsome\n     \tvalue #23\n  > k24=|=>\n     \tsome\n     \tvalue #24\n  > k25=|=>\n     \tsome\n     \tvalue #25\n  > k26=|=>\n     \tsome\n     \tvalue #26\n  > k27=|=>\n     \tsome\n     \tvalue #27\n  > k28=|=>\n     \tsome\n     \tvalue #28\n  > k29=|=>\n     \tsome\n     \tvalue #29\n  > k30=|=>\n     \tsome\n     \tvalue #30\n  > k31=|=>\n     \tsome\n     \tvalue #31\n  > k32=|=>\n     \tsome\n     \tvalue #32\n  > k33=|=>\n     \tsome\n     \tvalue #33"
    );
}

#[test]
fn test_complex_message_expansion() {
    let rec = Record {
        message: Some(EncodedString::json(r#""<Settings source=\"X\" type=\"Y\" version=\"1\">\n</Settings>""#).into()),
        fields: RecordFields::from_slice(&[
            ("level", EncodedString::raw("info").into()),
            ("ts", EncodedString::raw("2024-06-05T04:25:29Z").into()),
        ]),
        ..Default::default()
    };

    let mut expansion = Expansion::from(ExpansionMode::High);
    expansion.profiles.high.thresholds.cumulative = 32;
    expansion.profiles.high.thresholds.field = 1024;
    expansion.profiles.high.thresholds.global = 10;
    expansion.profiles.high.thresholds.message = 1024;

    let formatter = RecordFormatterBuilder {
        theme: Default::default(),
        flatten: true,
        expansion: Some(expansion),
        ..formatter()
    }
    .build();

    let result = formatter.format_to_string(&rec);

    assert_eq!(
        &result,
        "~\n  > msg=|=>\n     \t<Settings source=\"X\" type=\"Y\" version=\"1\">\n     \t</Settings>\n  > level=info\n  > ts=2024-06-05T04:25:29Z"
    );
}
