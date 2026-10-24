use super::*;

use crate::testing::diagnostics::CaptureSink;

fn inputs() -> Vec<InputDescriptor> {
    vec![InputDescriptor::new(0, "a.log"), InputDescriptor::new(1, "b.log")]
}

fn diagnostics(max_level: Level) -> (Diagnostics, Arc<CaptureSink>) {
    let sink = Arc::new(CaptureSink::new(max_level));
    (Diagnostics::new(sink.clone(), &inputs()), sink)
}

fn preview(content: &[u8]) -> String {
    Preview(content).to_string()
}

#[test]
fn test_report_dropped() {
    let (diagnostics, sink) = diagnostics(Level::Debug);
    let inputs = inputs();
    diagnostics.report_dropped(b"panic: boom", &Origin::new(&inputs[1], 87, 11), DropKind::Unparsed);
    diagnostics.report_dropped(b"tail", &Origin::new(&inputs[0], 10, 4), DropKind::Remainder);
    diagnostics.report_dropped(b"2026-07-06 ", &Origin::new(&inputs[1], 120, 11), DropKind::Prefix);
    assert_eq!(
        sink.entries(),
        [
            (
                Level::Debug,
                r#"b.log: dropped unparsed line at 87..98: "panic: boom""#.to_owned()
            ),
            (
                Level::Debug,
                r#"a.log: dropped unparsed text after a record at 10..14: "tail""#.to_owned()
            ),
            (
                Level::Debug,
                r#"b.log: dropped prefix before a record at 120..131: "2026-07-06 ""#.to_owned()
            ),
        ]
    );
}

#[test]
fn test_counts_without_debug_entries() {
    let (diagnostics, sink) = diagnostics(Level::Info);
    let inputs = inputs();
    for _ in 0..3 {
        diagnostics.report_dropped(b"x", &Origin::new(&inputs[1], 0, 1), DropKind::Unparsed);
    }
    diagnostics.report_dropped(b"x", &Origin::new(&inputs[0], 0, 1), DropKind::Unparsed);
    assert_eq!(sink.entries(), []);
    diagnostics.write_summary();
    assert_eq!(
        sink.entries(),
        [
            (Level::Info, "a.log: dropped 1 piece of unparsed text".to_owned()),
            (Level::Info, "b.log: dropped 3 pieces of unparsed text".to_owned()),
        ]
    );
}

#[test]
fn test_summary_lists_only_inputs_with_drops() {
    let (diagnostics, sink) = diagnostics(Level::Info);
    let inputs = inputs();
    diagnostics.report_dropped(b"x", &Origin::new(&inputs[1], 0, 1), DropKind::Prefix);
    diagnostics.write_summary();
    assert_eq!(
        sink.entries(),
        [(Level::Info, "b.log: dropped 1 piece of unparsed text".to_owned())]
    );
}

#[test]
fn test_summary_is_written_once() {
    let (diagnostics, sink) = diagnostics(Level::Info);
    let inputs = inputs();
    diagnostics.report_dropped(b"x", &Origin::new(&inputs[0], 0, 1), DropKind::Unparsed);
    diagnostics.write_summary();
    diagnostics.report_dropped(b"x", &Origin::new(&inputs[0], 1, 1), DropKind::Unparsed);
    diagnostics.write_summary();
    assert_eq!(
        sink.entries(),
        [(Level::Info, "a.log: dropped 1 piece of unparsed text".to_owned())]
    );
}

#[test]
fn test_summary_without_info_entries() {
    let (diagnostics, sink) = diagnostics(Level::Warn);
    let inputs = inputs();
    diagnostics.report_dropped(b"x", &Origin::new(&inputs[0], 0, 1), DropKind::Unparsed);
    diagnostics.write_summary();
    assert_eq!(sink.entries(), []);
}

#[test]
fn test_preview() {
    assert_eq!(preview(b""), r#""""#);
    assert_eq!(preview(b"say \"hi\" \\ bye"), r#""say \"hi\" \\ bye""#);
    assert_eq!(preview(b"\tat main\r\x1b[1m"), r#""\tat main\r\u{1b}[1m""#);
    assert_eq!(preview(b"bad \xff\xfe utf-8"), "\"bad \u{FFFD}\u{FFFD} utf-8\"");
    assert_eq!(preview("ünïcødé".as_bytes()), "\"ünïcødé\"");
}

#[test]
fn test_preview_is_cut() {
    let exact = "a".repeat(PREVIEW_LEN);
    assert_eq!(preview(exact.as_bytes()), format!("\"{exact}\""));

    let long = "a".repeat(PREVIEW_LEN + 1);
    assert_eq!(preview(long.as_bytes()), format!("\"{exact}\"…"));

    // The cut is at a character boundary, counting characters rather than bytes.
    let wide = "ж".repeat(PREVIEW_LEN + 5);
    assert_eq!(preview(wide.as_bytes()), format!("\"{}\"…", "ж".repeat(PREVIEW_LEN)));
}

#[test]
fn test_record_diagnostics() {
    let inputs = inputs();
    let origin = Origin::new(&inputs[0], 5, 20);

    let sink = CaptureSink::new(Level::Info);
    assert!(RecordDiagnostics::new(&sink, &origin).is_none());

    let sink = CaptureSink::new(Level::Debug);
    let diagnostics = RecordDiagnostics::new(&sink, &origin).unwrap();
    diagnostics.report_repeat("tag");
    diagnostics.report_collision("message");
    assert_eq!(
        sink.entries(),
        [
            (
                Level::Debug,
                r#"a.log: dropped repeated key "tag" of the record at 5..25, keeping its last value"#.to_owned()
            ),
            (
                Level::Debug,
                r#"a.log: dropped field "message" of the record at 5..25, its key is taken by a predefined field"#
                    .to_owned()
            ),
        ]
    );
}
