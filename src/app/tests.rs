// super imports
use super::*;

// std imports
use std::io::Cursor;

// third-party imports
use chrono::{Offset, Utc};
use chrono_tz::UTC;
use maplit::hashmap;
use rstest::rstest;

// local imports
use crate::{
    LinuxDateFormat,
    filtering::MatchOptions,
    level::{InfallibleLevel, Level},
    model::{FieldFilterSet, RecordWithSource},
    scanning::{BufFactory, PartialPlacement, Segment, SegmentBuf, SegmentBufFactory},
    settings::{self, AsciiMode, DisplayVariant, ExpansionMode, MessageFormat, MessageFormatting},
    syntax::*,
    themecfg,
};

fn theme() -> Arc<Theme> {
    Sample::sample()
}

/// Holds written bytes until flushed and then sends them to a channel.
struct FlushProbe {
    pending: Vec<u8>,
    flushed: channel::Sender<Vec<u8>>,
}

impl Write for FlushProbe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.pending.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.pending.is_empty() {
            self.flushed.send(std::mem::take(&mut self.pending)).ok();
        }
        Ok(())
    }
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
        expand: Default::default(),
        output_delimiter: "\n".to_string(),
    }
}

fn test_badges(width: usize) -> FollowBadges {
    FollowBadges {
        si: SyncIndicator {
            width,
            synced: "S".repeat(width),
            failed: "F".repeat(width),
            placeholder: " ".repeat(width),
        },
        input: None,
    }
}

fn ts(sec: i64, nsec: u32) -> Timestamp {
    Timestamp { sec, nsec }
}

/// A message from a follow mode worker to the merger.
type Processed = (usize, Vec<u8>, TimestampIndex);

/// Returns a worker message for a block holding a single record with timestamp `sec`.
fn record_block(input: usize, block: usize, text: &str, sec: i64) -> Processed {
    let index = TimestampIndex {
        block,
        lines: vec![TimestampIndexLine {
            location: 0..text.len(),
            ts: ts(sec, 0),
        }],
        part: None,
    };
    (input, text.as_bytes().to_vec(), index)
}

/// Returns a worker message for a block holding a part of a log entry too large to be read at once.
fn part_block(input: usize, block: usize, text: &str, part: PartialPlacement) -> Processed {
    let index = TimestampIndex {
        part: Some(part),
        ..TimestampIndex::new(block)
    };
    (input, text.as_bytes().to_vec(), index)
}

/// Returns a worker message for a block holding text that is not a record.
fn text_block(input: usize, block: usize, text: &str) -> Processed {
    (input, text.as_bytes().to_vec(), TimestampIndex::new(block))
}

/// Merges `messages`, received one by one, with every entry due for output as soon as it comes,
/// and returns the output.
fn merge(sync_interval: Duration, messages: Vec<Processed>) -> String {
    let app = App::new(Options {
        sync_interval,
        ..options()
    });
    let (txo, rxo) = channel::unbounded();
    for message in messages {
        txo.send(message).unwrap();
    }
    drop(txo);

    let mut output = Vec::new();
    // With zero concurrency the memory limit is zero, so nothing waits for the sync interval.
    app.merge_segments(&test_badges(2), rxo, &mut output, 0).unwrap();
    String::from_utf8(output).unwrap()
}

/// Scans each chunk as a separate follow mode scan and returns the text of the
/// segments sent after each of them.
fn scan_chunks(segment_size: usize, chunks: &[(&[u8], bool)]) -> Vec<Vec<String>> {
    let sfi = Arc::new(SegmentBufFactory::new(segment_size));
    let scanner = Scanner::new(sfi, Delimiter::default());
    let (txi, rxi) = channel::unbounded();
    let mut blocks = BlockScanner::new(&scanner, Delimiter::default(), segment_size);

    let text = |segment: Segment| match segment {
        Segment::Complete(buf) | Segment::Incomplete(buf, _) => String::from_utf8_lossy(buf.data()).into_owned(),
    };

    chunks
        .iter()
        .map(|&(chunk, hold_tail)| {
            blocks
                .scan(&mut Cursor::new(chunk.to_vec()), 0, &txi, hold_tail)
                .unwrap();
            rxi.try_iter().map(|(_, _, _, segment)| text(segment)).collect_vec()
        })
        .collect()
}

/// The input and byte range of a formatted record, as seen by the formatter.
type CapturedOrigin = (InputDescriptor, Range<u64>);

/// Records the origin of each formatted record and writes its source.
#[derive(Default)]
struct OriginCapture {
    origins: std::sync::Mutex<Vec<CapturedOrigin>>,
}

impl RecordWithSourceFormatter for OriginCapture {
    fn format_record(&self, buf: &mut Vec<u8>, _: Range<usize>, rec: RecordWithSource) {
        buf.extend_from_slice(rec.source);
        self.origins
            .lock()
            .unwrap()
            .push((rec.origin.input.clone(), rec.origin.range));
    }
}

fn app_with_origin_capture(options: Options) -> (App, Arc<OriginCapture>) {
    let capture = Arc::new(OriginCapture::default());
    let mut app = App::new(options);
    app.formatter = capture.clone();
    (app, capture)
}

fn file_input<S: Into<String>>(name: &str, s: S) -> InputHolder {
    InputHolder::new(
        InputReference::File(crate::input::InputPath::ephemeral(name.into())),
        Some(Box::new(Cursor::new(s.into()))),
    )
}

fn descriptor(index: usize, name: &str) -> InputDescriptor {
    InputDescriptor::new(index, name)
}

fn descriptors(inputs: &[InputReference]) -> Vec<InputDescriptor> {
    inputs.iter().enumerate().map(Into::into).collect_vec()
}

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

#[rstest]
#[case::filtered_first(r#"{"level":"debug","msg":"m1"}{"level":"info","msg":"m2"}"#, "|INF| m2\n")]
#[case::filtered_first_two(
    r#"{"level":"debug","msg":"m1"}{"level":"debug","msg":"m2"}{"level":"info","msg":"m3"}"#,
    "|INF| m3\n"
)]
#[case::filtered_last(r#"{"level":"info","msg":"m1"}{"level":"debug","msg":"m2"}"#, "|INF| m1\n")]
#[case::filtered_middle(
    r#"{"level":"info","msg":"m1"}{"level":"debug","msg":"m2"}{"level":"info","msg":"m3"}"#,
    "|INF| m1\n|INF| m3\n"
)]
#[case::filtered_all(r#"{"level":"debug","msg":"m1"}{"level":"debug","msg":"m2"}"#, "")]
fn test_filter_records_sharing_line(#[case] data: &str, #[case] expected: &str) {
    let mut output = Vec::new();
    let app = App::new(
        options().with_filter(
            Filter {
                level: Some(Level::Info),
                ..Default::default()
            }
            .into(),
        ),
    );
    app.run(vec![input(format!("{data}\n"))], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
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

#[rstest]
#[case::filter(
    options().with_filter(Filter { level: Some(Level::Error), ..Default::default() }.into()),
    "2024-01-25 18:10:20.435 |ERR| m1\n"
)]
#[case::input_format(
    Options { input_format: Some(InputFormat::Json), delimiter: Delimiter::Json, ..options() },
    "2024-01-25 18:10:20.435 |ERR| m1\n2024-01-25 18:10:21.764 |INF| m2\n"
)]
fn test_incomplete_segment_hidden(#[case] opts: Options, #[case] expected: &str, #[values(false, true)] follow: bool) {
    let input = input(format!(
        concat!(
            r#"{{"level":"error","ts":"2024-01-25T19:10:20.435369+01:00","msg":"m1"}}"#,
            "\n",
            "plain text line\n",
            "{}\n",
            r#"{{"level":"info","ts":"2024-01-25T19:10:21.764733+01:00","msg":"m2"}}"#,
            "\n",
        ),
        "x".repeat(200),
    ));

    let mut output = Vec::new();
    let app = App::new(Options {
        buffer_size: NonZeroUsize::new(32).unwrap(),
        max_message_size: NonZeroUsize::new(128).unwrap(),
        concurrency: 2,
        follow,
        ..opts
    });
    app.run(vec![input], &mut output).unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
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
        options()
            .with_expansion(ExpansionMode::Never)
            .with_fields(FieldOptions {
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
fn test_nested_level_field_in_second_variant() {
    let variant = |name: &str, level: Level, value: &str| settings::RawLevelFieldVariant {
        names: vec![name.to_owned()],
        values: [(level.into(), vec![value.to_owned()])].into_iter().collect(),
        level: None,
    };
    let fields = || FieldOptions {
        settings: Fields {
            predefined: settings::PredefinedFields {
                level: settings::LevelField {
                    variants: vec![
                        variant("level", Level::Info, "info"),
                        variant("log.level", Level::Warning, "warn"),
                    ],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    for (data, expected) in [
        (
            r#"{"message":"test","log":{"level":"warn","x":1}}"#,
            "|WRN| test log={ x=1 }\n",
        ),
        (r#"message=test log.level=warn log.x=1"#, "|WRN| test log.x=1\n"),
    ] {
        let mut output = Vec::new();
        let app = App::new(options().with_expansion(ExpansionMode::Never).with_fields(fields()));
        app.run(vec![input(format!("{data}\n"))], &mut output).unwrap();
        assert_eq!(std::str::from_utf8(&output).unwrap(), expected, "{data}");
    }
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
    formatter_ascii.format_record(&mut buf_ascii, 0..0, &record);
    let result_ascii = String::from_utf8(buf_ascii).unwrap();

    // Test Unicode mode
    let mut buf_utf8 = Vec::new();
    formatter_utf8.format_record(&mut buf_utf8, 0..0, &record);
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
    let badges_ascii = app_ascii.input_badges(&descriptors(&inputs));
    assert!(badges_ascii.is_some(), "Should produce badges");
    let badges_a = badges_ascii.unwrap();
    println!("ASCII badges: {:?}", badges_a);

    // Get badges with ASCII mode OFF (Unicode)
    let badges_utf8 = app_unicode.input_badges(&descriptors(&inputs));
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

#[test]
fn test_expand_always() {
    let input = input(concat!(
        r#"level=debug time=2024-01-25T19:10:20.435369+01:00 msg=m1 a.b.c=10 a.b.d=20 a.c.b=11 caller=src1"#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(Options {
        expand: ExpansionMode::Always,
        ..options()
    });

    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        format!(
            concat!(
                "2024-01-25 18:10:20.435 |{ld}| m1 @ src1\n",
                "                        |{lx}|   > a.b.c=10\n",
                "                        |{lx}|   > a.b.d=20\n",
                "                        |{lx}|   > a.c.b=11\n"
            ),
            ld = LEVEL_DEBUG,
            lx = LEVEL_EXPANDED,
        ),
    );
}

#[test]
fn test_expand_never() {
    let input = input(concat!(
        r#"level=debug time=2024-01-25T19:10:20.435369+01:00 msg="some long long long long message" caller=src1 a=long-long-long-value-1 b=long-long-long-value-2 c=long-long-long-value-3 d=long-long-long-value-4 e=long-long-long-value-5 f=long-long-long-value-6 g=long-long-long-value-7 h=long-long-long-value-8 i=long-long-long-value-9 j=long-long-long-value-10 k=long-long-long-value-11 l=long-long-long-value-12 m=long-long-long-value-13 n=long-long-long-value-14 o=long-long-long-value-15 p=long-long-long-value-16 q=long-long-long-value-17 r=long-long-long-value-18 s=long-long-long-value-19 t=long-long-long-value-20 u=long-long-long-value-21 v=long-long-long-value-22 w=long-long-long-value-23 x=long-long-long-value-24 w=long-long-long-value-26 z=long-long-long-value-26"#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(Options {
        expand: ExpansionMode::Never,
        ..options()
    });

    app.run(vec![input], &mut output).unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "2024-01-25 18:10:20.435 |DBG| some long long long long message a=long-long-long-value-1 b=long-long-long-value-2 c=long-long-long-value-3 d=long-long-long-value-4 e=long-long-long-value-5 f=long-long-long-value-6 g=long-long-long-value-7 h=long-long-long-value-8 i=long-long-long-value-9 j=long-long-long-value-10 k=long-long-long-value-11 l=long-long-long-value-12 m=long-long-long-value-13 n=long-long-long-value-14 o=long-long-long-value-15 p=long-long-long-value-16 q=long-long-long-value-17 r=long-long-long-value-18 s=long-long-long-value-19 t=long-long-long-value-20 u=long-long-long-value-21 v=long-long-long-value-22 w=long-long-long-value-23 x=long-long-long-value-24 w=long-long-long-value-26 z=long-long-long-value-26 @ src1\n",
    );
}

#[test]
fn test_expand_value_with_time() {
    let input = input(concat!(
        r#"level=debug time=2024-01-25T19:10:20.435369+01:00 msg=hello caller=src1 a="line one\nline two\nline three\n""#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(Options {
        expand: ExpansionMode::Always,
        theme: Theme::from(themecfg::Theme {
            elements: themecfg::StylePack::new(hashmap! {
                Element::ValueExpansion => themecfg::Style::default(),
            }),
            ..Default::default()
        })
        .into(),
        ..options()
    });

    app.run(vec![input], &mut output).unwrap();

    let actual = std::str::from_utf8(&output).unwrap();
    let expected = format!(
        concat!(
            "2024-01-25 18:10:20.435 |{ld}| hello @ src1\n",
            "                        |{lx}|   > a={vh}\n",
            "                        |{lx}|     {vi}line one\n",
            "                        |{lx}|     {vi}line two\n",
            "                        |{lx}|     {vi}line three\n",
        ),
        ld = LEVEL_DEBUG,
        lx = LEVEL_EXPANDED,
        vh = EXPANDED_VALUE_HEADER,
        vi = EXPANDED_VALUE_INDENT,
    );

    assert_eq!(actual, expected, "\nactual:\n{}expected:\n{}", actual, expected);
}

#[test]
fn test_expand_value_without_time() {
    let input = input(concat!(
        r#"level=debug msg=hello caller=src1 a="line one\nline two\nline three\n""#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(Options {
        expand: ExpansionMode::Always,
        theme: Theme::from(themecfg::Theme {
            elements: themecfg::StylePack::new(hashmap! {
                Element::ValueExpansion => themecfg::Style::default(),
            }),
            ..Default::default()
        })
        .into(),
        ..options()
    });

    app.run(vec![input], &mut output).unwrap();

    let actual = std::str::from_utf8(&output).unwrap();
    let expected = format!(
        concat!(
            "|{ld}| hello @ src1\n",
            "|{lx}|   > a={vh}\n",
            "|{lx}|     {vi}line one\n",
            "|{lx}|     {vi}line two\n",
            "|{lx}|     {vi}line three\n",
        ),
        ld = LEVEL_DEBUG,
        lx = LEVEL_EXPANDED,
        vh = EXPANDED_VALUE_HEADER,
        vi = EXPANDED_VALUE_INDENT,
    );

    assert_eq!(actual, expected, "\nactual:\n{}expected:\n{}", actual, expected);
}

#[test]
fn test_expand_empty_values() {
    let input = input(concat!(r#"level=debug msg=hello caller=src1 a="" b="" c="""#, "\n",));

    let mut output = Vec::new();
    let app = App::new(options());

    app.run(vec![input], &mut output).unwrap();

    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        concat!(r#"|DBG| hello a="" b="" c="" @ src1"#, "\n")
    );
}

#[test]
fn test_expand_empty_hidden_values() {
    let input = input(concat!(r#"level=debug msg=hello caller=src1 a="" b="" c="""#, "\n",));

    let mut output = Vec::new();
    let app = App::new(Options {
        hide_empty_fields: true,
        ..options()
    });

    app.run(vec![input], &mut output).unwrap();

    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        concat!(r#"|DBG| hello @ src1"#, "\n")
    );
}

#[test]
fn test_expand_unparseable_timestamp() {
    let input = input(concat!(
        r#"level=debug time=invalid-timestamp msg=hello caller=src1 a="line one\nline two\nline three\n""#,
        "\n",
    ));

    let mut output = Vec::new();
    let app = App::new(Options {
        expand: ExpansionMode::Always,
        theme: Theme::from(themecfg::Theme {
            elements: themecfg::StylePack::new(hashmap! {
                Element::ValueExpansion => themecfg::Style::default(),
            }),
            ..Default::default()
        })
        .into(),
        ..options()
    });

    app.run(vec![input], &mut output).unwrap();

    let actual = std::str::from_utf8(&output).unwrap();
    let expected = format!(
        concat!(
            "|{ld}| hello @ src1\n",
            "|{lx}|   > ts=invalid-timestamp\n",
            "|{lx}|   > a={vh}\n",
            "|{lx}|     {vi}line one\n",
            "|{lx}|     {vi}line two\n",
            "|{lx}|     {vi}line three\n",
        ),
        ld = LEVEL_DEBUG,
        lx = LEVEL_EXPANDED,
        vh = EXPANDED_VALUE_HEADER,
        vi = EXPANDED_VALUE_INDENT,
    );

    assert_eq!(actual, expected, "\nactual:\n{}expected:\n{}", actual, expected);
}

#[test]
fn test_input_badges() {
    let inputs = (1..12).map(|i| input(format!("msg=hello input={}\n", i))).collect_vec();

    let app = App::new(Options {
        input_info: InputInfo::Minimal.into(),
        ..options()
    });

    let mut output = Vec::new();
    app.run(inputs, &mut output).unwrap();

    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        concat!(
            " #0 | hello input=1\n",
            " #1 | hello input=2\n",
            " #2 | hello input=3\n",
            " #3 | hello input=4\n",
            " #4 | hello input=5\n",
            " #5 | hello input=6\n",
            " #6 | hello input=7\n",
            " #7 | hello input=8\n",
            " #8 | hello input=9\n",
            " #9 | hello input=10\n",
            "#10 | hello input=11\n",
        )
    );
}

#[test]
fn test_grapheme_slice_width() {
    use unicode_segmentation::UnicodeSegmentation;

    // ASCII characters
    let ascii: Vec<String> = "hello".graphemes(true).map(String::from).collect();
    assert_eq!(grapheme_slice_width(&ascii), 5);

    // Emoji (width 2)
    let emoji: Vec<String> = "🎉".graphemes(true).map(String::from).collect();
    assert_eq!(grapheme_slice_width(&emoji), 2);

    // Emoji with variation selector (should be treated as single grapheme)
    let emoji_vs: Vec<String> = "⚠️".graphemes(true).map(String::from).collect();
    assert_eq!(emoji_vs.len(), 1, "Emoji with variation selector should be 1 grapheme");
    assert_eq!(grapheme_slice_width(&emoji_vs), 2);

    // Mixed ASCII and emoji with variation selector
    let mixed: Vec<String> = "x-⚠️-.log".graphemes(true).map(String::from).collect();
    assert_eq!(grapheme_slice_width(&mixed), 9); // x(1) -(1) ⚠️(2) -(1) .(1) l(1) o(1) g(1)

    // CJK characters (width 2)
    let cjk: Vec<String> = "你好".graphemes(true).map(String::from).collect();
    assert_eq!(grapheme_slice_width(&cjk), 4); // 2 + 2

    // Empty
    let empty: Vec<String> = vec![];
    assert_eq!(grapheme_slice_width(&empty), 0);
}

#[test]
fn test_input_badges_with_emoji_variation_selectors() {
    // Test that emoji with variation selectors are preserved correctly
    let inputs = [InputReference::File(crate::input::InputPath {
        original: std::path::PathBuf::from("x-⚠️-.log"),
        canonical: std::path::PathBuf::from("x-⚠️-.log"),
    })];

    let mut opts = options();
    opts.input_info = InputInfo::Full.into();
    let app = App::new(opts);

    let badges = app.input_badges(&descriptors(&inputs)).expect("Should produce badges");
    assert_eq!(badges.len(), 1);

    // The badge should contain the emoji with variation selector
    // We can't check the exact styled output, but we can verify it contains the emoji
    assert!(
        badges[0].contains("⚠️"),
        "Badge should preserve emoji with variation selector"
    );
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

    let badges = app.input_badges(&descriptors(&inputs)).expect("Should produce badges");

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

    let badges = app.input_badges(&descriptors(&inputs));

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

    let badges = app.input_badges(&descriptors(&inputs)).expect("Should produce badges");

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

    let badges = app.input_badges(&descriptors(&inputs)).expect("Should produce badges");

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

    let badges = app.input_badges(&descriptors(&inputs));

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
    let badges = app.input_badges(&descriptors(&inputs));
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
    let badges = app.input_badges(&descriptors(&inputs));
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
    let badges = app.input_badges(&descriptors(&inputs));
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

    let badges = app.input_badges(&descriptors(&inputs)).expect("Should produce badges");

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

    let badges = app.input_badges(&descriptors(&inputs));

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

    let badges = app.input_badges(&descriptors(&inputs));

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

    let badges = app.input_badges(&descriptors(&inputs));

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

    let badges = app.input_badges(&descriptors(&inputs));

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

    let badges = app.input_badges(&descriptors(&inputs));

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

    let result = app.prepare_follow_badges(&descriptors(&inputs));

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

    let result = app.prepare_follow_badges(&descriptors(&inputs));

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

    let raw_badges = app.input_badges(&descriptors(&inputs)).unwrap();
    let result = app.prepare_follow_badges(&descriptors(&inputs));

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

    let result = app.prepare_follow_badges(&descriptors(&inputs));

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

    txi.send((0, 0, 0, segment)).unwrap();
    drop(txi);

    app.process_segments(
        &parser,
        &bfo,
        &sfi,
        &badges,
        &[InputDescriptor::new(0, "<stdin>")],
        rxi,
        txo.clone(),
    );
    drop(txo);

    let result = rxo.try_recv();
    assert!(result.is_ok(), "Should have received processed output");

    if let Ok((input_idx, buf, index)) = result {
        assert_eq!(input_idx, 0);
        assert!(!buf.is_empty());
        assert_eq!(index.block, 0);
    }
}

#[rstest]
#[case::first(PartialPlacement::First, "long", "  [0] long")]
#[case::next(PartialPlacement::Next, "long", "long")]
#[case::last(PartialPlacement::Last, "long\n", "long")]
#[case::last_unterminated(PartialPlacement::Last, "long", "long")]
fn test_process_segments_incomplete(
    #[case] part: PartialPlacement,
    #[case] data: &str,
    #[case] expected: &str,
    #[values(None, Some(InputFormat::Json))] input_format: Option<InputFormat>,
) {
    let mut opts = options();
    opts.theme = theme();
    opts.input_format = input_format;
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

    let segment = Segment::Incomplete(SegmentBuf::from(data.as_bytes()), part);

    txi.send((0, 3, 0, segment)).unwrap();
    drop(txi);

    app.process_segments(
        &parser,
        &bfo,
        &sfi,
        &badges,
        &[InputDescriptor::new(0, "<stdin>")],
        rxi,
        txo.clone(),
    );
    drop(txo);

    // Every block is sent, as the merger takes the blocks of an input in order,
    // but the part is shown only where text that is not a record is.
    let (input, buf, index) = rxo.try_recv().expect("a message for the block");
    assert_eq!(input, 0);
    assert_eq!(index.block, 3);
    assert!(index.lines.is_empty());
    if input_format.is_none() {
        assert_eq!(String::from_utf8(buf).unwrap(), expected);
        assert_eq!(index.part, Some(part));
    } else {
        assert!(buf.is_empty());
        assert_eq!(index.part, None);
    }
}

#[rstest]
#[case::complete(Segment::Complete(SegmentBuf::from(
    r#"{"level":"info","msg":"test","ts":"2023-12-07T20:07:05.949Z"}"#.as_bytes()
)))]
#[case::incomplete(Segment::Incomplete(SegmentBuf::from(b"long"), PartialPlacement::First))]
fn test_process_segments_receiver_dropped(#[case] segment: Segment) {
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

    drop(rxo);

    txi.send((0, 0, 0, segment)).unwrap();
    drop(txi);

    app.process_segments(
        &parser,
        &bfo,
        &sfi,
        &badges,
        &[InputDescriptor::new(0, "<stdin>")],
        rxi,
        txo,
    );
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

    let badges = app.input_badges(&descriptors(&inputs));

    assert!(badges.is_some());
    let badges = badges.unwrap();
    assert_eq!(badges.len(), 2);
    assert!(!badges[0].is_empty());
    assert!(!badges[1].is_empty());
}

// Critical tests based on real-world pretty-broken.log scenarios

#[test]
fn test_bug_multiline_json_missing_input_badges_on_closing_braces() {
    // This test catches the critical bug where closing braces in multi-line JSON
    // lose their input badge prefix when using auto delimiter with input badges
    let input = input("{\n\"level\":\"info\",\n\"msg\":\"test\"\n}\n");
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    // Every line should have the input badge, including the closing brace
    let lines: Vec<&str> = output_str.lines().collect();

    // Find the closing brace line - it should have the input badge
    let closing_brace_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.contains("}") && l.trim().ends_with("}"))
        .copied()
        .collect();
    assert!(
        !closing_brace_lines.is_empty(),
        "should find closing brace lines. Lines: {:?}",
        lines
    );

    // The closing brace line should have an input badge
    let closing_brace_with_badge = closing_brace_lines.iter().any(|l| l.starts_with("#0"));
    assert!(
        closing_brace_with_badge,
        "closing brace should have input badge prefix. Lines: {:?}",
        closing_brace_lines
    );

    // All non-empty lines should start with the input badge
    for line in lines.iter().filter(|l| !l.is_empty()) {
        assert!(line.starts_with("#0"), "Line should have input badge: {:?}", line);
    }
}

#[test]
fn test_bug_multiline_remainder_missing_input_badges() {
    // This test catches the bug where multi-line remainder text (unparsed data)
    // after a JSON object loses input badges on subsequent lines
    let input = input(concat!(
        r#"{"level":"info","msg":"test"}"#,
        "\n",
        "\trem1\n",
        "\trem2\n",
    ));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let lines: Vec<&str> = output_str.lines().collect();

    assert_eq!(lines.len(), 3, "should have exactly 3 lines: {:?}", lines);

    // Find remainder lines
    let l1 = lines[1];
    let l2 = lines[2];

    // Both should have input badges
    assert!(l1.starts_with("#0"), "remainder line should have input badge: {:?}", l1);
    assert!(l2.starts_with("#0"), "remainder line should have input badge: {:?}", l2);
}

#[test]
fn test_bug_prefix_with_closing_brace_on_same_line() {
    // This test verifies that a closing brace on the same line before JSON
    // is correctly treated as a prefix. The fix ensures that when there's
    // }{"level":"info","msg":"test"}, the } is shown as prefix to the formatted JSON.
    let input = input(concat!(
        "{\n",
        r#""level":"info","#,
        "\n",
        r#""nested":{"#,
        "\n",
        r#""key":"val""#,
        "\n",
        "}\n",
        "}",
        r#"{"level":"info","msg":"test"}"#,
        "\n",
    ));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.allow_prefix = true;
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    // The "test" message should be formatted
    assert!(output_str.contains("test"), "should find 'test' message");

    // The line with "test" should have the closing brace as a prefix
    // because } appears on the same line before the JSON
    let test_line = output_str.lines().find(|l| l.contains("test")).unwrap();

    // The formatted output should have "}" as prefix (it's on the same line before JSON)
    assert!(
        test_line.starts_with("} "),
        "message should have closing brace prefix when it's on the same line, line: {:?}",
        test_line
    );
}

#[test]
fn test_bug_prefix_multiline_block_closing_brace_not_included() {
    // This test catches the original bug where closing braces from a previous
    // multi-line JSON block would be incorrectly included in the prefix of the
    // next JSON message when they appeared on separate lines.
    // For example: with input "}\n{"level":"info","msg":"test"}", the } is on
    // a separate line from the JSON, so it should NOT be included as prefix.
    let input = input(concat!(
        "{\n",
        r#""level":"info","#,
        "\n",
        r#""nested":{"#,
        "\n",
        r#""key":"val""#,
        "\n",
        "}\n",
        "}\n",
        r#"{"level":"info","msg":"test"}"#,
        "\n",
    ));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.allow_prefix = true;
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    // The "test" message should be formatted
    assert!(output_str.contains("test"), "should find 'test' message");

    // The line with "test" should NOT have a closing brace prefix because
    // the } appears on a different line (previous line) from the JSON
    let test_line = output_str.lines().find(|l| l.contains("test")).unwrap();

    // The formatted output should NOT have "} " at the start
    // because the closing brace was on a previous line
    assert!(
        !test_line.starts_with("} "),
        "message should NOT have closing brace prefix when it's on a different line, line: {:?}",
        test_line
    );

    // But the closing brace line itself should be in the output
    let has_closing_brace_line = output_str.lines().any(|l| l.trim() == "}" || l.contains("}"));
    assert!(
        has_closing_brace_line,
        "closing brace should appear in output on its own line"
    );
}

#[test]
fn test_bug_complex_pretty_broken_scenario() {
    // This is the full scenario from pretty-broken.log that exhibits multiple bugs
    let input = input(concat!(
        "{\n",
        "\"level\":\"info\",\n",
        "\"nested\":{\n",
        "\"key\":\"val\"\n",
        "}\n",
        "}{\"level\":\"info\",\"msg\":\"test\"}\n",
        "\trem1\n",
        "\n",
        "\trem2\n",
    ));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.allow_prefix = true;
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let lines: Vec<&str> = output_str.lines().collect();

    // Count how many lines have input badges
    let badged_lines = lines.iter().filter(|l| l.starts_with("#0")).count();
    let non_empty_lines = lines.iter().filter(|l| !l.trim().is_empty()).count();

    // All non-empty lines should have input badges
    assert_eq!(
        badged_lines, non_empty_lines,
        "all non-empty lines should have input badges, found {} badged out of {} non-empty lines\nlines: {:?}",
        badged_lines, non_empty_lines, lines
    );

    // The closing braces should have badges
    let closing_brace_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.contains("}") && l.trim().ends_with("}"))
        .copied()
        .collect();
    assert!(!closing_brace_lines.is_empty(), "should find closing brace lines");

    // All closing braces should have input badges
    for line in &closing_brace_lines {
        assert!(line.starts_with("#0"), "closing brace should have badge: {:?}", line);
    }

    // The remainder lines should have badges
    let rem1_line = lines.iter().find(|l| l.contains("rem1"));
    assert!(rem1_line.is_some() && rem1_line.unwrap().starts_with("#0"));

    let rem2_line = lines.iter().find(|l| l.contains("rem2"));
    assert!(rem2_line.is_some() && rem2_line.unwrap().starts_with("#0"));

    // The "test" message should not have "}␣" prefix
    let test_line = lines.iter().find(|l| l.contains("test"));
    assert!(test_line.is_some());
    let test_line = test_line.unwrap();
    assert!(
        !test_line.starts_with("} ") && !test_line.contains(" } "),
        "message should not have brace prefix: {:?}",
        test_line
    );
}

#[test]
fn test_bug_continuation_lines_with_input_badges() {
    // Multi-line pretty-printed JSON where continuation lines (indented) should
    // all have input badges when input-info is enabled
    let input = input(concat!(
        "{\n",
        "\"level\":\"info\",\n",
        "\"msg\":\"test\",\n",
        "\"nested\":{\n",
        "\"key\":\"val\"\n",
        "}\n",
        "}\n",
    ));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let lines: Vec<&str> = output_str.lines().collect();

    // Every line should have the input badge
    for (i, line) in lines.iter().enumerate() {
        if !line.is_empty() {
            assert!(line.starts_with("#0"), "line {} should have input badge: {:?}", i, line);
        }
    }

    // Should have at least 5 lines (multi-line JSON)
    assert!(lines.len() >= 5, "should have multiple lines for pretty-printed JSON");
}

#[test]
fn test_bug_multiline_unparsed_prefix_data_loss() {
    // This test catches the bug where multi-line unparsed prefixes before JSON
    // would lose all lines except the last one (data loss)
    let input = input(concat!(
        "prefix line 1\n",
        "prefix line 2\n",
        "prefix line 3\n",
        r#"{"level":"info","msg":"test1"}"#,
        "\n",
    ));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.allow_prefix = true;
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let lines: Vec<&str> = output_str.lines().collect();

    // All prefix lines should be present in the output
    assert!(
        output_str.contains("prefix line 1"),
        "should contain 'prefix line 1', output: {:?}",
        output_str
    );
    assert!(
        output_str.contains("prefix line 2"),
        "should contain 'prefix line 2', output: {:?}",
        output_str
    );
    assert!(
        output_str.contains("prefix line 3"),
        "should contain 'prefix line 3', output: {:?}",
        output_str
    );

    // All prefix lines should have input badges
    let prefix1_line = lines.iter().find(|l| l.contains("prefix line 1"));
    assert!(prefix1_line.is_some() && prefix1_line.unwrap().starts_with("#0"));

    let prefix2_line = lines.iter().find(|l| l.contains("prefix line 2"));
    assert!(prefix2_line.is_some() && prefix2_line.unwrap().starts_with("#0"));

    let prefix3_line = lines.iter().find(|l| l.contains("prefix line 3"));
    assert!(prefix3_line.is_some() && prefix3_line.unwrap().starts_with("#0"));

    // The formatted JSON line should also be present
    assert!(output_str.contains("test1"), "should contain 'test1' message");
}

#[test]
fn test_bug_raw_output_missing_input_badges_on_continuation_lines() {
    // This test catches the bug where raw output (--raw) with multi-line JSON
    // would lose input badges on continuation lines
    let input = input(concat!("{\n", "\"level\":\"info\",\n", "\"msg\":\"test1\"\n", "}\n",));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.allow_prefix = true;
    opts.raw = true;
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let lines: Vec<&str> = output_str.lines().collect();

    // All lines should have input badges (this is raw output)
    for (i, line) in lines.iter().enumerate() {
        if !line.is_empty() {
            assert!(
                line.starts_with("#0"),
                "line {} should have input badge in raw output: {:?}",
                i,
                line
            );
        }
    }

    // Should have at least 4 lines (multi-line JSON)
    let non_empty_lines: Vec<&str> = lines.iter().filter(|l| !l.is_empty()).copied().collect();
    assert!(
        non_empty_lines.len() >= 4,
        "should have at least 4 non-empty lines for multi-line JSON, got: {:?}",
        non_empty_lines
    );

    // Verify the JSON structure is preserved in raw output
    assert!(output_str.contains("{"), "should contain opening brace");
    assert!(output_str.contains("\"level\":\"info\""), "should contain level field");
    assert!(output_str.contains("\"msg\":\"test1\""), "should contain msg field");
    assert!(output_str.contains("}"), "should contain closing brace");
}

#[test]
fn test_bug_comprehensive_multiline_prefix_remainder_raw() {
    // Comprehensive test combining:
    // 1. Multi-line unparsed prefixes (all lines should be preserved)
    // 2. Multi-line JSON input
    // 3. Multi-line remainders after JSON
    // 4. Raw output mode
    // 5. Input badges on all lines
    let input = input(concat!(
        "prefix line 1\n",
        "prefix line 2\n",
        "{\n",
        "\"level\":\"info\",\n",
        "\"msg\":\"test\"\n",
        "}\n",
        "remainder line 1\n",
        "remainder line 2\n",
    ));
    let mut output = Vec::new();
    let mut opts = options();
    opts.delimiter = Delimiter::PrettyCompatible;
    opts.allow_prefix = true;
    opts.raw = true;
    opts.input_info = InputInfo::Minimal.into();
    let app = App::new(opts);
    app.run(vec![input], &mut output).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let lines: Vec<&str> = output_str.lines().collect();

    // Verify all prefix lines are present (data loss bug)
    assert!(output_str.contains("prefix line 1"), "should contain 'prefix line 1'");
    assert!(output_str.contains("prefix line 2"), "should contain 'prefix line 2'");

    // Verify all remainder lines are present
    assert!(
        output_str.contains("remainder line 1"),
        "should contain 'remainder line 1'"
    );
    assert!(
        output_str.contains("remainder line 2"),
        "should contain 'remainder line 2'"
    );

    // Verify JSON content is present in raw mode
    assert!(output_str.contains("{"), "should contain opening brace");
    assert!(output_str.contains("\"level\":\"info\""), "should contain level field");
    assert!(output_str.contains("\"msg\":\"test\""), "should contain msg field");
    assert!(output_str.contains("}"), "should contain closing brace");

    // All non-empty lines should have input badges (raw output bug)
    for (i, line) in lines.iter().enumerate() {
        if !line.is_empty() {
            assert!(line.starts_with("#0"), "line {} should have input badge: {:?}", i, line);
        }
    }

    // Verify we have enough lines for all content
    let non_empty_lines: Vec<&str> = lines.iter().filter(|l| !l.is_empty()).copied().collect();
    assert!(
        non_empty_lines.len() >= 8,
        "should have at least 8 non-empty lines (2 prefix + 4 JSON + 2 remainder), got: {}",
        non_empty_lines.len()
    );
}

// --- merge_segments tests ---

#[test]
fn test_merge_segments_single_line() {
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Buffer: 2 bytes placeholder + content
    let buf = b"  hello world".to_vec();
    let index = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf.len(),
            ts: ts(100, 0),
        }],
    };
    txo.send((0, buf, index)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    assert_eq!(result, "SShello world\n");
}

#[test]
fn test_merge_segments_ordered_timestamps() {
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Send two lines with ascending timestamps from same source
    let buf1 = b"  line-one".to_vec();
    let index1 = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf1.len(),
            ts: ts(100, 0),
        }],
    };

    let buf2 = b"  line-two".to_vec();
    let index2 = TimestampIndex {
        block: 1,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf2.len(),
            ts: ts(200, 0),
        }],
    };

    txo.send((0, buf1, index1)).unwrap();
    txo.send((0, buf2, index2)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    assert_eq!(result, "SSline-one\nSSline-two\n");
}

#[test]
fn test_merge_segments_interleaved_sources() {
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Source 0: ts=200, Source 1: ts=100 → output should be sorted by ts
    let buf0 = b"  from-source-0".to_vec();
    let index0 = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf0.len(),
            ts: ts(200, 0),
        }],
    };

    let buf1 = b"  from-source-1".to_vec();
    let index1 = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf1.len(),
            ts: ts(100, 0),
        }],
    };

    txo.send((0, buf0, index0)).unwrap();
    txo.send((1, buf1, index1)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    // Source 1 (ts=100) should come before Source 0 (ts=200)
    assert_eq!(result, "SSfrom-source-1\nSSfrom-source-0\n");
}

#[test]
fn test_merge_segments_out_of_order_shows_failed() {
    let app = App::new(Options {
        sync_interval: Duration::ZERO,
        ..options()
    });
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // First batch: ts=200
    let buf1 = b"  late".to_vec();
    let index1 = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf1.len(),
            ts: ts(200, 0),
        }],
    };
    txo.send((0, buf1, index1)).unwrap();

    // Second batch: ts=100 (arrives after ts=200 has been flushed)
    let buf2 = b"  early".to_vec();
    let index2 = TimestampIndex {
        block: 1,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf2.len(),
            ts: ts(100, 0),
        }],
    };
    txo.send((0, buf2, index2)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    // ts=200 flushed first (synced), then ts=100 arrives and gets flushed (failed because 200 > 100)
    assert_eq!(result, "SSlate\nFFearly\n");
}

#[test]
fn test_merge_segments_empty_channel() {
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded::<(usize, Vec<u8>, TimestampIndex)>(10);
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    assert!(output.is_empty());
}

#[test]
fn test_merge_segments_flushes_window_when_inputs_close() {
    // With a sync interval far longer than the test, the buffered line can only
    // be written because the input channel was closed, not because it timed out.
    let app = App::new(Options {
        sync_interval: Duration::from_secs(3600),
        ..options()
    });
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    let buf = b"  hello".to_vec();
    let index = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..buf.len(),
            ts: ts(100, 0),
        }],
    };
    txo.send((0, buf, index)).unwrap();
    drop(txo);

    let (txr, rxr) = channel::bounded(1);
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let result = app.merge_segments(&badges, rxo, &mut output, 1);
        txr.send(result.map(|_| output)).ok();
    });

    let output = rxr
        .recv_timeout(Duration::from_secs(10))
        .expect("merge_segments did not return after inputs closed")
        .unwrap();
    assert_eq!(String::from_utf8(output).unwrap(), "SShello\n");
}

#[rstest]
#[case::record(true, b"SShello\0")]
#[case::unsorted_text(false, b"  hello\0")]
fn test_merge_segments_flushes_output_before_waiting(#[case] record: bool, #[case] expected: &[u8]) {
    // The input channel stays open, so the entry can reach the output only
    // through a flush made before waiting for more input.
    let app = App::new(Options {
        sync_interval: Duration::ZERO,
        output_delimiter: "\0".into(),
        ..options()
    });
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);
    let (txf, rxf) = channel::unbounded();

    let buf = b"  hello".to_vec();
    let mut index = TimestampIndex::new(0);
    if record {
        index.lines.push(TimestampIndexLine {
            location: 0..buf.len(),
            ts: ts(100, 0),
        });
    }
    txo.send((0, buf, index)).unwrap();

    let merger = std::thread::spawn(move || {
        let mut output = FlushProbe {
            pending: Vec::new(),
            flushed: txf,
        };
        app.merge_segments(&badges, rxo, &mut output, 1)
    });

    let flushed = rxf
        .recv_timeout(Duration::from_secs(10))
        .expect("output was not flushed while inputs were open");
    assert_eq!(flushed, expected);

    drop(txo);
    merger.join().unwrap().unwrap();
}

#[test]
fn test_flush_wait() {
    let interval = Duration::from_secs(5);
    let received = Instant::now();

    assert_eq!(flush_wait(received, received, interval), interval);
    assert_eq!(
        flush_wait(received, received + Duration::from_secs(2), interval),
        Duration::from_secs(3)
    );
    assert_eq!(flush_wait(received, received + interval, interval), Duration::ZERO);
    assert_eq!(
        flush_wait(received, received + Duration::from_secs(6), interval),
        Duration::ZERO
    );
    // A clock reading earlier than the receive time must not underflow.
    assert_eq!(
        flush_wait(received + Duration::from_secs(1), received, interval),
        interval
    );
}

#[test]
fn test_merge_segments_multiple_lines_per_segment() {
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Buffer with two lines separated by delimiter
    let buf = b"  line-A\n  line-B".to_vec();
    let index = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![
            TimestampIndexLine {
                location: 0..8, // "  line-A"
                ts: ts(200, 0),
            },
            TimestampIndexLine {
                location: 9..17, // "  line-B"
                ts: ts(100, 0),
            },
        ],
    };
    txo.send((0, buf, index)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    // BTreeMap sorts by timestamp, so ts=100 (line-B) comes first
    assert_eq!(result, "SSline-B\nSSline-A\n");
}

#[test]
fn test_merge_segments_text_stays_with_record_before_it() {
    // Text after a record, such as a stack trace, is sorted by that record's
    // timestamp, so a record of another input does not come in between.
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Buffer layout: "  line-A\n  trace\n  line-C"
    let buf0 = b"  line-A\n  trace\n  line-C".to_vec();
    let index0 = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![
            TimestampIndexLine {
                location: 0..8, // "  line-A"
                ts: ts(100, 0),
            },
            TimestampIndexLine {
                location: 17..25, // "  line-C"
                ts: ts(300, 0),
            },
        ],
    };
    let buf1 = b"  line-B".to_vec();
    let index1 = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..8,
            ts: ts(200, 0),
        }],
    };
    txo.send((0, buf0, index0)).unwrap();
    txo.send((1, buf1, index1)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    assert_eq!(result, "SSline-A\nSStrace\nSSline-B\nSSline-C\n");
}

#[test]
fn test_merge_segments_gap_without_timestamp_direct_output() {
    // When there's no timestamp for a gap (no indexed lines, no source history),
    // the gap is written directly to output.
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Buffer with no indexed lines → entire buffer is a trailing gap with no timestamp
    let buf = b"unparsed text\n".to_vec();
    let index = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![],
    };
    txo.send((0, buf, index)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    // No timestamp available → gap written directly (raw, no sync indicator stripping)
    assert_eq!(result, "unparsed text\n");
}

#[test]
fn test_merge_segments_gap_trimming_no_trailing_delimiter() {
    // Text with no record before it is written as is, ended by the output
    // delimiter even if it does not end with one.
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Buffer: "  gap-no-delim  line-A"
    // Gap region: 0..14 ("  gap-no-delim"), no trailing \n before the indexed line
    let buf = b"  gap-no-delim  line-A".to_vec();
    let index = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 14..22, // "  line-A"
            ts: ts(100, 0),
        }],
    };
    txo.send((0, buf, index)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    assert_eq!(result, "  gap-no-delim\nSSline-A\n");
}

#[test]
fn test_merge_segments_empty_gap_skipped() {
    // When indexed lines are adjacent (no gap between them), start >= end → continue
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // Two adjacent indexed lines with only a delimiter between them
    // Buffer: "  line-A\n  line-B"
    let buf = b"  line-A\n  line-B".to_vec();
    let index = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![
            TimestampIndexLine {
                location: 0..8, // "  line-A"
                ts: ts(100, 0),
            },
            TimestampIndexLine {
                location: 9..17, // "  line-B"
                ts: ts(200, 0),
            },
        ],
    };
    txo.send((0, buf, index)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    // Gap between lines: start = 0+8+1 = 9, end = 9 → start >= end, skipped
    // Only the two indexed lines appear
    assert_eq!(result, "SSline-A\nSSline-B\n");
}

#[test]
fn test_merge_segments_trailing_gap_with_source_history() {
    // Trailing gap (after last indexed line) is sorted with the record before it
    let app = App::new(options());
    let badges = test_badges(2);
    let (txo, rxo) = channel::bounded(10);

    // First segment establishes source_last_ts for source 0
    let buf1 = b"  line-A".to_vec();
    let index1 = TimestampIndex {
        block: 0,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..8,
            ts: ts(100, 0),
        }],
    };

    // Second segment from same source has an indexed line + trailing gap
    // Buffer: "  line-B\n  trailing-gap"
    let buf2 = b"  line-B\n  trailing-gap".to_vec();
    let index2 = TimestampIndex {
        block: 1,
        part: None,
        lines: vec![TimestampIndexLine {
            location: 0..8, // "  line-B"
            ts: ts(200, 0),
        }],
    };

    txo.send((0, buf1, index1)).unwrap();
    txo.send((0, buf2, index2)).unwrap();
    drop(txo);

    let mut output = Vec::new();
    app.merge_segments(&badges, rxo, &mut output, 1).unwrap();

    let result = String::from_utf8(output).unwrap();
    // The trailing gap is sorted by line-B (ts=200), the record right before it.
    assert_eq!(result, "SSline-A\nSSline-B\nSStrailing-gap\n");
}

#[test]
fn test_follow_merges_inputs_by_time() {
    let app = App::new(Options {
        follow: true,
        ..options()
    });
    let record = |sec, msg| format!(r#"{{"ts":"2026-07-06T10:00:0{sec}Z","msg":"{msg}"}}"#);
    let inputs = vec![
        input(format!("{}\n{}\n", record(0, "a0"), record(2, "a2"))),
        input(format!("{}\n{}\n", record(1, "b1"), record(3, "b3"))),
    ];

    let mut output = Vec::new();
    app.run(inputs, &mut output).unwrap();

    assert_eq!(
        String::from_utf8(output).unwrap(),
        concat!(
            "#0 | <stdin> | 2026-07-06 10:00:00.000 a0\n",
            "#1 | <stdin> | 2026-07-06 10:00:01.000 b1\n",
            "#0 | <stdin> | 2026-07-06 10:00:02.000 a2\n",
            "#1 | <stdin> | 2026-07-06 10:00:03.000 b3\n",
        )
    );
}

#[rstest]
#[case::kept_together(
    vec![
        record_block(0, 0, "  rec-a", 100),
        part_block(0, 1, "  xx", PartialPlacement::First),
        record_block(1, 0, "  rec-b", 200),
        part_block(0, 2, "yy", PartialPlacement::Last),
    ],
    "SSrec-a\nSSxxyy\nSSrec-b\n",
)]
#[case::taken_in_block_order(
    vec![
        part_block(0, 1, "  xx", PartialPlacement::First),
        part_block(0, 3, "zz", PartialPlacement::Last),
        record_block(0, 0, "  rec-a", 100),
        part_block(0, 2, "yy", PartialPlacement::Next),
    ],
    "SSrec-a\nSSxxyyzz\n",
)]
#[case::sorted_after_record_before(
    vec![
        record_block(0, 0, "  rec-a", 300),
        part_block(0, 1, "  xx", PartialPlacement::First),
        part_block(0, 2, "yy", PartialPlacement::Last),
        record_block(1, 0, "  rec-b", 200),
    ],
    "SSrec-a\nSSxxyy\nFFrec-b\n",
)]
#[case::ended_by_next_block(
    vec![
        record_block(0, 0, "  rec-a", 100),
        part_block(0, 1, "  xx", PartialPlacement::First),
        record_block(0, 2, "  rec-b", 100),
    ],
    "SSrec-a\nSSxx\nSSrec-b\n",
)]
#[case::ended_when_inputs_close(
    vec![
        record_block(0, 0, "  rec-a", 100),
        part_block(0, 1, "  xx", PartialPlacement::First),
    ],
    "SSrec-a\nSSxx\n",
)]
#[case::written_as_they_come_without_timestamp(
    vec![
        part_block(0, 0, "  xx", PartialPlacement::First),
        record_block(1, 0, "  rec-b", 100),
        text_block(2, 0, "text\n"),
        part_block(0, 1, "yy", PartialPlacement::Last),
    ],
    "  xxyy\ntext\nSSrec-b\n",
)]
#[case::held_while_other_entry_is_open(
    vec![
        record_block(0, 0, "  rec-a", 100),
        part_block(0, 1, "  aa", PartialPlacement::First),
        part_block(1, 0, "  xx", PartialPlacement::First),
        part_block(1, 1, "yy", PartialPlacement::Last),
        part_block(0, 2, "bb", PartialPlacement::Last),
    ],
    "SSrec-a\nSSaabb\n  xxyy\n",
)]
fn test_merge_segments_long_entry(#[case] messages: Vec<Processed>, #[case] expected: &str) {
    // The sync interval is far longer than the test, so a split entry can only end early
    // because its next part will not come.
    assert_eq!(merge(Duration::from_secs(3600), messages), expected);
}

#[test]
fn test_merge_segments_long_entry_ended_when_rest_is_late() {
    // With a zero sync interval, the rest of the entry is late at once,
    // so the entry is ended and the record of the other input is written.
    let messages = vec![
        record_block(0, 0, "  rec-a", 100),
        part_block(0, 1, "  xx", PartialPlacement::First),
        record_block(1, 0, "  rec-b", 200),
        part_block(0, 2, "yy", PartialPlacement::Last),
    ];
    assert_eq!(merge(Duration::ZERO, messages), "SSrec-a\nSSxx\nSSrec-b\nyy\n");
}

#[test]
fn test_follow_shows_long_entry_between_records() {
    // The entry is larger than the maximum message size, so it is read in several parts.
    let app = App::new(Options {
        follow: true,
        buffer_size: NonZeroUsize::new(1024).unwrap(),
        max_message_size: NonZeroUsize::new(2048).unwrap(),
        ..options()
    });
    let long = "x".repeat(5000);
    let data = format!(
        "{}\n{long}\n{}\n",
        r#"{"ts":"2026-07-06T10:00:00Z","msg":"before"}"#, r#"{"ts":"2026-07-06T10:00:01Z","msg":"after"}"#,
    );

    let mut output = Vec::new();
    app.run(vec![input(data)], &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert_eq!(
        output,
        format!("2026-07-06 10:00:00.000 before\n{long}\n2026-07-06 10:00:01.000 after\n")
    );
}

// --- BlockScanner tests ---

#[test]
fn test_block_scanner_continues_numbering_across_scans() {
    // In follow mode a fresh scan starts on every filesystem event. Block numbers
    // must keep growing across those scans, otherwise lines of different events
    // collide in the merge window key and all but the last are dropped.
    //
    // Segments are sized so that each of these lines lands in a segment of its
    // own, which makes numbering observable within a scan as well as across scans.
    const SEGMENT_SIZE: usize = 3;

    let sfi = Arc::new(SegmentBufFactory::new(SEGMENT_SIZE));
    let scanner = Scanner::new(sfi, Delimiter::default());
    let (txi, rxi) = channel::unbounded();

    let mut blocks = BlockScanner::new(&scanner, Delimiter::default(), SEGMENT_SIZE);
    blocks
        .scan(&mut Cursor::new(b"a1\na2\n".to_vec()), 0, &txi, true)
        .unwrap();
    blocks
        .scan(&mut Cursor::new(b"b1\nb2\n".to_vec()), 0, &txi, true)
        .unwrap();
    drop(txi);

    let text = |segment: &Segment| match segment {
        Segment::Complete(buf) | Segment::Incomplete(buf, _) => String::from_utf8_lossy(buf.data()).into_owned(),
    };
    let scanned = rxi
        .iter()
        .map(|(_, block, offset, segment)| (block, offset, text(&segment)))
        .collect_vec();

    assert_eq!(
        scanned,
        vec![
            (0, 0, "a1\n".to_owned()),
            (1, 3, "a2\n".to_owned()),
            (2, 6, "b1\n".to_owned()),
            (3, 9, "b2\n".to_owned()),
        ]
    );
}

#[test]
fn test_block_scanner_stops_scanning_when_receiver_is_gone() {
    // Once the processing threads are gone there is nobody left to hand segments
    // to, so scanning stops instead of draining the rest of the input.
    const SEGMENT_SIZE: usize = 3;

    let sfi = Arc::new(SegmentBufFactory::new(SEGMENT_SIZE));
    let scanner = Scanner::new(sfi, Delimiter::default());
    let (txi, rxi) = channel::unbounded::<(usize, usize, u64, Segment)>();
    drop(rxi);

    let mut blocks = BlockScanner::new(&scanner, Delimiter::default(), SEGMENT_SIZE);
    let mut input = Cursor::new(b"a1\na2\na3\na4\n".to_vec());
    blocks.scan(&mut input, 0, &txi, false).unwrap();

    // Only the first segment was read, and its failed hand-off was not counted.
    assert_eq!(input.position(), SEGMENT_SIZE as u64);
    assert_eq!(blocks.block, 0);
}

#[test]
fn test_block_scanner_completes_line_split_across_scans() {
    // A writer may append a single line in several writes, each of them
    // triggering a separate scan. The unterminated end of the first write must
    // wait for the rest of its line instead of being sent as a line of its own.
    let scanned = scan_chunks(
        1024,
        &[
            (br#"{"msg":"a"}"#.as_slice(), true),
            (b"\n{\"msg\":\"hel", true),
            (b"lo\"}\n", true),
        ],
    );

    assert_eq!(
        scanned,
        vec![
            vec![],
            vec!["{\"msg\":\"a\"}\n".to_owned()],
            vec!["{\"msg\":\"hello\"}\n".to_owned()],
        ]
    );
}

#[test]
fn test_block_scanner_flushes_held_tail() {
    // When the input is replaced or ends, the held tail will not be completed
    // and is sent as it is.
    let scanned = scan_chunks(1024, &[(b"a1\na2", true), (b"", false)]);

    assert_eq!(scanned, vec![vec!["a1\n".to_owned()], vec!["a2".to_owned()]]);
}

#[test]
fn test_block_scanner_passes_overlong_tail() {
    // An unterminated tail is not held back once it reaches the maximum segment
    // size, as it would not fit in a single segment anyway.
    let scanned = scan_chunks(4, &[(b"abcdefgh", true), (b"", true)]);

    assert_eq!(scanned[0].concat(), "abcdefgh");
    assert!(scanned[1].is_empty());
}

#[test]
fn test_block_scanner_propagates_read_error() {
    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("read error"))
        }
    }

    let sfi = Arc::new(SegmentBufFactory::new(1024));
    let scanner = Scanner::new(sfi, Delimiter::default());
    let (txi, _rxi) = channel::unbounded();
    let mut blocks = BlockScanner::new(&scanner, Delimiter::default(), 1024);

    for hold_tail in [true, false] {
        assert!(blocks.scan(&mut FailingReader, 0, &txi, hold_tail).is_err());
    }
    assert_eq!(blocks.block, 0);
}

#[test]
fn test_cat_passes_record_origin() {
    let (app, capture) = app_with_origin_capture(options().with_input_info(InputInfo::None.into()));

    let inputs = vec![
        file_input("a.log", "msg=a1\nmsg=a2\n"),
        file_input("b.log", "msg=b1\n"),
        input("msg=c1\n"),
    ];
    let mut output = Vec::new();
    app.run(inputs, &mut output).unwrap();

    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "msg=a1\nmsg=a2\nmsg=b1\nmsg=c1\n"
    );
    assert_eq!(
        *capture.origins.lock().unwrap(),
        vec![
            (descriptor(0, "a.log"), 0..6),
            (descriptor(0, "a.log"), 7..13),
            (descriptor(1, "b.log"), 0..6),
            (descriptor(2, "<stdin>"), 0..6),
        ]
    );
}

#[test]
fn test_cat_passes_record_origin_across_segments() {
    // A small buffer splits the input into several segments, so the offsets of
    // later records include the sizes of the segments before them.
    let (app, capture) = app_with_origin_capture(Options {
        buffer_size: NonZeroUsize::new(8).unwrap(),
        ..options()
    });

    let mut output = Vec::new();
    app.run(vec![input("msg=a1\nmsg=a2\nmsg=a3\n")], &mut output).unwrap();

    assert_eq!(
        *capture.origins.lock().unwrap(),
        vec![
            (descriptor(0, "<stdin>"), 0..6),
            (descriptor(0, "<stdin>"), 7..13),
            (descriptor(0, "<stdin>"), 14..20),
        ]
    );
}

#[test]
fn test_sort_passes_record_origin() {
    let (app, capture) = app_with_origin_capture(options().with_sort(true));

    const M1: &str = r#"{"ts":"2024-01-25T19:10:01Z","msg":"m1"}"#;
    const M2: &str = r#"{"ts":"2024-01-25T19:10:02Z","msg":"m2"}"#;
    const M3: &str = r#"{"ts":"2024-01-25T19:10:03Z","msg":"m3"}"#;
    let inputs = vec![input(format!("{M1}\n{M3}\n")), input(format!("{M2}\n"))];
    let mut output = Vec::new();
    app.run(inputs, &mut output).unwrap();

    let n = M1.len() as u64;
    let mut origins = capture.origins.lock().unwrap().clone();
    origins.sort_by_key(|(input, range)| (input.index, range.start));
    assert_eq!(
        origins,
        vec![
            (descriptor(0, "<stdin>"), 0..n),
            (descriptor(0, "<stdin>"), n + 1..2 * n + 1),
            (descriptor(1, "<stdin>"), 0..n),
        ]
    );
}

#[test]
fn test_process_segments_passes_record_origin() {
    let (app, capture) = app_with_origin_capture(options());

    let parser = app.parser();
    let bfo = BufFactory::new(4096);
    let sfi = SegmentBufFactory::new(4096);
    let badges = test_badges(2);
    let inputs = [descriptor(0, "a.log"), descriptor(1, "b.log")];

    let (txi, rxi) = channel::bounded(10);
    let (txo, rxo) = channel::bounded(10);

    const A1: &str = r#"{"ts":"2024-01-25T19:10:02Z","msg":"a1"}"#;
    const B1: &str = r#"{"ts":"2024-01-25T19:10:01Z","msg":"b1"}"#;
    let segment = |data: &str| Segment::Complete(SegmentBuf::from(data.as_bytes()));
    txi.send((1, 0, 100, segment(B1))).unwrap();
    txi.send((0, 0, 200, segment(A1))).unwrap();
    drop(txi);

    app.process_segments(&parser, &bfo, &sfi, &badges, &inputs, rxi, txo);

    let n = A1.len() as u64;
    assert_eq!(rxo.iter().count(), 2);
    assert_eq!(
        *capture.origins.lock().unwrap(),
        vec![
            (descriptor(1, "b.log"), 100..100 + n),
            (descriptor(0, "a.log"), 200..200 + n),
        ]
    );
}

#[test]
fn test_origin_slice() {
    let input = descriptor(0, "a.log");
    let origin = Origin::new(&input, 100, 20);
    assert_eq!(origin.range, 100..120);

    let part = origin.slice(5..8);
    assert_eq!(part.input, &input);
    assert_eq!(part.range, 105..108);
}

#[test]
fn test_input_reference_name() {
    assert_eq!(InputReference::Stdin.name(), "<stdin>");
    let path = crate::input::InputPath::new("logs/a.log".into(), "/var/logs/a.log".into());
    assert_eq!(InputReference::File(path).name(), "logs/a.log");
}

#[test]
fn test_compact_input_names() {
    let app = App::new(options());
    let p = &app.punctuation;
    let names = |names: &[&str]| {
        app.compact_input_names(names)
            .into_iter()
            .map(|name| name.concat())
            .collect_vec()
    };

    // Short names without a long common prefix stay as they are.
    assert_eq!(names(&["a.log", "b.log"]), vec!["a.log", "b.log"]);

    // A long common prefix is replaced with the common-part marker, keeping 4 of its last characters.
    assert_eq!(
        names(&["/var/log/app/a.log", "/var/log/app/b.log"]),
        vec![
            format!("{}app/a.log", p.input_name_common_part),
            format!("{}app/b.log", p.input_name_common_part),
        ]
    );

    // A long name after a long common prefix is also clipped in the middle.
    let long = "/var/log/app/0123456789abcdefghijklmnopqrstuvwxyz.log";
    assert_eq!(
        names(&[long, "/var/log/app/b.log"]),
        vec![
            format!(
                "{}app/01234567{}opqrstuvwxyz.log",
                p.input_name_common_part, p.input_name_clipping
            ),
            format!("{}app/b.log", p.input_name_common_part),
        ]
    );

    // A long name with a short common prefix is clipped in the middle only.
    let long = "0123456789abcdefghijklmnopqrstuvwxyz.log";
    assert_eq!(
        names(&[long, "b.log"]),
        vec![
            format!("0123456789ab{}stuvwxyz.log", p.input_name_clipping),
            "b.log".to_owned(),
        ]
    );
}

#[test]
fn test_block_scanner_start_sets_offsets_only() {
    // A new stream, such as a replaced file or a file seeked to its tail, has its
    // own offsets, while block numbers keep counting to stay unique for the input.
    let sfi = Arc::new(SegmentBufFactory::new(3));
    let scanner = Scanner::new(sfi, Delimiter::default());
    let (txi, rxi) = channel::unbounded();
    let mut blocks = BlockScanner::new(&scanner, Delimiter::default(), 3);

    blocks
        .scan(&mut Cursor::new(b"a1\na2\n".to_vec()), 0, &txi, false)
        .unwrap();
    blocks.start(0);
    blocks.scan(&mut Cursor::new(b"b1\n".to_vec()), 0, &txi, false).unwrap();
    blocks.start(100);
    blocks.scan(&mut Cursor::new(b"c1\n".to_vec()), 0, &txi, false).unwrap();
    drop(txi);

    let scanned = rxi.iter().map(|(_, block, offset, _)| (block, offset)).collect_vec();
    assert_eq!(scanned, vec![(0, 0), (1, 3), (2, 0), (3, 100)]);
}

#[test]
fn test_sort_passes_record_origin_across_blocks() {
    // A small buffer splits the input into several index blocks, so the offsets
    // of later records include the block offsets.
    let (app, capture) = app_with_origin_capture(Options {
        buffer_size: NonZeroUsize::new(64).unwrap(),
        ..options().with_sort(true)
    });

    let lines = (1..=4)
        .map(|i| format!(r#"{{"ts":"2024-01-25T19:10:0{i}Z","msg":"m{i}"}}"#))
        .collect_vec();
    let mut output = Vec::new();
    app.run(
        vec![input(lines.iter().map(|x| format!("{x}\n")).join(""))],
        &mut output,
    )
    .unwrap();

    let n = lines[0].len() as u64;
    let mut ranges = capture
        .origins
        .lock()
        .unwrap()
        .iter()
        .map(|x| x.1.clone())
        .collect_vec();
    ranges.sort_by_key(|x| x.start);
    assert_eq!(ranges, (0..4).map(|i| i * (n + 1)..i * (n + 1) + n).collect_vec());
}
