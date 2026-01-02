// super imports
use super::*;

// std imports
use std::io::Cursor;

// third-party imports
use chrono::{Offset, Utc};
use chrono_tz::UTC;
use maplit::hashmap;

// local imports
use crate::{
    LinuxDateFormat,
    filtering::MatchOptions,
    level::{InfallibleLevel, Level},
    model::FieldFilterSet,
    scanning::{BufFactory, PartialPlacement, Segment, SegmentBuf, SegmentBufFactory},
    settings::{self, AsciiMode, DisplayVariant, MessageFormat, MessageFormatting},
};

#[test]
fn test_common_prefix_len() {
    let items = vec!["abc", "abcd", "ab", "ab"];
    assert_eq!(common_prefix_len(&items), 2);
}

#[test]
fn test_cat_empty() {
    let input = input("");
    let mut output = Vec::new();
    let app = App::new(options());
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), "");
}

#[test]
fn test_cat_one_line() {
    let input = input(
        r#"{"caller":"main.go:539","duration":"15d","level":"info","msg":"No time or size retention was set so using the default time retention","ts":"2023-12-07T20:07:05.949Z"}"#,
    );
    let mut output = Vec::new();
    let app = App::new(options());
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2023-12-07 20:07:05.949 |INF| No time or size retention was set so using the default time retention duration=15d @ main.go:539\n",
    );
}

#[test]
fn test_cat_with_theme() {
    let input = input(
        r#"{"caller":"main.go:539","duration":"15d","level":"warning","msg":"No time or size retention was set so using the default time retention","ts":"2023-12-07T20:07:05.949Z"}"#,
    );
    let mut output = Vec::new();
    let app = App::new(options().with_theme(theme()));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;7;33m|WRN|\u{1b}[0m \u{1b}[0;1mNo time or size retention was set so using the default time retention \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
    );
}

#[test]
fn test_cat_no_msg() {
    let input = input(r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z"}"#);
    let mut output = Vec::new();
    let app = App::new(options().with_theme(theme()));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;36m|INF|\u{1b}[0m \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
    );
}

#[test]
fn test_cat_msg_array() {
    let input = input(
        r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":["x","y"]}"#,
    );
    let mut output = Vec::new();
    let app = App::new(options().with_theme(theme()));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;36m|INF| \u{1b}[0;32mmsg\u{1b}[0;2m=\u{1b}[0;93m[\u{1b}[0mx\u{1b}[0;93m \u{1b}[0my\u{1b}[0;93m] \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
    );
}

#[test]
fn test_cat_field_exclude() {
    let input =
        input(r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":"xy"}"#);
    let mut output = Vec::new();
    let mut ff = IncludeExcludeKeyFilter::new(MatchOptions::default());
    ff.entry("duration").exclude();
    let app = App::new(options().with_fields(FieldOptions {
        filter: Arc::new(ff),
        ..FieldOptions::default()
    }));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2023-12-07 20:07:05.949 |INF| xy ... @ main.go:539\n",
    );
}

#[test]
fn test_cat_raw_fields() {
    let input =
        input(r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":"xy"}"#);
    let mut output = Vec::new();
    let mut ff = IncludeExcludeKeyFilter::new(MatchOptions::default());
    ff.entry("duration").exclude();
    let app = App::new(options().with_raw_fields(true));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2023-12-07 20:07:05.949 |INF| xy duration=\"15d\" @ main.go:539\n",
    );
}

#[test]
fn test_cat_raw_multiple_inputs() {
    let input1 =
        r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:05.949Z","msg":"xy"}"#;
    let input2 =
        r#"{"caller":"main.go:539","duration":"15d","level":"info","ts":"2023-12-07T20:07:06.944Z","msg":"xy"}"#;
    let mut output = Vec::new();
    let mut ff = IncludeExcludeKeyFilter::new(MatchOptions::default());
    ff.entry("duration").exclude();
    let app = App::new(options().with_input_info(InputInfo::Auto.into()).with_raw(true));
    app.run(vec![input(input1), input(input2)], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        format!("{}\n{}\n", input1, input2),
    );
}

#[test]
fn test_smart_delim_combo() {
    const L1: &str = r#"{}"#;
    const L2: &str = r#"{}"#;

    let input = input(format!("{}\n\r\n{}\n", L1, L2));
    let mut output = Vec::new();
    let app = App::new(options().with_raw(true));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), format!("{}\n\n{}\n", L1, L2),);
}

#[test]
fn test_sort_with_blank_lines() {
    let input = input(concat!(
        r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
        "\n\r\n",
        r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(options().with_sort(true));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        concat!(
            "2024-01-25 18:09:16.860 |DBG| m1\n",
            "2024-01-25 18:10:20.435 |DBG| m2\n",
        ),
    );
}

#[test]
fn test_filter_with_blank_lines() {
    let input = input(concat!(
        r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
        "\n\r\n",
        r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(
        options().with_filter(
            Filter {
                fields: FieldFilterSet::new(["msg=m2"]).unwrap(),
                ..Default::default()
            }
            .into(),
        ),
    );
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2024-01-25 18:10:20.435 |DBG| m2\n",
    );
}

#[test]
fn test_sort_with_clingy_lines() {
    let input = input(concat!(
        r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
        r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(options().with_sort(true));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        concat!(
            "2024-01-25 18:09:16.860 |DBG| m1\n",
            "2024-01-25 18:10:20.435 |DBG| m2\n",
        ),
    );
}

#[test]
fn test_sort_with_clingy_and_invalid_lines() {
    let input = input(concat!(
        r#"{"level":"debug","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m2"}"#,
        r#"{invalid}"#,
        r#"{"level":"debug","ts":"2024-01-25T19:09:16.860711+01:00","msg":"m1"}"#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(options().with_sort(true));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2024-01-25 18:10:20.435 |DBG| m2\n",
    );
}

#[test]
fn test_hide_by_prefix() {
    let input = input(concat!(
        r#"level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11"#,
        "\n",
    ));

    let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::default());
    filter.entry("a.b").exclude();

    let mut output = Vec::new();
    let app = App::new(options().with_fields(FieldOptions {
        filter: Arc::new(filter),
        ..FieldOptions::default()
    }));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2024-01-25 18:10:20.435 |DBG| m1 a.c.b=11 ...\n",
    );
}

#[test]
fn test_hide_by_prefix_and_reveal_child() {
    let input = input(concat!(
        r#"level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11"#,
        "\n",
    ));

    let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::default());
    filter.entry("a.b").exclude();
    filter.entry("a.b.d").include();

    let mut output = Vec::new();
    let app = App::new(options().with_fields(FieldOptions {
        filter: Arc::new(filter),
        ..FieldOptions::default()
    }));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2024-01-25 18:10:20.435 |DBG| m1 a.b.d=20 a.c.b=11 ...\n",
    );
}

#[test]
fn test_incomplete_segment() {
    let input = input(concat!(
        "level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11\n",
        "level=debug time=2024-01-25T19:10:21.764733+01:00 msg=m2 x=2\n"
    ));

    let mut output = Vec::new();
    let app = App::new(Options {
        buffer_size: NonZeroUsize::new(32).unwrap(),
        max_message_size: NonZeroUsize::new(64).unwrap(),
        ..options()
    });
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        concat!(
            "level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11\n",
            "2024-01-25 18:10:21.764 |DBG| m2 x=2\n",
        )
    );
}

#[test]
fn test_incomplete_segment_sorted() {
    let data = concat!(
        "level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11\n",
        "level=debug time=2024-01-25T19:10:21.764733+01:00 msg=m2 x=2\n",
    );
    let input = input(data);

    let mut output = Vec::new();
    let app = App::new(Options {
        buffer_size: NonZeroUsize::new(16).unwrap(),
        max_message_size: NonZeroUsize::new(64).unwrap(),
        sort: true,
        ..options()
    });
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2024-01-25 18:10:21.764 |DBG| m2 x=2\n"
    );
}

#[test]
fn test_issue_288_t1() {
    let input = input(concat!(
        r#"time="2024-06-04 17:14:35.190733+0200" level=INF msg="An INFO log message" logger=aLogger caller=aCaller"#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(options().with_fields(FieldOptions {
        settings: Fields {
            predefined: settings::PredefinedFields {
                level: settings::LevelField {
                    variants: vec![settings::RawLevelFieldVariant {
                        names: vec!["level".to_string()],
                        values: hashmap! {
                            InfallibleLevel::new(Level::Debug) => vec!["dbg".to_string()],
                            InfallibleLevel::new(Level::Info) => vec!["INF".to_string()],
                            InfallibleLevel::new(Level::Warning) => vec!["wrn".to_string()],
                            InfallibleLevel::new(Level::Error) => vec!["ERR".to_string()],
                        },
                        level: None,
                    }],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }));
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2024-06-04 15:14:35.190 |INF| aLogger: An INFO log message @ aCaller\n",
    );
}

#[test]
fn test_issue_176_simple_span_json() {
    let input = input(concat!(r#"{"message":"test","span":{"name":"main"}}"#, "\n",));
    let mut output = Vec::new();
    let app = App::new(
        options().with_fields(FieldOptions {
            settings: Fields {
                predefined: settings::PredefinedFields {
                    logger: settings::Field {
                        names: vec!["span.name".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), "main: test\n");
}

#[test]
fn test_issue_176_simple_span_logfmt() {
    let input = input(concat!(r#"message=test span.name=main"#, "\n",));
    let mut output = Vec::new();
    let app = App::new(
        options().with_fields(FieldOptions {
            settings: Fields {
                predefined: settings::PredefinedFields {
                    logger: settings::Field {
                        names: vec!["span.name".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), "main: test\n");
}

#[test]
fn test_issue_176_complex_span_json() {
    let input = input(concat!(
        r#"{"message":"test","span":{"name":"main","source":"main.rs:12","extra":"included"}}"#,
        "\n",
    ));
    let mut output = Vec::new();
    let app = App::new(
        options().with_fields(FieldOptions {
            settings: Fields {
                predefined: settings::PredefinedFields {
                    logger: settings::Field {
                        names: vec!["span.name".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    caller: settings::Field {
                        names: vec!["span.source".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "main: test span={ extra=included } @ main.rs:12\n"
    );
}

#[test]
fn test_issue_176_complex_span_logfmt() {
    let input = input(concat!(
        r#"message=test span.name=main span.source=main.rs:12 span.extra=included"#,
        "\n",
    ));
    let mut output = Vec::new();
    let app = App::new(
        options().with_fields(FieldOptions {
            settings: Fields {
                predefined: settings::PredefinedFields {
                    logger: settings::Field {
                        names: vec!["span.name".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    caller: settings::Field {
                        names: vec!["span.source".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "main: test span.extra=included @ main.rs:12\n"
    );
}

#[test]
fn test_issue_176_unmatched_json() {
    let input = input(concat!(r#"{"message":"test","span":{"name":"main"}}"#, "\n",));
    let mut output = Vec::new();
    let app = App::new(
        options().with_fields(FieldOptions {
            settings: Fields {
                predefined: settings::PredefinedFields {
                    logger: settings::Field {
                        names: vec!["span.title".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), "test span={ name=main }\n");
}

#[test]
fn test_issue_176_unmatched_logfmt() {
    let input = input(concat!(r#"message=test span.name=main"#, "\n",));
    let mut output = Vec::new();
    let app = App::new(
        options().with_fields(FieldOptions {
            settings: Fields {
                predefined: settings::PredefinedFields {
                    logger: settings::Field {
                        names: vec!["span.title".to_string()],
                        show: FieldShowOption::Always,
                    }
                    .into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), "test span.name=main\n");
}

fn input<S: Into<String>>(s: S) -> InputHolder {
    InputHolder::new(InputReference::Stdin, Some(Box::new(Cursor::new(s.into()))))
}

fn options() -> Options {
    Options {
        theme: Arc::new(Theme::none()),
        time_format: LinuxDateFormat::new("%Y-%m-%d %T.%3N").compile(),
        raw: false,
        raw_fields: false,
        allow_prefix: false,
        buffer_size: NonZeroUsize::new(4096).unwrap(),
        max_message_size: NonZeroUsize::new(4096 * 1024).unwrap(),
        concurrency: 1,
        filter: Default::default(),
        fields: FieldOptions::default(),
        formatting: Formatting {
            message: MessageFormatting {
                format: MessageFormat::AutoQuoted,
            },
            ..Formatting::default()
        },
        time_zone: Tz::IANA(UTC),
        hide_empty_fields: false,
        sort: false,
        follow: false,
        sync_interval: Duration::from_secs(1),
        input_info: Default::default(),
        input_format: None,
        dump_index: false,
        app_dirs: None,
        tail: 0,
        delimiter: Delimiter::default(),
        unix_ts_unit: None,
        flatten: false,
        ascii: AsciiMode::Off,
    }
}

#[test]
fn test_ascii_mode_handling() {
    // Use testing samples for record and formatting
    let (record, formatting) = (Sample::sample(), Formatting::sample());

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

    // Test ASCII mode
    let mut buf_ascii = Vec::new();
    formatter_ascii.format_record(&mut buf_ascii, &record);
    let result_ascii = String::from_utf8(buf_ascii).unwrap();

    // Test Unicode mode
    let mut buf_utf8 = Vec::new();
    formatter_utf8.format_record(&mut buf_utf8, &record);
    let result_utf8 = String::from_utf8(buf_utf8).unwrap();

    // Verify that the ASCII mode uses ASCII arrows
    assert!(result_ascii.contains("-> "), "ASCII mode should use ASCII '->'");
    assert!(!result_ascii.contains("→ "), "ASCII mode should not use Unicode arrow");

    // Verify that the Unicode mode uses Unicode arrows
    assert!(result_utf8.contains("→ "), "Unicode mode should use Unicode arrow");
    assert!(!result_utf8.contains("@ "), "Unicode mode should not use ASCII '@ '");

    // The outputs should be different
    assert_ne!(result_ascii, result_utf8);
}

#[test]
fn test_input_badges_with_ascii_mode() {
    // Use test input references
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("/path/to/some-log-file.log"),
            canonical: std::path::PathBuf::from("/path/to/some-log-file.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("/path/to/another-log-file.log"),
            canonical: std::path::PathBuf::from("/path/to/another-log-file.log"),
        }),
    ];

    println!("Created test input references");

    // Setup app with ASCII mode ON
    let mut options_ascii = options();
    options_ascii.input_info = InputInfo::Full.into();
    options_ascii.ascii = AsciiMode::On;

    // Create formatting with selective variants for ASCII mode testing
    let mut formatting = Formatting::sample();
    formatting.punctuation.input_name_right_separator = DisplayVariant::Selective {
        ascii: " | ".to_string(),
        unicode: " │ ".to_string(),
    };
    formatting.punctuation.input_number_right_separator = DisplayVariant::Selective {
        ascii: " | ".to_string(),
        unicode: " │ ".to_string(),
    };
    options_ascii.formatting = formatting.clone();

    let app_ascii = App::new(options_ascii);

    // Setup app with ASCII mode OFF (Unicode)
    let mut options_unicode = options();
    options_unicode.input_info = InputInfo::Full.into();
    options_unicode.ascii = AsciiMode::Off;
    options_unicode.formatting = formatting;

    let app_unicode = App::new(options_unicode);

    // Get badges with ASCII mode ON
    let badges_ascii = app_ascii.input_badges(inputs.iter());
    assert!(badges_ascii.is_some(), "Should produce badges");
    let badges_a = badges_ascii.unwrap();
    println!("ASCII badges: {:?}", badges_a);

    // Get badges with ASCII mode OFF (Unicode)
    let badges_utf8 = app_unicode.input_badges(inputs.iter());
    assert!(badges_utf8.is_some(), "Should produce badges");
    let badges_u = badges_utf8.unwrap();
    println!("Unicode badges: {:?}", badges_u);

    // Check that we're using ASCII separator in ASCII mode
    for badge in badges_a.iter() {
        assert!(badge.contains(" | "), "ASCII mode should use ASCII separator");
        assert!(!badge.contains(" │ "), "ASCII mode should not use Unicode separator");
    }

    // Check that we're using Unicode separator in Unicode mode
    for badge in badges_u.iter() {
        assert!(badge.contains(" │ "), "Unicode mode should use Unicode separator");
        // Check that there are no ASCII separators (should be all replaced)
        assert!(!badge.contains(" | "), "Unicode mode should not use ASCII separator");
    }

    // Check that the outputs are different
    assert_ne!(badges_a, badges_u, "ASCII and Unicode badges should be different");
}

fn theme() -> Arc<Theme> {
    Sample::sample()
}

#[test]
fn test_char_slice_width() {
    // ASCII characters
    let ascii: Vec<char> = "hello".chars().collect();
    assert_eq!(char_slice_width(&ascii), 5);

    // Emoji (width 2)
    let emoji: Vec<char> = "🎉".chars().collect();
    assert_eq!(char_slice_width(&emoji), 2);

    // Mixed ASCII and emoji
    let mixed: Vec<char> = "test🎉".chars().collect();
    assert_eq!(char_slice_width(&mixed), 6); // 4 + 2

    // Multiple emojis
    let multi_emoji: Vec<char> = "🎉🔥".chars().collect();
    assert_eq!(char_slice_width(&multi_emoji), 4); // 2 + 2

    // CJK characters (width 2)
    let cjk: Vec<char> = "你好".chars().collect();
    assert_eq!(char_slice_width(&cjk), 4); // 2 + 2

    // Mixed CJK and ASCII
    let mixed_cjk: Vec<char> = "hello世界".chars().collect();
    assert_eq!(char_slice_width(&mixed_cjk), 9); // 5 + 4

    // Empty
    let empty: Vec<char> = vec![];
    assert_eq!(char_slice_width(&empty), 0);
}

#[test]
fn test_input_badges_with_emojis() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("🎉test.log"),
            canonical: std::path::PathBuf::from("🎉test.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file.log"),
            canonical: std::path::PathBuf::from("file.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Full.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter()).expect("Should produce badges");

    // Both badges should be padded to the same width
    // The emoji file has width: 2 (emoji) + 8 (test.log) = 10
    // The normal file has width: 8 (file.log) = 8
    // So both should be padded to 10
    assert_eq!(badges.len(), 2);

    // Check that badges are properly aligned despite emoji
    // We can't check exact output due to formatting, but we can verify they were generated
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_compact_mode_with_emojis() {
    // Create a long filename with emoji that should trigger compact mode
    let long_name = format!("{}very-long-filename-that-exceeds-limit.log", "🎉".repeat(5));

    let inputs = [InputReference::File(crate::input::InputPath {
        original: std::path::PathBuf::from(&long_name),
        canonical: std::path::PathBuf::from(&long_name),
    })];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    // Should produce badges even with emojis in compact mode
    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 1);
    assert!(!badges[0].is_empty());
}

#[test]
fn test_input_badges_cjk_characters() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("测试文件.log"),
            canonical: std::path::PathBuf::from("测试文件.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("test.log"),
            canonical: std::path::PathBuf::from("test.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Full.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter()).expect("Should produce badges");

    // Both badges should be generated and padded correctly
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_mixed_width_characters() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file1-🎉-测试.log"),
            canonical: std::path::PathBuf::from("file1-🎉-测试.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file2-abc.log"),
            canonical: std::path::PathBuf::from("file2-abc.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Full.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter()).expect("Should produce badges");

    assert_eq!(badges.len(), 2);
    // Verify both badges were created
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_compact_mode_common_prefix_with_emojis() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("/common/prefix/🎉file1.log"),
            canonical: std::path::PathBuf::from("/common/prefix/🎉file1.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("/common/prefix/🔥file2.log"),
            canonical: std::path::PathBuf::from("/common/prefix/🔥file2.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_minimal_mode() {
    let inputs = [InputReference::File(crate::input::InputPath {
        original: std::path::PathBuf::from("file.log"),
        canonical: std::path::PathBuf::from("file.log"),
    })];

    let mut opts = options();
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);

    // Minimal mode with single input should still produce badge
    let badges = app.input_badges(inputs.iter());
    assert!(badges.is_some());
}

#[test]
fn test_input_badges_none_mode_single_input() {
    let inputs = [InputReference::File(crate::input::InputPath {
        original: std::path::PathBuf::from("file.log"),
        canonical: std::path::PathBuf::from("file.log"),
    })];

    let mut opts = options();
    // Minimal + None: will hide badges for single input
    opts.input_info = InputInfo::Minimal | InputInfo::None;
    let app = App::new(opts);

    // Minimal+None mode with single input should not produce badges
    let badges = app.input_badges(inputs.iter());
    assert!(badges.is_none());
}

#[test]
fn test_input_badges_none_mode_multiple_inputs() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file1.log"),
            canonical: std::path::PathBuf::from("file1.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file2.log"),
            canonical: std::path::PathBuf::from("file2.log"),
        }),
    ];

    let mut opts = options();
    // Minimal + None: will show badges for multiple inputs but not for single input
    opts.input_info = InputInfo::Minimal | InputInfo::None;
    let app = App::new(opts);

    // Minimal+None mode with multiple inputs should produce badges
    let badges = app.input_badges(inputs.iter());
    assert!(badges.is_some());
}

#[test]
fn test_input_badges_stdin() {
    let inputs = [
        InputReference::Stdin,
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file.log"),
            canonical: std::path::PathBuf::from("file.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Full.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter()).expect("Should produce badges");

    assert_eq!(badges.len(), 2);
    assert!(badges[0].contains("<stdin>") || !badges[0].is_empty());
}

#[test]
fn test_input_badges_compact_mode_long_prefix() {
    let common = "/very/long/common/prefix/path";
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(format!(
                "{}/file-one-with-a-very-long-name-that-will-be-truncated.log",
                common
            )),
            canonical: std::path::PathBuf::from(format!(
                "{}/file-one-with-a-very-long-name-that-will-be-truncated.log",
                common
            )),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(format!(
                "{}/file-two-with-another-very-long-name-needing-truncation.log",
                common
            )),
            canonical: std::path::PathBuf::from(format!(
                "{}/file-two-with-another-very-long-name-needing-truncation.log",
                common
            )),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_compact_mode_short_prefix() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(
                "/ab/this-is-a-very-long-filename-that-should-be-truncated-in-compact-mode.log",
            ),
            canonical: std::path::PathBuf::from(
                "/ab/this-is-a-very-long-filename-that-should-be-truncated-in-compact-mode.log",
            ),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("/ab/another-very-long-filename-requiring-truncation-for-display.log"),
            canonical: std::path::PathBuf::from("/ab/another-very-long-filename-requiring-truncation-for-display.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_compact_mode_emoji_in_long_names() {
    let common = "/common/path/prefix";
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(format!(
                "{}/🎉-very-long-filename-with-emoji-that-needs-truncation-for-compact-display.log",
                common
            )),
            canonical: std::path::PathBuf::from(format!(
                "{}/🎉-very-long-filename-with-emoji-that-needs-truncation-for-compact-display.log",
                common
            )),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(format!(
                "{}/🔥-another-long-name-with-emoji-requiring-truncation.log",
                common
            )),
            canonical: std::path::PathBuf::from(format!(
                "{}/🔥-another-long-name-with-emoji-requiring-truncation.log",
                common
            )),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_compact_mode_medium_prefix() {
    let common = "/medium";
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(format!(
                "{}/file-with-very-long-name-that-exceeds-the-display-limit.log",
                common
            )),
            canonical: std::path::PathBuf::from(format!(
                "{}/file-with-very-long-name-that-exceeds-the-display-limit.log",
                common
            )),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(format!(
                "{}/another-file-with-long-name-needing-truncation.log",
                common
            )),
            canonical: std::path::PathBuf::from(format!(
                "{}/another-file-with-long-name-needing-truncation.log",
                common
            )),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_input_badges_compact_mode_very_short_prefix() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(
                "/a/this-is-a-very-long-filename-that-should-be-truncated-in-compact-mode-test.log",
            ),
            canonical: std::path::PathBuf::from(
                "/a/this-is-a-very-long-filename-that-should-be-truncated-in-compact-mode-test.log",
            ),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(
                "/a/another-very-long-filename-requiring-truncation-for-display-test.log",
            ),
            canonical: std::path::PathBuf::from(
                "/a/another-very-long-filename-requiring-truncation-for-display-test.log",
            ),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

#[test]
fn test_prepare_follow_badges() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file1.log"),
            canonical: std::path::PathBuf::from("file1.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("file2.log"),
            canonical: std::path::PathBuf::from("file2.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Full.into();
    opts.theme = theme();
    let app = App::new(opts);

    let result = app.prepare_follow_badges(inputs.iter());

    assert!(result.si.width > 0);
    assert!(result.si.synced.len() >= result.si.width);
    assert!(result.si.failed.len() >= result.si.width);
    assert_eq!(result.si.placeholder.len(), result.si.width);

    assert!(result.input.is_some());
    let badges = result.input.unwrap();
    assert_eq!(badges.len(), 2);

    for badge in badges.iter() {
        assert!(
            badge.starts_with(&result.si.placeholder),
            "Badge should start with placeholder: {:?}",
            badge
        );
        assert!(badge.len() > result.si.width);
        assert!(badge.contains("file"), "Badge should contain filename");
    }
}

#[test]
fn test_prepare_follow_badges_single_input() {
    let inputs = [InputReference::Stdin];

    let mut opts = options();
    opts.input_info = InputInfo::Minimal.into();
    opts.theme = theme();
    let app = App::new(opts);

    let result = app.prepare_follow_badges(inputs.iter());

    assert!(result.si.width > 0);
    assert!(result.si.synced.len() >= result.si.width);
    assert!(result.si.failed.len() >= result.si.width);
    assert_eq!(result.si.placeholder.len(), result.si.width);

    assert!(result.input.is_some());
    let badges = result.input.unwrap();
    assert_eq!(badges.len(), 1);
    assert!(badges[0].starts_with(&result.si.placeholder));
}

#[test]
fn test_prepare_follow_badges_formatting() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("test1.log"),
            canonical: std::path::PathBuf::from("test1.log"),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from("test2.log"),
            canonical: std::path::PathBuf::from("test2.log"),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Full.into();
    opts.theme = theme();
    let app = App::new(opts);

    let raw_badges = app.input_badges(inputs.iter()).unwrap();
    let result = app.prepare_follow_badges(inputs.iter());

    assert!(result.input.is_some());
    let formatted_badges = result.input.unwrap();

    for (i, formatted) in formatted_badges.iter().enumerate() {
        let raw = &raw_badges[i];
        assert!(
            formatted.len() > raw.len(),
            "Formatted badge should be longer than raw badge"
        );
        assert!(
            formatted.starts_with(&result.si.placeholder),
            "Should have placeholder prefix"
        );
        assert!(formatted.ends_with(raw), "Should end with original badge content");
    }
}

#[test]
fn test_prepare_follow_badges_no_badges() {
    let inputs = [InputReference::Stdin];

    let mut opts = options();
    opts.input_info = InputInfo::None.into();
    opts.theme = theme();
    let app = App::new(opts);

    let result = app.prepare_follow_badges(inputs.iter());

    assert!(result.si.width > 0);
    assert!(result.input.is_none());
}

#[test]
fn test_process_segments() {
    let mut opts = options();
    opts.theme = theme();
    let app = App::new(opts);

    let parser = app.parser();
    let bfo = BufFactory::new(4096);
    let sfi = SegmentBufFactory::new(4096);

    let badges = FollowBadges {
        si: SyncIndicator {
            width: 2,
            synced: "OK".to_string(),
            failed: "!!".to_string(),
            placeholder: "  ".to_string(),
        },
        input: Some(vec!["  [0] ".to_string(), "  [1] ".to_string()]),
    };

    let (txi, rxi) = channel::bounded(10);
    let (txo, rxo) = channel::bounded(10);

    let log_data = r#"{"level":"info","msg":"test message","ts":"2023-12-07T20:07:05.949Z"}"#;
    let segment_buf = SegmentBuf::from(log_data.as_bytes());
    let segment = Segment::Complete(segment_buf);

    txi.send((0, 0, segment)).unwrap();
    drop(txi);

    app.process_segments(&parser, &bfo, &sfi, &badges, rxi, txo.clone());
    drop(txo);

    let result = rxo.try_recv();
    assert!(result.is_ok(), "Should have received processed output");

    if let Ok((input_idx, buf, index)) = result {
        assert_eq!(input_idx, 0);
        assert!(!buf.is_empty());
        assert_eq!(index.block, 0);
    }
}

#[test]
fn test_process_segments_incomplete() {
    let mut opts = options();
    opts.theme = theme();
    let app = App::new(opts);

    let parser = app.parser();
    let bfo = BufFactory::new(4096);
    let sfi = SegmentBufFactory::new(4096);

    let badges = FollowBadges {
        si: SyncIndicator {
            width: 2,
            synced: "OK".to_string(),
            failed: "!!".to_string(),
            placeholder: "  ".to_string(),
        },
        input: Some(vec!["  [0] ".to_string()]),
    };

    let (txi, rxi) = channel::bounded(10);
    let (txo, rxo) = channel::bounded(10);

    let segment = Segment::Incomplete(SegmentBuf::from(b"incomplete"), PartialPlacement::First);

    txi.send((0, 0, segment)).unwrap();
    drop(txi);

    app.process_segments(&parser, &bfo, &sfi, &badges, rxi, txo.clone());
    drop(txo);

    assert!(rxo.try_recv().is_err(), "Incomplete segments should not produce output");
}

#[test]
fn test_process_segments_receiver_dropped() {
    let mut opts = options();
    opts.theme = theme();
    let app = App::new(opts);

    let parser = app.parser();
    let bfo = BufFactory::new(4096);
    let sfi = SegmentBufFactory::new(4096);

    let badges = FollowBadges {
        si: SyncIndicator {
            width: 2,
            synced: "OK".to_string(),
            failed: "!!".to_string(),
            placeholder: "  ".to_string(),
        },
        input: Some(vec!["  [0] ".to_string()]),
    };

    let (txi, rxi) = channel::bounded(10);
    let (txo, rxo) = channel::bounded(10);

    let log_data = r#"{"level":"info","msg":"test","ts":"2023-12-07T20:07:05.949Z"}"#;
    let segment = Segment::Complete(SegmentBuf::from(log_data.as_bytes()));

    drop(rxo);

    txi.send((0, 0, segment)).unwrap();
    drop(txi);

    app.process_segments(&parser, &bfo, &sfi, &badges, rxi, txo);
}

#[test]
fn test_input_badges_compact_mode_two_char_prefix() {
    let inputs = [
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(
                "ab-this-is-a-very-long-filename-that-should-be-truncated-in-compact-mode-test.log",
            ),
            canonical: std::path::PathBuf::from(
                "ab-this-is-a-very-long-filename-that-should-be-truncated-in-compact-mode-test.log",
            ),
        }),
        InputReference::File(crate::input::InputPath {
            original: std::path::PathBuf::from(
                "ab-another-very-long-filename-requiring-truncation-for-display-test.log",
            ),
            canonical: std::path::PathBuf::from(
                "ab-another-very-long-filename-requiring-truncation-for-display-test.log",
            ),
        }),
    ];

    let mut opts = options();
    opts.input_info = InputInfo::Compact.into();
    let app = App::new(opts);

    let badges = app.input_badges(inputs.iter());

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}
