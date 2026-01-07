// Comprehensive rstest-based tests for JsonDelimitSearcher
//
// The JsonDelimitSearcher is responsible for finding newline delimiters between top-level
// JSON objects, where a `}` must end a line and a `{` must start a line.
//
// Test coverage includes:
// 1. Basic delimiter detection between JSON objects
// 2. Edge mode handling (delimiters at buffer boundaries)
// 3. Partial match semantics (cross-boundary delimiter detection)
// 4. Multiple newlines and whitespace handling
// 5. CRLF line endings
// 6. Nested JSON objects (only top-level boundaries)
// 7. Non-JSON content between objects
// 8. Array rejection (commas, brackets)
// 9. Whitespace-only sections
// 10. Mixed content scenarios

use std::ops::Range;

use const_str::concat_bytes;
use rstest::rstest;

use super::JsonDelimitSearcher;
use crate::scanning::{Search, SearchExt};

// search_l tests
#[rstest]
#[case(b"{\"a\":1}\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\n\n  \n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\n\t\n{\"b\":2}", false, Some(7..8))]
fn test_json_search_l_basic(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"{\"a\":1}\n\n", true, Some(7..8))]
#[case(b"{\"a\":1}\n\n", false, None)]
#[case(b"{\"a\":1}  {\"b\":2}", false, None)]
#[case(b"{\"a\":1}   {\"b\":2}", false, None)]
fn test_json_search_l_edge_cases(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"{\"a\":1\n{\"b\":2}", false, Some(6..7))]
#[case(b"{\"a\":1}", false, None)]
#[case(b"", false, None)]
#[case(b"{", false, None)]
#[case(b"}", false, None)]
fn test_json_search_l_no_match(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// search_r tests
#[rstest]
#[case(b"{\"a\":1}\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\n\n  \n{\"b\":2}", false, Some(11..12))]
#[case(b"{\"a\":1}\n\t\n{\"b\":2}", false, Some(9..10))]
fn test_json_search_r_basic(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

#[rstest]
#[case(b"\n\n{\"b\":2}", true, Some(1..2))]
#[case(b"\n\n{\"b\":2}", false, None)]
#[case(b"{\"a\":1}", false, None)]
fn test_json_search_r_edge_cases(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

#[rstest]
#[case(b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}", false, Some(15..16))]
#[case(b"{\"a\":1}\n  \n{\"b\":2}\n\t{\"c\":3}", false, Some(18..20))]
fn test_json_search_r_multiple_objects(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

// partial_match_r tests
#[rstest]
#[case(b"{\"a\":1}\n\n", Some(7))]
#[case(b"{\"a\":1}  \n", Some(7))]
#[case(b"{\"a\":1}", Some(7))]
#[case(b"{\"a\":1}  ", Some(7))]
#[case(b"{\"a\":1}x", None)]
#[case(b"}", Some(1))]
#[case(b"}  ", Some(1))]
#[case(b"} \t\r\n", Some(1))]
#[case(b"}x", None)]
#[case(b"} x", None)]
#[case(b"}foo", None)]
#[case(b"no closing brace", None)]
#[case(b"", None)]
#[case(b"abc", None)]
#[case(b"   ", None)]
fn test_json_partial_match_r(#[case] input: &[u8], #[case] expected: Option<usize>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.partial_match_r(input), expected);
}

// partial_match_l tests
#[rstest]
#[case(b"\n\n{\"b\":2}", Some(2))]
#[case(b"  \n{\"b\":2}", Some(3))]
#[case(b"  \t{\"b\":2}", Some(3))]
#[case(b"\n\n{", Some(2))]
#[case(b"{", Some(0))]
#[case(b"", None)]
#[case(b" \t\r\n{", Some(4))]
#[case(b"   {", Some(3))]
#[case(b"x{", None)]
#[case(b"foo{", None)]
#[case(b"a {", None)]
#[case(b"no opening brace", None)]
#[case(b"abc", None)]
#[case(b"   ", None)]
fn test_json_partial_match_l(#[case] input: &[u8], #[case] expected: Option<usize>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.partial_match_l(input), expected);
}

// Nested objects tests
#[rstest]
#[case(b"{\"a\":{\"b\":1}}\n{\"c\":2}", false, Some(13..14))]
#[case(b"{\"a\":{\"b\":{\"c\":1}}}\n{\"d\":2}", false, Some(19..20))]
fn test_json_nested_objects(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// CRLF tests
#[rstest]
#[case(b"{\"a\":1}\r\n{\"b\":2}", false, Some(7..9))]
#[case(b"{\"a\":1}\r\n  \n{\"b\":2}", false, Some(7..9))]
#[case(b"}\r\n{", false, Some(1..3))]
#[case(b"}\r\n{", true, Some(1..3))]
fn test_json_crlf(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"}\r\n{", false, Some(1..3))]
#[case(b"}\r\n  {", false, Some(1..5))]
#[case(b"}\r\n\t{", false, Some(1..4))]
#[case(b"{", false, None)]
#[case(b",{", false, None)]
#[case(b"[{", false, None)]
fn test_json_crlf_search_r(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

// Non-JSON content tests
#[rstest]
#[case(b"{\"a\":1}\nx\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\nERROR: something failed\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\nERROR: line 1\nERROR: line 2\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"log\":\"data\"}\n  at Module.func (file.js:123:45)\n  at process._tickCallback\n{\"next\":\"log\"}", false, Some(14..15))]
fn test_json_with_non_json_content(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// Array rejection tests
#[rstest]
#[case(b"{\"a\":1}\n,\n{\"b\":2}", false, None)]
#[case(b"{\"a\":1}\n  ,  \n{\"b\":2}", false, None)]
#[case(b"{\"a\":1}\n]\n{\"b\":2}", false, Some(9..10))]
#[case(b"{\"a\":1}\n[\n{\"b\":2}", false, None)]
#[case(b",{", false, None)]
#[case(b"[{", false, None)]
#[case(b":{", false, None)]
#[case(b"{", false, None)]
fn test_json_rejects_array_elements(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"[{\"a\":1}\n,\n{\"b\":2}]", false, None)]
#[case(b"{\"arr\":[\n{\"nested\":1}\n]}", false, None)]
#[case(b"[{\"a\":1}\n,{\"b\":2}\n,{\"c\":3}]", false, None)]
fn test_json_rejects_nested_structures(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// Edge cases with non-JSON content
#[rstest]
#[case(b"Some preamble text\n{\"a\":1}", true, Some(18..19))]
#[case(b"{\"a\":1}\nSome epilogue text", true, Some(7..8))]
#[case(b"{\"a\":1}\nLine 1\nLine 2\nLine 3\n{\"b\":2}", false, Some(7..8))]
fn test_json_edge_cases_with_non_json(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// No newline between braces
#[rstest]
#[case(b"{\"a\":1}{\"b\":2}", false, None)]
#[case(b"{\"a\":1}   {\"b\":2}", false, None)]
#[case(b"{\"a\":1}\t\t{\"b\":2}", false, None)]
fn test_json_no_newline_between(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// Non-JSON content with structural characters
#[rstest]
#[case(b"{\"a\":1}\nERROR: found { in message\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\nERROR: found } in message\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\nERROR: found [foo] in message\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\nERROR: found, comma\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\nERROR: {foo}, [bar], etc.\n{\"b\":2}", false, Some(7..8))]
fn test_json_rejects_non_json_content(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// Whitespace-only tests
#[rstest]
#[case(b"{\"a\":1}\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\n  \n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\n\t\t\n{\"b\":2}", false, Some(7..8))]
#[case(b"{\"a\":1}\nx\n{\"b\":2}", false, Some(7..8))]
fn test_json_whitespace_only(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// Delimiter content verification
#[test]
fn test_json_delimiter_includes_newlines_correctly() {
    let searcher = JsonDelimitSearcher;

    let buf = b"{\"a\":1}\n\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    if let Some(range) = result {
        let delimiter = &buf[range.clone()];
        assert_eq!(delimiter, b"\n", "Should be single newline");
    }

    let buf = b"{\"a\":1}\nnon-JSON line\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));
}

// Pretty log case simulation
#[rstest]
#[case(b"}\nasd\n{", false, Some(1..2))]
#[case(b"}\nasd\n{", true, Some(1..2))]
fn test_json_pretty_log_case_search_l(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = JsonDelimitSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[test]
fn test_json_pretty_log_case_search_r() {
    let searcher = JsonDelimitSearcher;
    let buf = b"}\nasd\n{";
    let result = searcher.search_r(buf, true);
    assert_eq!(result, Some(5..6));
}

// Split tests
#[test]
fn test_json_split_basic() {
    let searcher = JsonDelimitSearcher;
    let buf = b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"{\"a\":1}"[..]));
    assert_eq!(iter.next(), Some(&b"{\"b\":2}"[..]));
    assert_eq!(iter.next(), Some(&b"{\"c\":3}"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_json_split_with_whitespace() {
    let searcher = JsonDelimitSearcher;
    let buf = b"{\"a\":1}\n  \n{\"b\":2}\n\t{\"c\":3}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"{\"a\":1}"[..]));
    assert_eq!(iter.next(), Some(&b"  "[..]));
    assert_eq!(iter.next(), Some(&b"{\"b\":2}"[..]));
    assert_eq!(iter.next(), Some(&b"\t{\"c\":3}"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_json_split_with_non_json() {
    let searcher = JsonDelimitSearcher;
    let buf = b"{\"a\":1}\nERROR\n{\"b\":2}\nWARNING\n{\"c\":3}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"{\"a\":1}"[..]));
    assert_eq!(iter.next(), Some(&b"ERROR"[..]));
    assert_eq!(iter.next(), Some(&b"{\"b\":2}"[..]));
    assert_eq!(iter.next(), Some(&b"WARNING"[..]));
    assert_eq!(iter.next(), Some(&b"{\"c\":3}"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_json_split_with_preamble() {
    let searcher = JsonDelimitSearcher;
    let buf = b"Starting log...\n{\"timestamp\":\"2024-01-01\",\"level\":\"info\"}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"Starting log..."[..]));
    assert_eq!(
        iter.next(),
        Some(&b"{\"timestamp\":\"2024-01-01\",\"level\":\"info\"}"[..])
    );
    assert_eq!(iter.next(), None);
}

// Full pretty log case
#[test]
fn test_json_full_pretty_log_case() {
    let searcher = JsonDelimitSearcher;
    let buf = b"{\"a\":1}\n{\"b\":2}\nasd\n{\"c\":3}\n";

    let entries: Vec<&[u8]> = searcher.split(buf).collect();

    assert_eq!(entries.len(), 4, "Should produce 4 separate entries");
    assert_eq!(entries[0], b"{\"a\":1}");
    assert_eq!(entries[1], b"{\"b\":2}");
    assert_eq!(entries[2], b"asd");
    assert_eq!(entries[3], b"{\"c\":3}");
}

// Cross-boundary scenarios
#[rstest]
#[case(b"{\"a\":1}\n", b"{\"b\":2}", b"{\"a\":1}\n{\"b\":2}")]
#[case(b"{\"a\":1}", b"\n{\"b\":2}", b"{\"a\":1}\n{\"b\":2}")]
fn test_json_cross_boundary_scenarios(#[case] left: &[u8], #[case] right: &[u8], #[case] combined: &[u8]) {
    let searcher = JsonDelimitSearcher;

    let partial_r = searcher.partial_match_r(left);
    let partial_l = searcher.partial_match_l(right);

    assert!(
        partial_r.is_some() || partial_l.is_some(),
        "Should have partial match at boundary"
    );

    let combined_result = searcher.search_l(combined, false);
    assert!(combined_result.is_some(), "Should find delimiter across boundary");
}

// Test for multi-line JSON where closing brace is not on its own line
// This format should work with --input-format json but currently fails
#[test]
fn test_json_multiline_with_indented_continuation() {
    let searcher = JsonDelimitSearcher;
    let buf = concat_bytes!(
        concat_bytes!(br#"{"timestamp":"2024-01-01T00:00:00Z","level":"info","#, "\n"),
        concat_bytes!(br#"  "message":"first"}"#, "\n"),
        concat_bytes!(br#"{"timestamp":"2024-01-01T00:00:01Z","level":"warn","#, "\n"),
        concat_bytes!(br#"  "message":"second"}"#),
    );

    let entries: Vec<&[u8]> = searcher.split(buf).collect();

    assert_eq!(
        entries.len(),
        2,
        "Should produce 2 separate entries, got {}: {:?}",
        entries.len(),
        entries.iter().map(|e| String::from_utf8_lossy(e)).collect::<Vec<_>>()
    );
    assert_eq!(
        entries[0],
        b"{\"timestamp\":\"2024-01-01T00:00:00Z\",\"level\":\"info\",\n  \"message\":\"first\"}"
    );
    assert_eq!(
        entries[1],
        b"{\"timestamp\":\"2024-01-01T00:00:01Z\",\"level\":\"warn\",\n  \"message\":\"second\"}"
    );
}

// Scanner simulation
#[test]
fn test_json_scanner_simulation() {
    let searcher = JsonDelimitSearcher;
    let full_buf = b"}\nasd\n{";

    let result1 = searcher.search_l(full_buf, true);
    assert_eq!(result1, Some(1..2));

    if let Some(range) = result1 {
        let remaining = &full_buf[range.end..];
        let result2 = searcher.search_l(remaining, true);
        assert_eq!(result2, Some(3..4));
    }
}
