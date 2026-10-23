use rstest::rstest;

use super::{
    ExtendedSpaceAction, Format, MessageFormatAlwaysQuoted, MessageFormatAutoQuoted, MessageFormatDelimited,
    MessageFormatDoubleQuoted, MessageFormatRaw, Result, ValueFormatAuto, ValueFormatDoubleQuoted, ValueFormatRaw,
};
use encstr::{EncodedString, json::JsonEncodedString, raw::RawString};

/// Helper to format a string using a formatter and return the result
fn format<F: Format>(formatter: &F, input: &str) -> String {
    try_format(formatter, input).unwrap()
}

/// Helper to format a string using a formatter and return the result
fn try_format<F: Format>(formatter: &F, input: &str) -> Result<String> {
    let mut buf = Vec::new();
    let _ = formatter.format(
        EncodedString::Raw(RawString::new(input)),
        &mut buf,
        ExtendedSpaceAction::Inline.into(),
    )?;
    Ok(String::from_utf8(buf).unwrap())
}

// ---
// ValueFormatAuto tests
// ---

#[test]
fn test_value_format_auto_empty_string() {
    // Empty string should produce empty quotes
    assert_eq!(format(&ValueFormatAuto, ""), r#""""#);
}

#[rstest]
#[case::simple_word("hello", "hello")]
#[case::another_word("world", "world")]
#[case::with_digits("world123", "world123")]
#[case::with_underscore("hello_world", "hello_world")]
#[case::with_hyphen("hello-world", "hello-world")]
fn test_value_format_auto_simple_words(#[case] input: &str, #[case] expected: &str) {
    // Simple words without special characters should stay plain (unquoted)
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 3: ValueFormatAuto number handling
// ---

// IMPORTANT: Understanding ValueFormatAuto number behavior
//
// RecordFormatter uses RawValue::auto() which detects numbers via looks_like_number()
// and formats them as RawValue::Number (unquoted, direct output).
// ValueFormatAuto only receives RawValue::String values.
//
// ValueFormatAuto logic for number-like strings:
// 1. If mask contains ONLY Digit/Dot/Minus flags:
//    - If mask == Digit only → QUOTE (pure digits like "42")
//    - If mask has no Other flag → check looks_like_number():
//      - If looks_like_number() = true → QUOTE (ambiguous, could be parsed as number)
//      - If looks_like_number() = false → PLAIN (safe, not a valid number)
// 2. If mask has Other flag → check for JSON literals and looks_like_number():
//    - If matches JSON literals or looks_like_number() = true → QUOTE
//    - Otherwise → PLAIN
// 3. Otherwise → check for other special characters and quote as needed

#[rstest]
// Pure digits → quoted (would be parsed as numbers)
#[case::zero("0", r#""0""#)]
#[case::positive_integer("42", r#""42""#)]
#[case::three_digit_integer("123", r#""123""#)]
// Valid numbers with Digit+Dot or Digit+Minus → quoted (looks_like_number = true)
#[case::negative_integer("-456", r#""-456""#)]
#[case::positive_float("1.23", r#""1.23""#)]
#[case::negative_float("-4.56", r#""-4.56""#)]
#[case::zero_float("0.0", r#""0.0""#)]
// Scientific notation → quoted (has Other flag 'e', but looks_like_number = true)
#[case::scientific_notation("1e10", r#""1e10""#)]
#[case::negative_exponent("2.5e-3", r#""2.5e-3""#)]
fn test_value_format_auto_numbers(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 4: ValueFormatAuto JSON literal detection
// ---

#[rstest]
#[case::empty_object("{}", r#""{}""#)]
#[case::empty_array("[]", r#""[]""#)]
#[case::bool_true("true", r#""true""#)]
#[case::bool_false("false", r#""false""#)]
#[case::null("null", r#""null""#)]
#[case::object_with_content(r#"{"key":"value"}"#, r#"'{"key":"value"}'"#)]
#[case::array_with_content("[1,2,3]", r#""[1,2,3]""#)]
#[case::starts_with_brace("{abc", r#""{abc""#)]
#[case::starts_with_bracket("[abc", r#""[abc""#)]
// Partial matches should NOT be quoted (case sensitive)
#[case::true_upper("True", "True")]
#[case::false_upper("FALSE", "FALSE")]
#[case::truth("truth", "truth")]
#[case::null_upper("Null", "Null")]
fn test_value_format_auto_json_literals(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 5: ValueFormatAuto quote selection
// ---

#[rstest]
// Double quotes when space is present
#[case::needs_quotes_space("hello world", r#""hello world""#)]
// Comma doesn't trigger quoting (maps to Other, which is safe)
#[case::comma_plain("a,b,c", "a,b,c")]
// Single quotes when double quote in content
#[case::has_double_quote(r#"say "hi""#, r#"'say "hi"'"#)]
#[case::multiple_double_quotes(r#"a "b" c "d""#, r#"'a "b" c "d"'"#)]
// Backticks when both double and single quotes present
#[case::both_quotes(r#""both" and 'single'"#, r#"`"both" and 'single'`"#)]
// JSON escaping when all quote types present or control chars
#[case::all_three_quotes(r#""double" 'single' `back`"#, r#""\"double\" 'single' `back`""#)]
#[case::control_char("hello\x00world", r#""hello\u0000world""#)]
// Tab uses backticks (tab doesn't prevent backtick usage)
#[case::tab_char("a\tb", "`a\tb`")]
// Newline uses backticks (newline doesn't prevent backtick usage)
#[case::newline_char("a\nb", "`a\nb`")]
// Backslash uses backticks (backslash doesn't prevent backtick usage)
#[case::backslash(r#"path\to\file"#, r#"`path\to\file`"#)]
fn test_value_format_auto_quote_selection(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 6: ValueFormatAuto whitespace handling (auto-trim)
// ---

#[rstest]
#[case::trailing_space("hello ", "hello")]
#[case::trailing_tab("hello\t", "hello")]
#[case::trailing_newline("hello\n", "hello")]
#[case::trailing_crlf("hello\r\n", "hello")]
#[case::multiple_trailing("text   \t\n", "text")]
// Only whitespace requires quoting
#[case::only_spaces("   ", r#""   ""#)]
#[case::only_tabs("\t\t", r#""\t\t""#)]
// Leading whitespace is preserved and triggers quoting
#[case::leading_space(" hello", r#"" hello""#)]
// Leading tab uses backticks
#[case::leading_tab("\thello", "`\thello`")]
#[case::both_sides(" hello ", r#"" hello""#)]
fn test_value_format_auto_whitespace(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 7: ValueFormatAuto UTF-8 handling
// ---

#[rstest]
#[case::emoji("hello👍", "hello👍")]
#[case::accented("café", "café")]
#[case::chinese("你好", "你好")]
#[case::mixed("test🎉done", "test🎉done")]
#[case::emoji_needs_quotes("a b👍", r#""a b👍""#)]
fn test_value_format_auto_utf8(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 8: ValueFormatAuto error handling
// ---

#[test]
fn test_value_format_auto_invalid_json() {
    let invalid_json = JsonEncodedString::new(r#""invalid\xZZ""#);
    let mut buf = Vec::new();
    let result = ValueFormatAuto.format(
        EncodedString::Json(invalid_json),
        &mut buf,
        ExtendedSpaceAction::Inline.into(),
    );
    assert!(result.is_err());
}

// ---
// Test 9: ValueFormatAuto special characters and edge cases
// ---

#[rstest]
// Most special characters map to Other flag and don't trigger quoting
#[case::underscore("hello_world", "hello_world")]
#[case::hyphen("hello-world", "hello-world")]
#[case::dot_in_path("file.txt", "file.txt")]
#[case::forward_slash("/usr/bin", "/usr/bin")]
#[case::colon("user:group", "user:group")]
#[case::at_sign("user@host", "user@host")]
#[case::semicolon("a;b", "a;b")]
#[case::ampersand("a&b", "a&b")]
#[case::pipe("a|b", "a|b")]
#[case::less_than("a<b", "a<b")]
#[case::greater_than("a>b", "a>b")]
#[case::question_mark("a?b", "a?b")]
#[case::asterisk("a*b", "a*b")]
// Leading quote characters trigger quoting with different quote type
#[case::starts_with_double(r#""quoted""#, r#"'"quoted"'"#)]
#[case::starts_with_single("'quoted'", r#""'quoted'""#)]
#[case::starts_with_backtick("`quoted`", r#""`quoted`""#)]
fn test_value_format_auto_special_chars(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 9: ValueFormatAuto number edge cases
// ---

#[rstest]
#[case::just_minus("-", "-")]
#[case::just_dot(".", ".")]
#[case::minus_dot("-.", "-.")]
#[case::leading_dot(".5", r#"".5""#)]
#[case::trailing_dot("5.", r#""5.""#)]
#[case::multiple_dots("1.2.3", "1.2.3")]
#[case::mixed_alpha("123abc", "123abc")]
#[case::alpha_mixed("abc123", "abc123")]
#[case::leading_zeros("007", r#""007""#)]
// Plus sign at start - looks_like_number returns true, so gets quoted
#[case::plus_sign("+123", r#""+123""#)]
fn test_value_format_auto_number_edge_cases(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatAuto, input), expected);
}

// ---
// Test 10: ValueFormatRaw - passthrough without modification
// ---

#[rstest]
#[case::empty("", "")]
#[case::simple("hello", "hello")]
#[case::with_quotes(r#""quoted""#, r#""quoted""#)]
#[case::with_newlines("line1\nline2", "line1\nline2")]
#[case::with_control("a\x00b", "a\x00b")]
#[case::trailing_space("hello ", "hello ")]
#[case::trailing_newline("hello\n", "hello\n")]
#[case::utf8("café👍", "café👍")]
fn test_value_format_raw(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatRaw, input), expected);
}

// ---
// Test 11: ValueFormatDoubleQuoted - JSON escaping
// ---

#[rstest]
#[case::empty("", r#""""#)]
#[case::simple("text", r#""text""#)]
#[case::double_quote(r#"he"llo"#, r#""he\"llo""#)]
#[case::backslash(r#"path\to"#, r#""path\\to""#)]
#[case::tab("a\tb", r#""a\tb""#)]
#[case::newline("a\nb", r#""a\nb""#)]
#[case::carriage_return("a\rb", r#""a\rb""#)]
#[case::null_char("a\x00b", r#""a\u0000b""#)]
#[case::utf8_preserved("café", r#""café""#)]
#[case::emoji("👍", r#""👍""#)]
fn test_value_format_double_quoted(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&ValueFormatDoubleQuoted, input), expected);
}

// ---
// Test 11b: ValueFormatDoubleQuoted - complete control character suite
// ---

#[rstest]
// 0x00-0x08
#[case::nul("\x00", r#""\u0000""#)]
#[case::soh("\x01", r#""\u0001""#)]
#[case::stx("\x02", r#""\u0002""#)]
#[case::etx("\x03", r#""\u0003""#)]
#[case::eot("\x04", r#""\u0004""#)]
#[case::enq("\x05", r#""\u0005""#)]
#[case::ack("\x06", r#""\u0006""#)]
#[case::bel("\x07", r#""\u0007""#)]
#[case::bs("\x08", r#""\b""#)]
// 0x09-0x0d (special handling)
#[case::ht("\x09", r#""\t""#)]
#[case::lf("\x0a", r#""\n""#)]
#[case::vt("\x0b", r#""\u000b""#)]
#[case::ff("\x0c", r#""\f""#)]
#[case::cr("\x0d", r#""\r""#)]
// 0x0e-0x1f
#[case::so("\x0e", r#""\u000e""#)]
#[case::si("\x0f", r#""\u000f""#)]
#[case::dle("\x10", r#""\u0010""#)]
#[case::dc1("\x11", r#""\u0011""#)]
#[case::dc2("\x12", r#""\u0012""#)]
#[case::dc3("\x13", r#""\u0013""#)]
#[case::dc4("\x14", r#""\u0014""#)]
#[case::nak("\x15", r#""\u0015""#)]
#[case::syn("\x16", r#""\u0016""#)]
#[case::etb("\x17", r#""\u0017""#)]
#[case::can("\x18", r#""\u0018""#)]
#[case::em("\x19", r#""\u0019""#)]
#[case::sub("\x1a", r#""\u001a""#)]
#[case::esc("\x1b", r#""\u001b""#)]
#[case::fs("\x1c", r#""\u001c""#)]
#[case::gs("\x1d", r#""\u001d""#)]
#[case::rs("\x1e", r#""\u001e""#)]
#[case::us("\x1f", r#""\u001f""#)]
// DEL (0x7F) - Now escaped to match jq and best practice
#[case::del("\x7f", r#""\u007f""#)]
fn test_value_format_double_quoted_control_chars(#[case] input: &str, #[case] expected: &str) {
    // Tests the complete control character suite (0x00-0x1F, 0x7F)
    assert_eq!(format(&ValueFormatDoubleQuoted, input), expected);
}

// ---
// Test 12: MessageFormatAutoQuoted - empty handling
// ---

#[test]
fn test_message_format_auto_quoted_empty() {
    assert_eq!(format(&MessageFormatAutoQuoted, ""), r#""""#);
}

// ---
// Test 13: MessageFormatAutoQuoted - plain messages (no quoting)
// ---

#[rstest]
#[case::simple("Hello world", "Hello world")]
#[case::with_punctuation("Hello, world!", "Hello, world!")]
#[case::multiple_words("This is a message", "This is a message")]
#[case::with_hyphen("hello-world", "hello-world")]
#[case::with_colon("Status: OK", "Status: OK")]
#[case::with_slash("path/to/file", "path/to/file")]
#[case::with_numbers("test123", "test123")]
fn test_message_format_auto_quoted_plain(#[case] input: &str, #[case] expected: &str) {
    // Messages without equal sign, control chars, newlines, backslashes, or leading quotes
    assert_eq!(format(&MessageFormatAutoQuoted, input), expected);
}

// ---
// Test 14: MessageFormatAutoQuoted - equal sign triggers quoting
// ---

#[rstest]
#[case::simple_assignment("key=value", r#""key=value""#)]
#[case::math("x=1", r#""x=1""#)]
#[case::in_sentence("where x = y", r#""where x = y""#)]
#[case::multiple("a=1, b=2", r#""a=1, b=2""#)]
#[case::url_param("url?id=123", r#""url?id=123""#)]
fn test_message_format_auto_quoted_equal_sign(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAutoQuoted, input), expected);
}

// ---
// Test 15: MessageFormatAutoQuoted - leading quote triggers quoting
// ---

#[rstest]
#[case::starts_with_double(r#""quoted""#, r#"'"quoted"'"#)]
#[case::starts_with_single("'quoted'", r#""'quoted'""#)]
#[case::starts_with_backtick("`quoted`", r#""`quoted`""#)]
// Double quote in middle doesn't trigger quoting (only LEADING quotes do)
#[case::double_in_middle(r#"say "hi""#, r#"say "hi""#)]
fn test_message_format_auto_quoted_leading_quote(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAutoQuoted, input), expected);
}

// ---
// Test 16: MessageFormatAutoQuoted - control/newline/backslash triggers quoting
// ---

#[rstest]
#[case::newline("line1\nline2", "`line1\nline2`")]
// Tab alone doesn't trigger quoting in MessageFormatAutoQuoted (unlike ValueFormatAuto)
#[case::tab("col1\tcol2", "col1\tcol2")]
#[case::backslash(r#"path\to\file"#, r#"`path\to\file`"#)]
#[case::control_char("text\x00here", r#""text\u0000here""#)]
#[case::carriage_return("a\rb", "`a\rb`")]
fn test_message_format_auto_quoted_control_chars(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAutoQuoted, input), expected);
}

// ---
// Test 17: MessageFormatAutoQuoted - quote selection with conflicts
// ---

#[rstest]
// Double quotes preferred
#[case::equal_no_quotes("a=b", r#""a=b""#)]
// Single when double quote present
#[case::equal_with_double(r#"a="b""#, r#"'a="b"'"#)]
// Backtick when double and single present
#[case::both_quotes(r#"a="b" c='d'"#, r#"`a="b" c='d'`"#)]
// JSON when all quotes present
#[case::all_quotes(r#"a="b" 'c' `d`"#, r#""a=\"b\" 'c' `d`""#)]
fn test_message_format_auto_quoted_quote_selection(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAutoQuoted, input), expected);
}

// ---
// Test 18: MessageFormatAutoQuoted - whitespace with auto-trim
// ---

#[rstest]
#[case::trailing_space("message ", "message")]
#[case::trailing_tab("message\t", "message")]
#[case::trailing_newline("message\n", "message")]
#[case::multiple_trailing("text   \t\n", "text")]
#[case::leading_space(" message", " message")]
#[case::only_spaces("   ", "")]
// Trailing whitespace removed, then no quoting needed
#[case::space_after_trim("hello ", "hello")]
// Leading space preserved but no quoting trigger
#[case::leading_preserved(" hello", " hello")]
// Equal sign with trailing space
#[case::equal_with_trailing("key=value ", r#""key=value""#)]
fn test_message_format_auto_quoted_whitespace(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAutoQuoted, input), expected);
}

// ---
// Test 19: MessageFormatAlwaysQuoted - basic quoting
// ---

#[test]
fn test_message_format_always_quoted_empty() {
    // Empty messages produce no output (not even quotes)
    assert_eq!(format(&MessageFormatAlwaysQuoted, ""), "");
}

#[rstest]
#[case::simple("hello", r#""hello""#)]
#[case::with_space("hello world", r#""hello world""#)]
#[case::with_single("it's", r#""it's""#)]
#[case::safe_chars("test123", r#""test123""#)]
fn test_message_format_always_quoted_double(#[case] input: &str, #[case] expected: &str) {
    // Simple cases use double quotes
    assert_eq!(format(&MessageFormatAlwaysQuoted, input), expected);
}

// ---
// Test 20: MessageFormatAlwaysQuoted - switch to single quote
// ---

#[rstest]
#[case::has_double(r#"say "hi""#, r#"'say "hi"'"#)]
#[case::multiple_doubles(r#"a "b" c "d""#, r#"'a "b" c "d"'"#)]
fn test_message_format_always_quoted_single(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAlwaysQuoted, input), expected);
}

// ---
// Test 21: MessageFormatAlwaysQuoted - switch to backtick
// ---

#[rstest]
#[case::both_quotes(r#""both" and 'single'"#, r#"`"both" and 'single'`"#)]
#[case::complex(r#"a "b" c 'd'"#, r#"`a "b" c 'd'`"#)]
fn test_message_format_always_quoted_backtick(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAlwaysQuoted, input), expected);
}

// ---
// Test 22: MessageFormatAlwaysQuoted - JSON fallback
// ---

#[rstest]
#[case::all_three(r#""double" 'single' `back`"#, r#""\"double\" 'single' `back`""#)]
#[case::with_control("text\x00", r#""text\u0000""#)]
// Newline with double quotes uses backticks (newline doesn't prevent backticks)
#[case::newline_and_quotes("line1\n\"quote\"", "`line1\n\"quote\"`")]
fn test_message_format_always_quoted_json(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatAlwaysQuoted, input), expected);
}

// ---
// Test 23: MessageFormatDelimited - empty handling
// ---

#[test]
fn test_message_format_delimited_empty() {
    // Empty messages produce no output (no delimiter either)
    let formatter = MessageFormatDelimited::new(" | ".to_string());
    assert_eq!(format(&formatter, ""), "");
}

// ---
// Test 24: MessageFormatDelimited - plain with delimiter
// ---

#[rstest]
#[case::simple("text", " | ", "text | ")]
#[case::multiple_words("Multiple words", " :: ", "Multiple words :: ")]
#[case::numbers("123", " | ", "123 | ")]
#[case::single_char_delim("hello", "|", "hello|")]
// Empty delimiter causes the memmem search to succeed, triggering quoting
#[case::empty_delim("test", "", r#""test""#)]
fn test_message_format_delimited_plain(#[case] input: &str, #[case] delim: &str, #[case] expected: &str) {
    let formatter = MessageFormatDelimited::new(delim.to_string());
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 25: MessageFormatDelimited - delimiter in content triggers quoting
// ---

#[rstest]
#[case::exact_match("a | b", " | ", r#""a | b" | "#)]
#[case::multiple_occurrences("a::b::c", "::", r#""a::b::c"::"#)]
#[case::partial_no_match("a|b", " | ", "a|b | ")]
// Delimiter search is exact - "| test" doesn't contain " | "
#[case::delim_at_start("| test", " | ", "| test | ")]
#[case::delim_at_end("test |", " | ", "test | | ")]
fn test_message_format_delimited_in_content(#[case] input: &str, #[case] delim: &str, #[case] expected: &str) {
    let formatter = MessageFormatDelimited::new(delim.to_string());
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 26: MessageFormatDelimited - control chars trigger quoting
// ---

#[rstest]
#[case::control_char("text\x00", " | ", r#""text\u0000" | "#)]
// Newline and tab don't trigger quoting in MessageFormatDelimited (only Control flag does)
#[case::newline("a\nb", " | ", "a\nb | ")]
#[case::tab("a\tb", " | ", "a\tb | ")]
fn test_message_format_delimited_control_chars(#[case] input: &str, #[case] delim: &str, #[case] expected: &str) {
    let formatter = MessageFormatDelimited::new(delim.to_string());
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 27: MessageFormatDelimited - leading quote triggers quoting
// ---

#[rstest]
#[case::starts_with_double(r#""quoted""#, " | ", r#"'"quoted"' | "#)]
#[case::starts_with_single("'quoted'", " | ", r#""'quoted'" | "#)]
#[case::starts_with_backtick("`quoted`", " | ", r#""`quoted`" | "#)]
fn test_message_format_delimited_leading_quote(#[case] input: &str, #[case] delim: &str, #[case] expected: &str) {
    let formatter = MessageFormatDelimited::new(delim.to_string());
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 28: MessageFormatDelimited - quote selection
// ---

#[rstest]
// Double quotes when no conflicts
#[case::needs_quotes_double(r#"a | b"#, " | ", r#""a | b" | "#)]
// Single when double quote in content
#[case::has_double(r#"a | "b""#, " | ", r#"'a | "b"' | "#)]
// Backtick when both quotes
#[case::both_quotes(r#""a" | 'b'"#, " | ", r#"`"a" | 'b'` | "#)]
// JSON when all quotes - delimiter in content gets quoted, delimiter appended after
#[case::all_quotes(r#""a" 'b' `c` | d"#, " | ", r#""\"a\" 'b' `c` | d" | "#)]
fn test_message_format_delimited_quote_selection(#[case] input: &str, #[case] delim: &str, #[case] expected: &str) {
    let formatter = MessageFormatDelimited::new(delim.to_string());
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 29: MessageFormatDelimited - whitespace with auto-trim
// ---

#[rstest]
#[case::trailing_space("message ", " | ", "message | ")]
#[case::trailing_tab("message\t", " | ", "message | ")]
#[case::trailing_newline("message\n", " | ", "message | ")]
// Whitespace-only: trimmed to empty, so only delimiter appears
#[case::only_spaces("   ", " | ", " | ")]
fn test_message_format_delimited_whitespace(#[case] input: &str, #[case] delim: &str, #[case] expected: &str) {
    let formatter = MessageFormatDelimited::new(delim.to_string());
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 30: MessageFormatRaw - passthrough without modification
// ---

#[rstest]
#[case::empty("", "")]
#[case::simple("hello", "hello")]
#[case::with_quotes(r#""quoted""#, r#""quoted""#)]
#[case::with_newlines("line1\nline2", "line1\nline2")]
#[case::with_control("a\x00b", "a\x00b")]
#[case::trailing_space("hello ", "hello ")]
#[case::trailing_newline("hello\n", "hello\n")]
#[case::utf8("café👍", "café👍")]
fn test_message_format_raw(#[case] input: &str, #[case] expected: &str) {
    // MessageFormatRaw is identical to ValueFormatRaw - no auto-trim, pure passthrough
    assert_eq!(format(&MessageFormatRaw, input), expected);
}

// ---
// Test 31: MessageFormatDoubleQuoted - JSON escaping
// ---

#[rstest]
#[case::empty("", r#""""#)]
#[case::simple("text", r#""text""#)]
#[case::double_quote(r#"he"llo"#, r#""he\"llo""#)]
#[case::backslash(r#"path\to"#, r#""path\\to""#)]
#[case::tab("a\tb", r#""a\tb""#)]
#[case::newline("a\nb", r#""a\nb""#)]
#[case::carriage_return("a\rb", r#""a\rb""#)]
#[case::null_char("a\x00b", r#""a\u0000b""#)]
#[case::utf8_preserved("café", r#""café""#)]
#[case::emoji("👍", r#""👍""#)]
// Control characters (additional coverage)
#[case::bell("\x07", r#""\u0007""#)]
#[case::backspace("\x08", r#""\b""#)]
#[case::vertical_tab("\x0b", r#""\u000b""#)]
#[case::form_feed("\x0c", r#""\f""#)]
fn test_message_format_double_quoted(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(format(&MessageFormatDoubleQuoted, input), expected);
}

// ---
// Test 31b: MessageFormatDoubleQuoted - complete control character suite
// ---

#[rstest]
// 0x00-0x08
#[case::nul("\x00", r#""\u0000""#)]
#[case::soh("\x01", r#""\u0001""#)]
#[case::stx("\x02", r#""\u0002""#)]
#[case::etx("\x03", r#""\u0003""#)]
#[case::eot("\x04", r#""\u0004""#)]
#[case::enq("\x05", r#""\u0005""#)]
#[case::ack("\x06", r#""\u0006""#)]
#[case::bel("\x07", r#""\u0007""#)]
#[case::bs("\x08", r#""\b""#)]
// 0x09-0x0d (special handling)
#[case::ht("\x09", r#""\t""#)]
#[case::lf("\x0a", r#""\n""#)]
#[case::vt("\x0b", r#""\u000b""#)]
#[case::ff("\x0c", r#""\f""#)]
#[case::cr("\x0d", r#""\r""#)]
// 0x0e-0x1f
#[case::so("\x0e", r#""\u000e""#)]
#[case::si("\x0f", r#""\u000f""#)]
#[case::dle("\x10", r#""\u0010""#)]
#[case::dc1("\x11", r#""\u0011""#)]
#[case::dc2("\x12", r#""\u0012""#)]
#[case::dc3("\x13", r#""\u0013""#)]
#[case::dc4("\x14", r#""\u0014""#)]
#[case::nak("\x15", r#""\u0015""#)]
#[case::syn("\x16", r#""\u0016""#)]
#[case::etb("\x17", r#""\u0017""#)]
#[case::can("\x18", r#""\u0018""#)]
#[case::em("\x19", r#""\u0019""#)]
#[case::sub("\x1a", r#""\u001a""#)]
#[case::esc("\x1b", r#""\u001b""#)]
#[case::fs("\x1c", r#""\u001c""#)]
#[case::gs("\x1d", r#""\u001d""#)]
#[case::rs("\x1e", r#""\u001e""#)]
#[case::us("\x1f", r#""\u001f""#)]
// DEL (0x7F) - Now escaped to match jq and best practice
#[case::del("\x7f", r#""\u007f""#)]
fn test_message_format_double_quoted_control_chars(#[case] input: &str, #[case] expected: &str) {
    // Tests the complete control character suite (0x00-0x1F, 0x7F)
    assert_eq!(format(&MessageFormatDoubleQuoted, input), expected);
}

// ---
// Test 32: FormatRightTrimmed - basic trimming
// ---

#[test]
fn test_format_right_trimmed_no_trim() {
    let formatter = ValueFormatRaw.rtrim(0);
    assert_eq!(format(&formatter, "hello"), "hello");
}

#[rstest]
#[case::trim_2("hello", 2, "hel")]
#[case::trim_5("hello", 5, "")]
#[case::trim_beyond("hello", 10, "")]
#[case::trim_1("test", 1, "tes")]
fn test_format_right_trimmed_basic(#[case] input: &str, #[case] n: usize, #[case] expected: &str) {
    let formatter = ValueFormatRaw.rtrim(n);
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 33: FormatRightTrimmed - empty results
// ---

#[rstest]
#[case::empty_no_trim("", 0, "")]
#[case::empty_with_trim("", 5, "")]
fn test_format_right_trimmed_empty(#[case] input: &str, #[case] n: usize, #[case] expected: &str) {
    let formatter = ValueFormatRaw.rtrim(n);
    assert_eq!(format(&formatter, input), expected);
}

// ---
// Test 34: FormatRightTrimmed - with different inner formatters
// ---

#[test]
fn test_format_right_trimmed_with_auto() {
    // ValueFormatAuto adds quotes, then trim removes from the quotes
    let formatter = ValueFormatAuto.rtrim(1);
    assert_eq!(format(&formatter, "42"), r#""42"#); // Trims closing quote
}

#[test]
fn test_format_right_trimmed_with_double_quoted() {
    // ValueFormatDoubleQuoted adds "text", trim 1 removes closing quote
    let formatter = ValueFormatDoubleQuoted.rtrim(1);
    assert_eq!(format(&formatter, "hello"), r#""hello"#);
}

#[test]
fn test_format_right_trimmed_with_delimited() {
    // MessageFormatDelimited appends delimiter, trim removes it
    let formatter = MessageFormatDelimited::new(" | ".to_string()).rtrim(3);
    assert_eq!(format(&formatter, "test"), "test");
}

// ---
// Test 35: FormatRightTrimmed - nested wrappers
// ---

#[test]
fn test_format_right_trimmed_nested() {
    // Double wrapping: inner trims 2, outer trims 1
    let formatter = ValueFormatRaw.rtrim(2).rtrim(1);
    assert_eq!(format(&formatter, "hello"), "he"); // "hello" -> "hel" -> "he"
}

// ---
// Test 36: FormatRightTrimmed - UTF-8 behavior
// ---

#[test]
fn test_format_right_trimmed_emoji() {
    // 👍 is 4 bytes, trimming 4 should remove it completely
    let formatter = ValueFormatRaw.rtrim(4);
    assert_eq!(format(&formatter, "test👍"), "test");
}

#[test]
fn test_format_right_trimmed_utf8_safe() {
    // Trim ASCII characters safely
    let formatter = ValueFormatRaw.rtrim(1);
    assert_eq!(format(&formatter, "hello"), "hell");
}

// Note: Trimming can split multi-byte UTF-8 characters, causing invalid UTF-8.
// This is expected byte-level behavior - callers must ensure trim amount is safe.

// ---
// Test 37: FormatRightTrimmed - with MessageFormatAutoQuoted
// ---

#[test]
fn test_format_right_trimmed_with_message_auto() {
    // Plain message, no trimming
    let formatter = MessageFormatAutoQuoted.rtrim(0);
    assert_eq!(format(&formatter, "hello"), "hello");
}

#[test]
fn test_format_right_trimmed_message_auto_with_quotes() {
    // Message with equal sign gets quoted, then trim
    let formatter = MessageFormatAutoQuoted.rtrim(1);
    assert_eq!(format(&formatter, "key=value"), r#""key=value"#); // Trim closing quote
}
