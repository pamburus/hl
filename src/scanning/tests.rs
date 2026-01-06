use super::*;

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

#[test]
fn test_json_delimiter_search_l_basic() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Two JSON objects with newline between closing and opening brace
    let buf = b"{\"a\":1}\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Multiple newlines and spaces
    let buf = b"{\"a\":1}\n\n  \n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..12));

    // Tab characters in whitespace
    let buf = b"{\"a\":1}\n\t\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..10));
}

#[test]
fn test_json_delimiter_search_l_edge_cases() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Closing brace at end (edge=true) - should find whitespace after }
    let buf = b"{\"a\":1}\n\n";
    let result = searcher.search_l(buf, true);
    assert_eq!(result, Some(7..buf.len()));

    // Closing brace at end (edge=false) - no opening brace after, should not match
    let buf = b"{\"a\":1}\n\n";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // No newline in whitespace - should not match
    let buf = b"{\"a\":1}  {\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Only spaces, no newlines - should not match
    let buf = b"{\"a\":1}   {\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_search_l_no_match() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // No closing brace
    let buf = b"{\"a\":1\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // No opening brace after closing
    let buf = b"{\"a\":1}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Empty buffer
    let buf = b"";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Only opening brace
    let buf = b"{";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Only closing brace
    let buf = b"}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_search_r_basic() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Two JSON objects with newline between closing and opening brace
    let buf = b"{\"a\":1}\n{\"b\":2}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(7..8));

    // Multiple newlines and spaces
    let buf = b"{\"a\":1}\n\n  \n{\"b\":2}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(7..12));

    // CRLF newlines
    let buf = b"{\"a\":1}\r\n{\"b\":2}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(7..9));
}

#[test]
fn test_json_delimiter_search_r_edge_cases() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Opening brace at start (edge=true) - whitespace before first {
    let buf = b"\n\n{\"b\":2}";
    let result = searcher.search_r(buf, true);
    assert_eq!(result, Some(0..2));

    // Opening brace at start (edge=false) - no closing brace before
    let buf = b"\n\n{\"b\":2}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, None);

    // No closing brace found before opening brace
    let buf = b"{\"a\":1}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_search_r_multiple_objects() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Multiple JSON objects - search_r finds rightmost { and looks back for }
    let buf = b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(15..16));

    // Three objects with varying whitespace
    let buf = b"{\"a\":1}\n  \n{\"b\":2}\n\t{\"c\":3}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(18..20));
}

#[test]
fn test_json_delimiter_partial_match_r() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Buffer ending with closing brace and newlines
    // Returns position after } where whitespace starts: position 7
    let buf = b"{\"a\":1}\n\n";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(7));

    // Buffer ending with closing brace and space+newline
    // Returns position after } where whitespace starts: position 7
    let buf = b"{\"a\":1}  \n";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(7));

    // Buffer ending with closing brace only (no whitespace with newline)
    let buf = b"{\"a\":1}";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, None);

    // Buffer ending with spaces only (no closing brace)
    let buf = b"{\"a\":1  \n";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, None);

    // Empty buffer
    let buf = b"";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_partial_match_l() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Buffer starting with newlines and opening brace
    let buf = b"\n\n{\"b\":2}";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(2));

    // Buffer starting with space+newline and opening brace
    let buf = b"  \n{\"b\":2}";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(3));

    // Buffer starting with opening brace only
    let buf = b"{\"b\":2}";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);

    // Buffer with no opening brace
    let buf = b"\n\n  ";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);

    // Empty buffer
    let buf = b"";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_split() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Multiple JSON objects
    let buf = b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"{\"a\":1}"[..]));
    assert_eq!(iter.next(), Some(&b"{\"b\":2}"[..]));
    assert_eq!(iter.next(), Some(&b"{\"c\":3}"[..]));
    assert_eq!(iter.next(), None);

    // With extra whitespace
    let buf = b"{\"a\":1}\n  \n{\"b\":2}\n\t{\"c\":3}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"{\"a\":1}"[..]));
    assert_eq!(iter.next(), Some(&b"{\"b\":2}"[..]));
    assert_eq!(iter.next(), Some(&b"{\"c\":3}"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_auto_delimiter_search_l_basic() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Normal newline followed by regular character
    let buf = b"line1\nline2";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(5..6));

    // CRLF followed by regular character
    // SmartNewLineSearcher with edge=false skips first byte, finds \n at relative position 5
    // checks buf[5-1]=buf[4]='1' (not \r), so returns 6..7 (just the \n)
    let buf = b"line1\r\nline2";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(6..7));
}

#[test]
fn test_auto_delimiter_search_l_skip_continuations() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Newline followed by closing brace - should skip
    let buf = b"line1\n}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Newline followed by space - should skip and continue searching
    // First match at 5..6, buf[6]=' ' (continuation), so skip
    // No more newlines found
    let buf = b"line1\n line2";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Newline followed by tab - should skip
    let buf = b"line1\n\tline2";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Multiple newlines, first followed by }, second by regular char
    let buf = b"line1\n}\nline2";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));
}

#[test]
fn test_auto_delimiter_search_l_edge_cases() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // At end of buffer (edge=true)
    let buf = b"line1\n";
    let result = searcher.search_l(buf, true);
    assert_eq!(result, Some(5..6));

    // At end of buffer (edge=false) - None because at boundary
    let buf = b"line1\n";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Empty buffer
    let buf = b"";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // No newline
    let buf = b"line1";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_auto_delimiter_search_r_basic() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Normal newline followed by regular character
    let buf = b"line1\nline2";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(5..6));

    // CRLF followed by regular character
    let buf = b"line1\r\nline2";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(5..7));

    // Multiple lines - should find last valid delimiter
    let buf = b"line1\nline2\nline3";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(11..12));
}

#[test]
fn test_auto_delimiter_search_r_skip_continuations() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Newline followed by closing brace - should skip
    let buf = b"line1\n}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, None);

    // Newline followed by space - should skip
    let buf = b"line1\n line2";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, None);

    // Newline followed by tab - should skip
    let buf = b"line1\n\tline2";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, None);

    // Multiple newlines, last followed by }, earlier by regular char
    let buf = b"line1\nline2\n}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(5..6));
}

#[test]
fn test_auto_delimiter_search_r_edge_cases() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // At start of buffer (edge=true)
    let buf = b"\nline2";
    let result = searcher.search_r(buf, true);
    assert_eq!(result, Some(0..1));

    // Empty buffer
    let buf = b"";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, None);

    // No newline
    let buf = b"line1";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_auto_delimiter_partial_match_r() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Buffer ending with LF - returns position where \n starts
    let buf = b"line1\n";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(5));

    // Buffer ending with CRLF - returns position where \r starts
    let buf = b"line1\r\n";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(5));

    // Buffer ending with CR only - SmartNewLineSearcher returns position where \r starts
    let buf = b"line1\r";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(5));

    // Buffer not ending with newline
    let buf = b"line1";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, None);

    // Empty buffer
    let buf = b"";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, None);
}

#[test]
fn test_auto_delimiter_partial_match_l() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Buffer starting with LF followed by non-continuation character
    let buf = b"\nline2";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(1));

    // Buffer starting with LF followed by }
    let buf = b"\n}";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(1));

    // Buffer not starting with newline
    let buf = b"line1";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);

    // Empty buffer
    let buf = b"";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, None);
}

#[test]
fn test_auto_delimiter_split() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Multiple lines with valid delimiters
    let buf = b"line1\nline2\nline3";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"line1"[..]));
    assert_eq!(iter.next(), Some(&b"line2"[..]));
    assert_eq!(iter.next(), Some(&b"line3"[..]));
    assert_eq!(iter.next(), None);

    // Lines with continuation (closing brace)
    let buf = b"line1\n}continued\nline2";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"line1\n}continued"[..]));
    assert_eq!(iter.next(), Some(&b"line2"[..]));
    assert_eq!(iter.next(), None);

    // Lines with continuation (space)
    let buf = b"line1\n continued\nline2";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"line1\n continued"[..]));
    assert_eq!(iter.next(), Some(&b"line2"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_delimiter_arc_types() {
    // Test Arc<[u8]> delimiter
    let bytes: Arc<[u8]> = Arc::from(b"::".as_slice());
    let delimiter = Delimiter::from(bytes);
    let searcher = delimiter.into_searcher();
    let buf = b"a::b::c";
    assert_eq!(searcher.search_l(buf, false), Some(1..3));

    // Test Arc<str> delimiter
    let s: Arc<str> = Arc::from("::");
    let delimiter = Delimiter::from(s);
    let searcher = delimiter.into_searcher();
    let buf = b"a::b::c";
    assert_eq!(searcher.search_l(buf, false), Some(1..3));
}

#[test]
fn test_delimiter_enum_conversions() {
    // Test conversion from u8
    let delimiter = Delimiter::from(b'/');
    assert_eq!(delimiter, Delimiter::Byte(b'/'));

    // Test conversion from Vec<u8>
    let delimiter = Delimiter::from(vec![b':', b':']);
    match delimiter {
        Delimiter::Bytes(b) => assert_eq!(&*b, &[b':', b':'][..]),
        _ => panic!("Expected Delimiter::Bytes"),
    }

    // Test conversion from String
    let delimiter = Delimiter::from(String::from("::"));
    match delimiter {
        Delimiter::Str(s) => assert_eq!(&*s, "::"),
        _ => panic!("Expected Delimiter::Str"),
    }
}

#[test]
fn test_delimiter_default() {
    let delimiter = Delimiter::default();
    assert_eq!(delimiter, Delimiter::Auto);
}

#[test]
fn test_json_delimiter_nested_objects() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Nested JSON objects - finds boundary between top-level objects
    // Buffer has actual newline between objects
    let buf = b"{\"a\":{\"b\":1}}\n{\"c\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(13..14));

    // Multiple levels of nesting
    let buf = b"{\"a\":{\"b\":{\"c\":1}}}\n{\"d\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(19..20));
}

#[test]
fn test_json_delimiter_with_crlf() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // CRLF between objects
    let buf = b"{\"a\":1}\r\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..9));

    // Mixed newlines
    // Buffer: {\"a\":1}\r\n  \n{\"b\":2}
    // } at 6, { at 12, space between is buf[7..12] = "\r\n  \n"
    let buf = b"{\"a\":1}\r\n  \n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..12));
}

#[test]
fn test_auto_delimiter_with_mixed_newlines() {
    use super::auto::AutoDelimitSearcher;
    let searcher = AutoDelimitSearcher;

    // Mix of LF and CRLF
    let buf = b"line1\nline2\r\nline3";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"line1"[..]));
    assert_eq!(iter.next(), Some(&b"line2"[..]));
    assert_eq!(iter.next(), Some(&b"line3"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_json_delimiter_invalid_whitespace() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Invalid character in whitespace - should not match
    let buf = b"{\"a\":1}\nx\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Number in whitespace - should not match
    let buf = b"{\"a\":1}\n1{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_searcher_arc_delegation() {
    // Test that Arc<dyn Search> properly delegates to inner searcher
    let searcher: Arc<dyn Search> = Arc::new(b'/'.into_searcher());
    let buf = b"a/b/c";
    assert_eq!(searcher.search_l(buf, false), Some(1..2));
    assert_eq!(searcher.search_r(buf, false), Some(3..4));
    assert_eq!(searcher.partial_match_l(buf), None);
    assert_eq!(searcher.partial_match_r(buf), None);
}

#[test]
fn test_auto_delimiter_with_scanner() {
    use super::auto::AutoDelimiter;

    // Test that AutoDelimiter works correctly with Scanner
    let sf = Arc::new(SegmentBufFactory::new(10));
    let scanner = Scanner::new(sf.clone(), AutoDelimiter);

    // Test case: buffer ends with LF
    let mut data = std::io::Cursor::new(b"test\nmore");
    let tokens = scanner.items(&mut data).collect::<Result<Vec<_>>>().unwrap();

    // Should successfully parse the data
    assert!(!tokens.is_empty());
}

#[test]
fn test_partial_match_r_position_semantics() {
    use super::SmartNewLineSearcher;

    // Verify SmartNewLineSearcher correctly returns positions
    let searcher = SmartNewLineSearcher;

    // Buffer ending with CR
    let buf = b"test\r";
    let result = searcher.partial_match_r(buf);
    // Should return position where CR starts (4)
    assert_eq!(result, Some(4));

    // Verify this works correctly in split calculation
    let bs = buf.len(); // 5
    if let Some(n) = result {
        let length = bs - n; // 5 - 4 = 1 (length of partial match to extract)
        assert_eq!(length, 1);
        // Would copy buf[4..5] = "\r"
        assert_eq!(&buf[bs - length..bs], b"\r");
    }

    // Test with SubStrSearcher for multi-byte delimiter
    let searcher = SubStrSearcher::new(b"abc");
    let buf = b"xyzab";
    let result = searcher.partial_match_r(buf);
    // Should return position where "ab" starts (3)
    assert_eq!(result, Some(3));

    if let Some(n) = result {
        let bs = buf.len(); // 5
        let length = bs - n; // 5 - 3 = 2 (length of "ab")
        assert_eq!(length, 2);
        assert_eq!(&buf[bs - length..bs], b"ab");
    }
}
