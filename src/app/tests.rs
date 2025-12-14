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
        "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;7;33m|WRN|\u{1b}[0m \u{1b}[0;1;39mNo time or size retention was set so using the default time retention \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0;39m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
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
        "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;36m|INF|\u{1b}[0m \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0;39m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
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
        "\u{1b}[0;2;3m2023-12-07 20:07:05.949 \u{1b}[0;36m|INF| \u{1b}[0;32mmsg\u{1b}[0;2m=\u{1b}[0;93m[\u{1b}[0;39mx\u{1b}[0;93m \u{1b}[0;39my\u{1b}[0;93m] \u{1b}[0;32mduration\u{1b}[0;2m=\u{1b}[0;39m15d\u{1b}[0;2;3m @ main.go:539\u{1b}[0m\n",
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
