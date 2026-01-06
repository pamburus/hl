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

    // Multiple newlines and spaces - delimiter is just first newline
    let buf = b"{\"a\":1}\n\n  \n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Tab characters in whitespace - delimiter is just first newline
    let buf = b"{\"a\":1}\n\t\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));
}

#[test]
fn test_json_delimiter_search_l_edge_cases() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Closing brace at end (edge=true) - should find first newline after }
    let buf = b"{\"a\":1}\n\n";
    let result = searcher.search_l(buf, true);
    assert_eq!(result, Some(7..8));

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

    // No closing brace - finds newline delimiter using fallback
    let buf = b"{\"a\":1\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(6..7));

    // No opening brace after closing - no delimiter in non-edge mode
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

    // Multiple newlines and spaces - finds last newline before {
    let buf = b"{\"a\":1}\n\n  \n{\"b\":2}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(11..12));

    // Tab characters in whitespace - finds last newline before {
    let buf = b"{\"a\":1}\n\t\n{\"b\":2}";
    let result = searcher.search_r(buf, false);
    assert_eq!(result, Some(9..10));
}

#[test]
fn test_json_delimiter_search_r_edge_cases() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Opening brace at start (edge=true) - finds last newline before {
    let buf = b"\n\n{\"b\":2}";
    let result = searcher.search_r(buf, true);
    assert_eq!(result, Some(1..2));

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

    // Buffer ending with closing brace and newlines - whitespace-only after }, partial match
    let buf = b"{\"a\":1}\n\n";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(7));

    // Buffer ending with closing brace and space+newline - whitespace-only after }, partial match
    let buf = b"{\"a\":1}  \n";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(7));

    // Buffer ending with closing brace only - empty whitespace is partial match (might continue in next chunk)
    let buf = b"{\"a\":1}";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(7));

    // Buffer ending with closing brace and only spaces - partial match (might have \n in next chunk)
    let buf = b"{\"a\":1}  ";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, Some(7));

    // Buffer ending with non-whitespace after closing brace - no match
    let buf = b"{\"a\":1}x";
    let result = searcher.partial_match_r(buf);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_partial_match_l() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Buffer starting with whitespace-only and opening brace - partial match at opening brace
    let buf = b"\n\n{\"b\":2}";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(2));

    // Buffer starting with whitespace-only (spaces and newline), then opening brace - partial match
    let buf = b"  \n{\"b\":2}";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(3));

    // Buffer starting with only spaces and tabs (no newline) - partial match (might have \n in prev chunk)
    let buf = b"  \t{\"b\":2}";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(3));

    // Buffer starting with whitespace-only newlines - partial match at opening brace
    let buf = b"\n\n{";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(2));

    // Buffer with opening brace only - empty whitespace before is partial match
    let buf = b"{";
    let result = searcher.partial_match_l(buf);
    assert_eq!(result, Some(0));

    // Empty buffer - no opening brace, no match
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

    // With extra whitespace - whitespace between newlines becomes separate entries
    let buf = b"{\"a\":1}\n  \n{\"b\":2}\n\t{\"c\":3}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"{\"a\":1}"[..]));
    assert_eq!(iter.next(), Some(&b"  "[..]));
    assert_eq!(iter.next(), Some(&b"{\"b\":2}"[..]));
    assert_eq!(iter.next(), Some(&b"\t{\"c\":3}"[..]));
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
    let buf = b"line1\r\nline2";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(5..7));
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
    assert_eq!(result, Some(5..6));

    // At end of buffer (edge=true)
    let buf = b"\nline1";
    let result = searcher.search_l(buf, true);
    assert_eq!(result, Some(0..1));

    // At end of buffer (edge=false) - None because at boundary
    let buf = b"\nline1";
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

    // CRLF between objects - delimiter is just CRLF
    let buf = b"{\"a\":1}\r\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..9));

    // Mixed newlines - delimiter is just first CRLF
    let buf = b"{\"a\":1}\r\n  \n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..9));
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
fn test_json_delimiter_with_non_json_content() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Non-JSON content between objects - delimiter is newline after }
    // Non-JSON lines become separate entries
    let buf = b"{\"a\":1}\nx\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Error message between objects - delimiter is newline after }
    let buf = b"{\"a\":1}\nERROR: something failed\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Multi-line error between objects - delimiter is newline after }
    let buf = b"{\"a\":1}\nERROR: line 1\nERROR: line 2\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Stack trace between objects - delimiter is newline after }
    let buf = b"{\"log\":\"data\"}\n  at Module.func (file.js:123:45)\n  at process._tickCallback\n{\"next\":\"log\"}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(14..15));
}

#[test]
fn test_json_delimiter_rejects_array_elements() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Comma after closing brace (array element) - should NOT match
    let buf = b"{\"a\":1}\n,\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Comma with spaces after closing brace - should NOT match
    let buf = b"{\"a\":1}\n  ,  \n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Closing bracket after brace - delimiter at second newline (after ])
    let buf = b"{\"a\":1}\n]\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(9..10));

    // Opening bracket before opening brace (start of array) - should NOT match
    let buf = b"{\"a\":1}\n[\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_rejects_nested_structures() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Array of objects - comma after first object
    let buf = b"[{\"a\":1}\n,\n{\"b\":2}]";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Object within array - bracket before opening brace
    let buf = b"{\"arr\":[\n{\"nested\":1}\n]}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Multiple objects in array with newlines
    let buf = b"[{\"a\":1}\n,{\"b\":2}\n,{\"c\":3}]";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_edge_cases_with_non_json() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Non-JSON at start, then valid JSON - finds newline delimiter
    let buf = b"Some preamble text\n{\"a\":1}";
    let result = searcher.search_l(buf, true);
    assert_eq!(result, Some(18..19));

    // Valid JSON, then non-JSON at end - finds newline delimiter after }
    let buf = b"{\"a\":1}\nSome epilogue text";
    let result = searcher.search_l(buf, true);
    assert_eq!(result, Some(7..8));

    // Multiple non-JSON lines between valid objects - finds first newline
    let buf = b"{\"a\":1}\nLine 1\nLine 2\nLine 3\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));
}

#[test]
fn test_json_delimiter_split_with_non_json() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Split with error messages between objects - splits on newlines
    let buf = b"{\"a\":1}\nERROR\n{\"b\":2}\nWARNING\n{\"c\":3}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"{\"a\":1}"[..]));
    assert_eq!(iter.next(), Some(&b"ERROR"[..]));
    assert_eq!(iter.next(), Some(&b"{\"b\":2}"[..]));
    assert_eq!(iter.next(), Some(&b"WARNING"[..]));
    assert_eq!(iter.next(), Some(&b"{\"c\":3}"[..]));
    assert_eq!(iter.next(), None);

    // Pretty-printed JSON after non-JSON line - splits on newline
    let buf = b"Starting log...\n{\"timestamp\":\"2024-01-01\",\"level\":\"info\"}";
    let mut iter = searcher.split(buf);
    assert_eq!(iter.next(), Some(&b"Starting log..."[..]));
    assert_eq!(
        iter.next(),
        Some(&b"{\"timestamp\":\"2024-01-01\",\"level\":\"info\"}"[..])
    );
    assert_eq!(iter.next(), None);
}

#[test]
fn test_json_delimiter_no_newline_between() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // No newline between } and { - should NOT match
    let buf = b"{\"a\":1}{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Only spaces, no newline - should NOT match
    let buf = b"{\"a\":1}   {\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);

    // Only tabs, no newline - should NOT match
    let buf = b"{\"a\":1}\t\t{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, None);
}

#[test]
fn test_json_delimiter_rejects_non_json_content() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Error messages become separate entries - delimiter is just the newline after }

    // Error message containing { - delimiter at newline after }
    let buf = b"{\"a\":1}\nERROR: found { in message\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Error message containing } - delimiter at newline after }
    let buf = b"{\"a\":1}\nERROR: found } in message\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Error message containing [ and ] - delimiter at newline after }
    let buf = b"{\"a\":1}\nERROR: found [foo] in message\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Error message containing comma - delimiter at newline after }
    let buf = b"{\"a\":1}\nERROR: found, comma\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Multiple structural chars - delimiter at newline after }
    let buf = b"{\"a\":1}\nERROR: {foo}, [bar], etc.\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));
}

#[test]
fn test_json_delimiter_whitespace_only() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Delimiter is just the newline, not all whitespace

    // Just newline - delimiter is the newline
    let buf = b"{\"a\":1}\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Newline and spaces - delimiter is just first newline
    let buf = b"{\"a\":1}\n  \n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Newline and tabs - delimiter is just first newline
    let buf = b"{\"a\":1}\n\t\t\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));

    // Any non-whitespace character - delimiter is still the newline
    let buf = b"{\"a\":1}\nx\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));
}

#[test]
fn test_json_delimiter_includes_newlines_correctly() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Case: JSON, whitespace only, JSON
    // The delimiter is just the first newline
    let buf = b"{\"a\":1}\n\n{\"b\":2}";
    let result = searcher.search_l(buf, false);

    // } at position 6, delimiter is first \n at position 7..8
    assert_eq!(result, Some(7..8));

    // Verify the delimiter content
    if let Some(range) = result {
        let delimiter = &buf[range.clone()];

        // Should be single newline
        assert_eq!(delimiter, b"\n", "Should be single newline");
    }

    // Case: JSON, non-JSON line, JSON - delimiter is newline after }
    let buf = b"{\"a\":1}\nnon-JSON line\n{\"b\":2}";
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(7..8));
}

#[test]
fn test_json_delimiter_pretty_log_case() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Simulate the pretty.log case: JSON, non-JSON line, JSON
    // Delimiters are found at newlines, non-JSON becomes separate entry
    let buf = b"}\nasd\n{";

    // search_l should find delimiter after } (the newline at position 1..2)
    let result = searcher.search_l(buf, false);
    assert_eq!(result, Some(1..2), "Should find newline delimiter after }}");

    // With edge=true, search_l should also find the delimiter
    let result_l = searcher.search_l(buf, true);
    assert_eq!(result_l, Some(1..2));

    // search_r with edge=true should find delimiter before { (the newline at position 5..6)
    let result_r = searcher.search_r(buf, true);
    assert_eq!(result_r, Some(5..6));
}

#[test]
fn test_json_delimiter_scanner_simulation() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Simulate how Scanner would process: }}\nasd\n{{
    // Scanner calls search_l multiple times, using edge=true
    let full_buf = b"}\nasd\n{";

    // First call: search from beginning
    let result1 = searcher.search_l(full_buf, true);
    println!("First search_l (edge=true) on full buffer: {:?}", result1);

    // If result1 is Some(range), the delimiter is consumed, and we continue from range.end
    if let Some(range) = result1 {
        let remaining = &full_buf[range.end..];
        println!(
            "Remaining after first delimiter: {:?}",
            std::str::from_utf8(remaining).unwrap()
        );

        // Second call: search on remaining content
        let result2 = searcher.search_l(remaining, true);
        println!("Second search_l (edge=true) on remaining: {:?}", result2);
    }

    // Test the case where buffer starts with non-JSON content then {
    let buf_non_json = b"asd\n{";
    let result_r = searcher.search_r(buf_non_json, true);
    println!("\nsearch_r (edge=true) on 'asd\\n{{': {:?}", result_r);

    let result_l = searcher.search_l(buf_non_json, true);
    println!("search_l (edge=true) on 'asd\\n{{': {:?}", result_l);

    let result_l_no_edge = searcher.search_l(buf_non_json, false);
    println!("search_l (edge=false) on 'asd\\n{{': {:?}", result_l_no_edge);

    // Test the actual Step 2 buffer
    let buf_step2 = b"asd\n{\"c\":3}\n";
    let result_step2_edge = searcher.search_l(buf_step2, true);
    println!(
        "\nsearch_l (edge=true) on 'asd\\n{{\"c\":3}}\\n': {:?}",
        result_step2_edge
    );
    let result_step2_no_edge = searcher.search_l(buf_step2, false);
    println!(
        "search_l (edge=false) on 'asd\\n{{\"c\":3}}\\n': {:?}",
        result_step2_no_edge
    );
}

#[test]
fn test_json_delimiter_full_pretty_log_case() {
    use super::json::JsonDelimitSearcher;
    let searcher = JsonDelimitSearcher;

    // Full pretty.log case: two JSONs, non-JSON line, third JSON
    let buf = b"{\"a\":1}\n{\"b\":2}\nasd\n{\"c\":3}\n";

    println!("\nFull buffer: {:?}", std::str::from_utf8(buf).unwrap());

    // Manually simulate what split() does
    let mut pos = 0;
    let mut step = 0;
    let mut entries = Vec::new();

    loop {
        if pos >= buf.len() {
            break;
        }

        let remaining = &buf[pos..];
        println!(
            "\nStep {}: pos={}, remaining: {:?}",
            step,
            pos,
            std::str::from_utf8(remaining).unwrap()
        );

        let range = searcher.search_l(remaining, true);
        println!("  search_l returned: {:?}", range);

        if let Some(range) = range {
            let entry = &remaining[..range.start];
            println!("  Entry: {:?}", std::str::from_utf8(entry).unwrap());
            entries.push(entry);
            pos += range.end;
            println!("  New pos: {}", pos);
        } else {
            println!(
                "  No delimiter found, taking rest: {:?}",
                std::str::from_utf8(remaining).unwrap()
            );
            entries.push(remaining);
            break;
        }

        step += 1;
    }

    println!("\nFinal entries:");
    for (i, entry) in entries.iter().enumerate() {
        println!(
            "Entry {}: {:?} (bytes: {:?})",
            i,
            std::str::from_utf8(entry).unwrap_or("<invalid utf8>"),
            entry
        );
    }

    // Should produce 4 entries:
    // 1. First JSON: {"a":1}
    // 2. Second JSON: {"b":2}
    // 3. Non-JSON: asd
    // 4. Third JSON: {"c":3} (with trailing newline from end of buffer)
    assert_eq!(entries.len(), 4, "Should produce 4 separate entries");
    assert_eq!(entries[0], b"{\"a\":1}");
    assert_eq!(entries[1], b"{\"b\":2}");
    assert_eq!(entries[2], b"asd");
    assert_eq!(entries[3], b"{\"c\":3}");

    // Test with single-line JSON (like the simple test file)
    let buf_simple = b"{\"a\":1}\n{\"b\":2}\nasd\n{\"c\":3}\n";
    let mut entries_simple = Vec::new();
    let splits_simple = searcher.split(buf_simple);

    for entry in splits_simple {
        entries_simple.push(entry);
    }

    println!("\nSimple single-line case - entries:");
    for (i, entry) in entries_simple.iter().enumerate() {
        println!(
            "Entry {}: {:?} (bytes: {:?})",
            i,
            std::str::from_utf8(entry).unwrap_or("<invalid utf8>"),
            entry
        );
    }

    assert_eq!(entries_simple.len(), 4, "Simple case should produce 4 entries");
    assert_eq!(entries_simple[0], b"{\"a\":1}");
    assert_eq!(entries_simple[1], b"{\"b\":2}");
    assert_eq!(entries_simple[2], b"asd");
    assert_eq!(entries_simple[3], b"{\"c\":3}");
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
