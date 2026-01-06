use super::*;
use rstest::rstest;

#[test]
fn test_empty() {
    let searcher = b'/'.into_searcher();
    let buf = b"";
    let mut iter = searcher.split(buf);

    assert_eq!(iter.next(), None);
}

#[test]
fn test_only_delim() {
    let searcher = b'/'.into_searcher();
    let buf = b"/";
    let mut iter = searcher.split(buf);

    assert_eq!(iter.next(), Some(&b""[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_only_delim_auto() {
    let searcher = SmartNewLine.into_searcher();
    let buf = b"\n";
    let mut iter = searcher.split(buf);

    assert_eq!(iter.next(), Some(&b""[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_delim_combo_auto() {
    let searcher = SmartNewLine.into_searcher();
    let buf = b"a\n\r\nb\naaaa\n\r\nbbbb\n";
    let mut iter = searcher.split(buf);

    assert_eq!(iter.next(), Some(&b"a"[..]));
    assert_eq!(iter.next(), Some(&b""[..]));
    assert_eq!(iter.next(), Some(&b"b"[..]));
    assert_eq!(iter.next(), Some(&b"aaaa"[..]));
    assert_eq!(iter.next(), Some(&b""[..]));
    assert_eq!(iter.next(), Some(&b"bbbb"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_no_delim() {
    let searcher = b'/'.into_searcher();
    let buf = b"some";
    let mut iter = searcher.split(buf);

    assert_eq!(iter.next(), Some(&b"some"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_split_iter_byte() {
    let searcher = b'/'.into_searcher();
    let buf = b"test/token/";
    let mut iter = searcher.split(buf);

    assert_eq!(iter.next(), Some(&b"test"[..]));
    assert_eq!(iter.next(), Some(&b"token"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_split_iter_substr() {
    let input = b"test/token/";

    let searcher = SubStrSearcher::new(b"t/");
    let mut iter = searcher.split(input);
    assert_eq!(iter.next(), Some(&b"tes"[..]));
    assert_eq!(iter.next(), Some(&b"token/"[..]));
    assert_eq!(iter.next(), None);

    let searcher = SubStrSearcher::new(b"/t");
    let mut iter = searcher.split(input);
    assert_eq!(iter.next(), Some(&b"test"[..]));
    assert_eq!(iter.next(), Some(&b"oken/"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_substr_search_l() {
    let input = b"test/token/";

    let searcher = SubStrSearcher::new(b"t/");
    assert_eq!(searcher.search_l(input, false), Some(3..5));
    assert_eq!(searcher.search_l(input, true), Some(3..5));

    let searcher = SubStrSearcher::new(b"/t");
    assert_eq!(searcher.search_l(input, false), Some(4..6));
    assert_eq!(searcher.search_l(input, true), Some(4..6));

    let searcher = SubStrSearcher::new(b"n/");
    assert_eq!(searcher.search_l(input, false), Some(9..11));
    assert_eq!(searcher.search_l(input, true), Some(9..11));

    let searcher = SubStrSearcher::new(b"te");
    assert_eq!(searcher.search_l(input, false), Some(0..2));
    assert_eq!(searcher.search_l(input, true), Some(0..2));

    let searcher = SubStrSearcher::new(b"xt");
    assert_eq!(searcher.search_l(input, false), None);
    assert_eq!(searcher.search_l(input, true), None);

    let searcher = SubStrSearcher::new(b"/x");
    assert_eq!(searcher.search_l(input, false), None);
    assert_eq!(searcher.search_l(input, true), None);
}

#[test]
fn test_substr_search_r() {
    let input = b"test/token/";

    let searcher = SubStrSearcher::new(b"t/");
    assert_eq!(searcher.search_r(input, false), Some(3..5));
    assert_eq!(searcher.search_r(input, true), Some(3..5));

    let searcher = SubStrSearcher::new(b"/t");
    assert_eq!(searcher.search_r(input, false), Some(4..6));
    assert_eq!(searcher.search_r(input, true), Some(4..6));

    let searcher = SubStrSearcher::new(b"n/");
    assert_eq!(searcher.search_r(input, false), Some(9..11));
    assert_eq!(searcher.search_r(input, true), Some(9..11));

    let searcher = SubStrSearcher::new(b"te");
    assert_eq!(searcher.search_r(input, false), Some(0..2));
    assert_eq!(searcher.search_r(input, true), Some(0..2));

    let searcher = SubStrSearcher::new(b"xt");
    assert_eq!(searcher.search_r(input, false), None);
    assert_eq!(searcher.search_r(input, true), None);
}

#[test]
fn test_small_token() {
    let sf = Arc::new(SegmentBufFactory::new(20));
    let scanner = Scanner::new(sf.clone(), b'/');
    let mut data = std::io::Cursor::new(b"token");
    let tokens = scanner.items(&mut data).collect::<Result<Vec<_>>>().unwrap();
    assert_eq!(tokens, vec![Segment::Complete(b"token".into())])
}

#[test]
fn test_empty_token_and_small_token() {
    let sf = Arc::new(SegmentBufFactory::new(20));
    let scanner = Scanner::new(sf.clone(), b'/');
    let mut data = std::io::Cursor::new(b"/token");
    let tokens = scanner.items(&mut data).collect::<Result<Vec<_>>>().unwrap();
    assert_eq!(
        tokens,
        vec![Segment::Complete(b"/".into()), Segment::Complete(b"token".into())]
    )
}

#[test]
fn test_small_token_and_empty_token() {
    let sf = Arc::new(SegmentBufFactory::new(20));
    let scanner = Scanner::new(sf.clone(), b'/');
    let mut data = std::io::Cursor::new(b"token/");
    let tokens = scanner.items(&mut data).collect::<Result<Vec<_>>>().unwrap();
    assert_eq!(tokens, vec![Segment::Complete(b"token/".into())])
}

#[test]
fn test_two_small_tokens() {
    let sf = Arc::new(SegmentBufFactory::new(20));
    let scanner = Scanner::new(sf.clone(), b'/');
    let mut data = std::io::Cursor::new(b"test/token/");
    let tokens = scanner.items(&mut data).collect::<Result<Vec<_>>>().unwrap();
    assert_eq!(tokens, vec![Segment::Complete(b"test/token/".into())])
}

#[test]
fn test_two_tokens_over_segment_size() {
    let sf = Arc::new(SegmentBufFactory::new(10));
    let scanner = Scanner::new(sf.clone(), b'/');
    let mut data = std::io::Cursor::new(b"test/token/");
    let tokens = scanner.items(&mut data).collect::<Result<Vec<_>>>().unwrap();
    assert_eq!(
        tokens,
        vec![Segment::Complete(b"test/".into()), Segment::Complete(b"token/".into())]
    )
}

#[test]
fn test_jumbo_1() {
    let sf = Arc::new(SegmentBufFactory::new(2));
    let scanner = Scanner::new(sf.clone(), '/');
    let mut data = std::io::Cursor::new(b"test/token/very/large/");
    let tokens = scanner
        .items(&mut data)
        .with_max_segment_size(6)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Segment::Complete(b"test/".into()),
            Segment::Complete(b"token/".into()),
            Segment::Complete(b"very/".into()),
            Segment::Complete(b"large/".into()),
        ]
    )
}

#[test]
fn test_jumbo_2() {
    let sf = Arc::new(SegmentBufFactory::new(3));
    let scanner = Scanner::new(sf.clone(), "/:");
    let mut data = std::io::Cursor::new(b"test/:token/:very/:large/:x/");
    let tokens = scanner
        .items(&mut data)
        .with_max_segment_size(7)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Segment::Complete(b"test/:".into()),
            Segment::Complete(b"token/:".into()),
            Segment::Complete(b"very/:".into()),
            Segment::Complete(b"large/:".into()),
            Segment::Complete(b"x/".into()),
        ]
    )
}

#[test]
fn test_jumbo_0() {
    let sf = Arc::new(SegmentBufFactory::new(3));
    let scanner = Scanner::new(sf.clone(), "");
    let mut data = std::io::Cursor::new(b"test/:token/:very/:large/:");
    let tokens = scanner
        .items(&mut data)
        .with_max_segment_size(7)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Segment::Incomplete(b"tes".into(), PartialPlacement::First),
            Segment::Incomplete(b"t/:".into(), PartialPlacement::Next),
            Segment::Incomplete(b"tok".into(), PartialPlacement::Next),
            Segment::Incomplete(b"en/".into(), PartialPlacement::Next),
            Segment::Incomplete(b":ve".into(), PartialPlacement::Next),
            Segment::Incomplete(b"ry/".into(), PartialPlacement::Next),
            Segment::Incomplete(b":la".into(), PartialPlacement::Next),
            Segment::Incomplete(b"rge".into(), PartialPlacement::Next),
            Segment::Incomplete(b"/:".into(), PartialPlacement::Last),
        ]
    )
}

#[test]
fn test_jumbo_smart_new_line() {
    let sf = Arc::new(SegmentBufFactory::new(3));
    let scanner = Scanner::new(sf.clone(), SmartNewLine);
    let mut data = std::io::Cursor::new(b"test\r\ntoken\r\nvery\r\nlarge\nx/");
    let tokens = scanner
        .items(&mut data)
        .with_max_segment_size(7)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Segment::Complete(b"test\r\n".into()),
            Segment::Complete(b"token\r\n".into()),
            Segment::Complete(b"very\r\n".into()),
            Segment::Complete(b"large\n".into()),
            Segment::Complete(b"x/".into()),
        ]
    )
}

#[test]
fn test_jumbo_smart_new_line_2() {
    let sf = Arc::new(SegmentBufFactory::new(3));
    let scanner = Scanner::new(sf.clone(), SmartNewLine);
    let mut data = std::io::Cursor::new(b"test token\r\neof\r\n");
    let tokens = scanner
        .items(&mut data)
        .with_max_segment_size(9)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Segment::Incomplete(b"tes".into(), PartialPlacement::First),
            Segment::Incomplete(b"t t".into(), PartialPlacement::Next),
            Segment::Incomplete(b"oke".into(), PartialPlacement::Next),
            Segment::Incomplete(b"n\r\n".into(), PartialPlacement::Last),
            Segment::Complete(b"eof\r\n".into()),
        ]
    )
}

#[test]
fn test_buf_factory_recycle() {
    let factory = BufFactory::new(10);
    let mut buf = factory.new_buf();
    buf.extend_from_slice(b"test");
    factory.recycle(buf);
    let buf = factory.new_buf();
    assert_eq!(buf, b"");
}

#[test]
fn test_substr_searcher_partial_match_l() {
    // Test multi-character delimiter partial matching from left
    let searcher = SubStrSearcher::new("abc".as_bytes());

    // Empty buffer - no range to iterate over
    let buf = b"";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);

    // Buffer that could have a partial match - always finds empty string match
    let buf = b"bc";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(0)); // "abc".ends_with("") is true

    // Buffer with no meaningful match - still finds empty string
    let buf = b"xy";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(0));

    // Test single character delimiter (returns None due to len < 2 check)
    let searcher = SubStrSearcher::new("a".as_bytes());
    let buf = b"a";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);
}

#[test]
fn test_smart_newline_searcher_partial_match_l() {
    let searcher = SmartNewLineSearcher;

    // Buffer starting with \n
    let buf = b"\n";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(1));

    // Buffer starting with \n followed by other characters
    let buf = b"\ntest";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(1));

    // Buffer not starting with \n
    let buf = b"test";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);

    // Empty buffer
    let buf = b"";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);

    // Buffer starting with \r (not \n)
    let buf = b"\rtest";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);
}

#[rstest]
#[case(b"test\n", false, Some(4..5))] // case 1
#[case(b"test\n", true, Some(4..5))] // case 2
#[case(b"test\r\n", false, Some(4..6))] // case 3
#[case(b"test\r\n", true, Some(4..6))] // case 4
#[case(b"\n", false, None)] // case 5
#[case(b"\n", true, Some(0..1))] // case 6
#[case(b"\r\n", false, Some(0..2))] // case 7
#[case(b"\r\n", true, Some(0..2))] // case 8
#[case(b"no newline", false, None)] // case 9
#[case(b"no newline", true, None)] // case 10
#[case(b"", false, None)] // case 11
#[case(b"", true, None)] // case 12
fn test_smart_newline_search_l_edge_cases(
    #[case] buf: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = SmartNewLineSearcher;
    let result = searcher.search_l(buf, edge);
    assert_eq!(result, expected);
}

#[rstest]
#[case(b"first\nsecond\n", false, Some(5..6))] // case 1
#[case(b"first\nsecond\n", true, Some(5..6))] // case 2
#[case(b"first\r\nsecond\r\n", false, Some(5..7))] // case 3
#[case(b"first\r\nsecond\r\n", true, Some(5..7))] // case 4
#[case(b"a\nb\nc\n", false, Some(1..2))] // case 5
#[case(b"a\nb\nc\n", true, Some(1..2))] // case 6
#[case(b"a\r\nb\r\nc\r\n", false, Some(1..3))] // case 7
#[case(b"a\r\nb\r\nc\r\n", true, Some(1..3))] // case 8
fn test_smart_newline_search_l_multiple_newlines(
    #[case] buf: &[u8],
    #[case] edge: bool,
    #[case] expected: Option<Range<usize>>,
) {
    let searcher = SmartNewLineSearcher;
    let result = searcher.search_l(buf, edge);
    assert_eq!(result, expected);
}

#[rstest]
#[case(b"test\n", Some(4..5))] // case 1
#[case(b"test\r\n", Some(4..6))] // case 2
#[case(b"\n", Some(0..1))] // case 3
#[case(b"\r\n", Some(0..2))] // case 4
#[case(b"no newline", None)] // case 5
#[case(b"", None)] // case 6
#[case(b"first\nsecond\n", Some(12..13))] // case 7
#[case(b"first\r\nsecond\r\n", Some(13..15))] // case 8
fn test_smart_newline_search_r(#[case] buf: &[u8], #[case] expected: Option<Range<usize>>) {
    let searcher = SmartNewLineSearcher;
    let result = searcher.search_r(buf, false);
    assert_eq!(result, expected);
    let result = searcher.search_r(buf, true);
    assert_eq!(result, expected);
}

#[rstest]
#[case(b"\r", Some(0))] // case 1
#[case(b"test\r", Some(4))] // case 2
#[case(b"", None)] // case 3
#[case(b"test", None)] // case 4
#[case(b"test\n", None)] // case 5
#[case(b"test\r\n", None)] // case 6
fn test_smart_newline_partial_match_r(#[case] buf: &[u8], #[case] expected: Option<usize>) {
    let searcher = SmartNewLineSearcher;
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, expected);
}

#[test]
fn test_smart_newline_split_mixed_line_endings() {
    let searcher = SmartNewLineSearcher;
    let buf = b"line1\nline2\r\nline3\nline4\r\n";
    let parts: Vec<_> = searcher.split(buf).collect();
    assert_eq!(parts, vec![&b"line1"[..], &b"line2"[..], &b"line3"[..], &b"line4"[..]]);
}

#[test]
fn test_smart_newline_split_only_lf() {
    let searcher = SmartNewLineSearcher;
    let buf = b"a\nb\nc\n";
    let parts: Vec<_> = searcher.split(buf).collect();
    assert_eq!(parts, vec![&b"a"[..], &b"b"[..], &b"c"[..]]);
}

#[test]
fn test_smart_newline_split_only_crlf() {
    let searcher = SmartNewLineSearcher;
    let buf = b"a\r\nb\r\nc\r\n";
    let parts: Vec<_> = searcher.split(buf).collect();
    assert_eq!(parts, vec![&b"a"[..], &b"b"[..], &b"c"[..]]);
}

#[test]
fn test_smart_newline_split_empty_lines() {
    let searcher = SmartNewLineSearcher;
    let buf = b"a\n\nb\r\n\r\nc\n";
    let parts: Vec<_> = searcher.split(buf).collect();
    assert_eq!(parts, vec![&b"a"[..], &b""[..], &b"b"[..], &b""[..], &b"c"[..]]);
}

#[test]
fn test_smart_newline_search_l_edge_false_with_crlf() {
    let searcher = SmartNewLineSearcher;

    // When edge=false, search_l skips the first byte
    // This test verifies correct offset calculation for both LF and CRLF

    // CRLF with edge=false at position 1
    let buf = b"x\r\ntest";
    let result = searcher.search_l(buf, false);
    assert_eq!(
        result,
        Some(1..3),
        "CRLF should be found at correct position with edge=false"
    );

    // LF with edge=false at position 1
    let buf = b"x\ntest";
    let result = searcher.search_l(buf, false);
    assert_eq!(
        result,
        Some(1..2),
        "LF should be found at correct position with edge=false"
    );

    // CRLF with edge=false at later position
    let buf = b"abc\r\ndefg";
    let result = searcher.search_l(buf, false);
    assert_eq!(
        result,
        Some(3..5),
        "CRLF at position 3 should be found correctly with edge=false"
    );

    // Verify edge=true still works correctly
    let buf = b"\r\ntest";
    let result = searcher.search_l(buf, true);
    assert_eq!(result, Some(0..2), "CRLF at start should be found with edge=true");
}
