// Comprehensive rstest-based tests for AutoDelimitSearcher
//
// The AutoDelimitSearcher is responsible for finding newline delimiters while skipping
// continuation lines (lines starting with '}', ' ', or '\t').
//
// Test coverage includes:
// 1. Basic delimiter detection (LF and CRLF)
// 2. Continuation character detection and skipping ('}', ' ', '\t')
// 3. Edge mode handling (delimiters at buffer boundaries)
// 4. Partial match semantics (cross-boundary delimiter detection)
// 5. Empty buffers and buffers without delimiters
// 6. Multiple consecutive newlines
// 7. Mixed line endings (LF and CRLF in same buffer)
// 8. Sequences of continuations followed by valid delimiters
// 9. Very long lines without delimiters
// 10. Exact boundary position handling
// 11. Scanner simulation scenarios (multi-chunk processing)

use super::PrettyCompatibleSearcher;
use crate::scanning::{Search, SearchExt};
use rstest::rstest;
use std::ops::Range;

// search_r tests
#[rstest]
#[case(b"line1\nline2", false, Some(5..6))]
#[case(b"line1\r\nline2", false, Some(5..7))]
#[case(b"line1\nline2\nline3", false, Some(11..12))]
#[case(b"a\nb\nc", false, Some(3..4))]
fn test_auto_search_r_valid_delimiters(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

#[rstest]
#[case(b"line1\n}", false, None)]
#[case(b"line1\n ", false, None)]
#[case(b"line1\n\t", false, None)]
#[case(b"line1\nline2\n}", false, Some(5..6))]
#[case(b"a\n}\nb", false, Some(3..4))]
#[case(b"x\n \n\ty\nz", false, Some(6..7))]
fn test_auto_search_r_skip_continuations(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

#[rstest]
#[case(b"\nline", true, Some(0..1))]
#[case(b"line\n", true, Some(4..5))]
#[case(b"line\r\n", true, Some(4..6))]
#[case(b"", false, None)]
#[case(b"no_newline", false, None)]
fn test_auto_search_r_edge_cases(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

#[rstest]
#[case(b"a\n}", true, None)]
#[case(b"a\n ", true, None)]
#[case(b"line\n", true, Some(4..5))]
fn test_auto_search_r_edge_mode_at_buffer_end(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_r(input, edge), expected);
}

// search_l tests
#[rstest]
#[case(b"line1\nline2", false, Some(5..6))]
#[case(b"line1\r\nline2", false, Some(5..7))]
#[case(b"line1\nline2\nline3", false, Some(5..6))]
#[case(b"a\nb\nc", false, Some(1..2))]
fn test_auto_search_l_valid_delimiters(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"line1\n}", false, None)]
#[case(b"line1\n ", false, None)]
#[case(b"line1\n\t", false, None)]
#[case(b"line1\n}line2\nline3", false, Some(12..13))]
#[case(b"a\n}\nb", false, Some(3..4))]
#[case(b"x\n \n\ty\nz", false, Some(6..7))]
fn test_auto_search_l_skip_continuations(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"\nline", true, Some(0..1))]
#[case(b"\nline", false, None)]
#[case(b"line\n", true, Some(4..5))]
#[case(b"line\n", false, Some(4..5))]
#[case(b"", false, None)]
#[case(b"no_newline", false, None)]
fn test_auto_search_l_edge_cases(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"\n}", true, Some(0..1))]
#[case(b"\n ", true, Some(0..1))]
#[case(b"\nline", true, Some(0..1))]
fn test_auto_search_l_edge_mode_at_buffer_start(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

// partial_match_r tests
#[rstest]
#[case(b"line\n", Some(4))]
#[case(b"line\r\n", Some(4))]
#[case(b"line\r", Some(4))]
#[case(b"line", None)]
#[case(b"", None)]
#[case(b"\n", Some(0))]
#[case(b"\r\n", Some(0))]
#[case(b"\r", Some(0))]
fn test_auto_partial_match_r_basic(#[case] input: &[u8], #[case] expected: Option<usize>) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.partial_match_r(input), expected);
}

#[rstest]
#[case(b"abc\n", Some(3))]
#[case(b"abc\r\n", Some(3))]
#[case(b"a\n", Some(1))]
#[case(b"abcdef", None)]
fn test_auto_partial_match_r_positions(#[case] input: &[u8], #[case] expected: Option<usize>) {
    let searcher = PrettyCompatibleSearcher;
    let result = searcher.partial_match_r(input);
    assert_eq!(result, expected);

    if let Some(pos) = result {
        assert!(pos < input.len());
        assert!(input[pos] == b'\n' || input[pos] == b'\r');
    }
}

// partial_match_l tests
#[rstest]
#[case(b"\nline", Some(1))]
#[case(b"\n}", Some(1))]
#[case(b"\n ", Some(1))]
#[case(b"\n\t", Some(1))]
#[case(b"line", None)]
#[case(b"", None)]
#[case(b"\n", Some(1))]
fn test_auto_partial_match_l_basic(#[case] input: &[u8], #[case] expected: Option<usize>) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.partial_match_l(input), expected);
}

// Comprehensive edge case combinations
#[rstest]
#[case(b"\n\n\n", false, Some(1..2))]
#[case(b"\n\n\n", true, Some(0..1))]
#[case(b"a\n\n\n", false, Some(1..2))]
#[case(b"\n}\n}\n}", false, None)]
fn test_auto_multiple_newlines(#[case] input: &[u8], #[case] edge: bool, #[case] expected: Option<Range<usize>>) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_l(input, edge), expected);
}

#[rstest]
#[case(b"line1\nline2", vec![&b"line1"[..], &b"line2"[..]])]
#[case(b"a\nb\nc", vec![&b"a"[..], &b"b"[..], &b"c"[..]])]
#[case(b"x\n}y\nz", vec![&b"x\n}y"[..], &b"z"[..]])]
#[case(b"x\n y\nz", vec![&b"x\n y"[..], &b"z"[..]])]
#[case(b"x\n\ty\nz", vec![&b"x\n\ty"[..], &b"z"[..]])]
#[case(b"no_delim", vec![&b"no_delim"[..]])]
fn test_auto_split_combinations(#[case] input: &[u8], #[case] expected: Vec<&[u8]>) {
    let searcher = PrettyCompatibleSearcher;
    let result: Vec<&[u8]> = searcher.split(input).collect();
    assert_eq!(result, expected);
}

// Test interaction between partial_match and search methods
#[rstest]
#[case(b"line\n", b"next", b"line\nnext")]
#[case(b"line\r", b"\nnext", b"line\r\nnext")]
#[case(b"line\n", b"}", b"line\n}")]
#[case(b"line\n", b" next", b"line\n next")]
fn test_auto_cross_boundary_scenarios(#[case] left: &[u8], #[case] right: &[u8], #[case] combined: &[u8]) {
    let searcher = PrettyCompatibleSearcher;

    let partial = searcher.partial_match_r(left);
    assert!(partial.is_some(), "Should have partial match at boundary");

    let combined_result = searcher.search_l(combined, false);
    if right.first() == Some(&b'}') || right.first() == Some(&b' ') || right.first() == Some(&b'\t') {
        assert_eq!(combined_result, None, "Should skip continuation at boundary");
    } else if combined == b"line\r\nnext" {
        assert_eq!(combined_result, Some(4..6), "Should find CRLF delimiter");
    } else {
        assert!(combined_result.is_some(), "Should find delimiter across boundary");
    }
}

// Test all continuation characters systematically
#[rstest]
#[case(b'}')]
#[case(b' ')]
#[case(b'\t')]
fn test_auto_all_continuation_chars_search_r(#[case] cont: u8) {
    let searcher = PrettyCompatibleSearcher;
    let mut input = Vec::from(&b"line\n"[..]);
    input.push(cont);

    let result = searcher.search_r(&input, false);
    assert_eq!(result, None, "Should skip continuation character {:?}", cont as char);
}

#[rstest]
#[case(b'}')]
#[case(b' ')]
#[case(b'\t')]
fn test_auto_all_continuation_chars_search_l(#[case] cont: u8) {
    let searcher = PrettyCompatibleSearcher;
    let mut input = Vec::from(&b"line\n"[..]);
    input.push(cont);

    let result = searcher.search_l(&input, false);
    assert_eq!(result, None, "Should skip continuation character {:?}", cont as char);
}

// Test mixed line endings
#[rstest]
#[case(b"a\nb\r\nc\nd", false)]
#[case(b"a\r\nb\nc\r\nd", false)]
fn test_auto_mixed_line_endings_search_r(#[case] input: &[u8], #[case] edge: bool) {
    let searcher = PrettyCompatibleSearcher;
    let result = searcher.search_r(input, edge);
    assert!(result.is_some(), "Should handle mixed line endings");

    let range = result.unwrap();
    let delimiter = &input[range.clone()];
    assert!(delimiter == b"\n" || delimiter == b"\r\n");
}

#[rstest]
#[case(b"a\nb\r\nc\nd", false)]
#[case(b"a\r\nb\nc\r\nd", false)]
fn test_auto_mixed_line_endings_search_l(#[case] input: &[u8], #[case] edge: bool) {
    let searcher = PrettyCompatibleSearcher;
    let result = searcher.search_l(input, edge);
    assert!(result.is_some(), "Should handle mixed line endings");

    let range = result.unwrap();
    let delimiter = &input[range.clone()];
    assert!(delimiter == b"\n" || delimiter == b"\r\n");
}

// Test sequences of all continuations followed by valid delimiter
#[rstest]
#[case(b"a\n}\n \n\tb\nc", Some(8..9))]
#[case(b"a\n}\n}\n}b\nc", Some(8..9))]
#[case(b"x\n \n\t\n}y\nz", Some(8..9))]
fn test_auto_continuation_sequences_search_r(#[case] input: &[u8], #[case] expected: Option<Range<usize>>) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_r(input, false), expected);
}

#[rstest]
#[case(b"a\n}\n \n\tb\nc", Some(8..9))]
#[case(b"a\n}\n}\n}b\nc", Some(8..9))]
#[case(b"x\n \n\t\n}y\nz", Some(8..9))]
fn test_auto_continuation_sequences_search_l(#[case] input: &[u8], #[case] expected: Option<Range<usize>>) {
    let searcher = PrettyCompatibleSearcher;
    assert_eq!(searcher.search_l(input, false), expected);
}

// Test very long lines
#[rstest]
#[case(1000)]
#[case(10000)]
fn test_auto_long_lines(#[case] length: usize) {
    let searcher = PrettyCompatibleSearcher;
    let mut input = vec![b'x'; length];
    input.push(b'\n');
    input.push(b'y');

    let result = searcher.search_r(&input, false);
    assert_eq!(result, Some(length..length + 1));

    let result = searcher.search_l(&input, false);
    assert_eq!(result, Some(length..length + 1));
}

// Test exact boundary positions
#[rstest]
#[case(b"\n", true, 0, 1)]
#[case(b"\r\n", true, 0, 2)]
#[case(b"x\n", true, 1, 2)]
#[case(b"x\r\n", true, 1, 3)]
fn test_auto_exact_edge_boundaries_search_r(
    #[case] input: &[u8],
    #[case] edge: bool,
    #[case] exp_start: usize,
    #[case] exp_end: usize,
) {
    let searcher = PrettyCompatibleSearcher;
    let result = searcher.search_r(input, edge);
    assert_eq!(result, Some(exp_start..exp_end));
}

#[test]
fn test_auto_exact_edge_boundaries_search_l() {
    let searcher = PrettyCompatibleSearcher;

    let result = searcher.search_l(b"\nx", true);
    assert_eq!(result, Some(0..1));

    let result = searcher.search_l(b"\nx", false);
    assert_eq!(result, None);
}

// Multi-chunk processing test
#[rstest]
#[case(b"line1\n", b"line2\n", b"line3\n")]
#[case(b"a\n", b"b\n", b"c\n")]
fn test_auto_scanner_simulation_no_extra_blocks(#[case] chunk1: &[u8], #[case] chunk2: &[u8], #[case] chunk3: &[u8]) {
    let searcher = PrettyCompatibleSearcher;

    // Simulate scanner behavior across chunk boundaries
    // Chunk 1: ends with \n
    let partial1 = searcher.partial_match_r(chunk1);
    assert!(partial1.is_some(), "Should detect partial match at end of chunk1");

    // Combine chunk1 remainder with chunk2
    let mut combined = Vec::new();
    if let Some(pos) = partial1 {
        combined.extend_from_slice(&chunk1[pos..]);
    }
    combined.extend_from_slice(chunk2);

    // Should find delimiter in combined
    let found = searcher.search_l(&combined, false);
    assert!(found.is_some(), "Should find delimiter across boundary");

    // Chunk 2: ends with \n
    let partial2 = searcher.partial_match_r(chunk2);
    assert!(partial2.is_some(), "Should detect partial match at end of chunk2");

    // Combine chunk2 remainder with chunk3
    let mut combined2 = Vec::new();
    if let Some(pos) = partial2 {
        combined2.extend_from_slice(&chunk2[pos..]);
    }
    combined2.extend_from_slice(chunk3);

    // Should find delimiter in combined2
    let found2 = searcher.search_l(&combined2, false);
    assert!(found2.is_some(), "Should find delimiter across second boundary");
}
