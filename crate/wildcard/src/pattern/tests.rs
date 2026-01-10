use rstest::rstest;

use super::*;

fn pattern(s: &str) -> Pattern {
    Pattern::new(s)
}

fn matches(pattern: &str, text: &str) -> bool {
    Pattern::new(pattern).matches(text)
}

#[test]
fn test_pattern_parsing_literal() {
    let p = pattern("hello");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, "hello");
}

#[test]
fn test_pattern_parsing_single_asterisk() {
    let p = pattern("*");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: true, min: 0 });
    assert_eq!(p.segments[0].text, "");
}

#[test]
fn test_pattern_parsing_multiple_asterisks() {
    let p = pattern("***");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: true, min: 0 });
    assert_eq!(p.segments[0].text, "");
}

#[test]
fn test_pattern_parsing_single_question() {
    let p = pattern("?");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 1 });
    assert_eq!(p.segments[0].text, "");
}

#[test]
fn test_pattern_parsing_multiple_questions() {
    let p = pattern("???");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 3 });
    assert_eq!(p.segments[0].text, "");
}

#[test]
fn test_pattern_parsing_mixed_wildcards() {
    let p = pattern("?*?");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: true, min: 2 });
    assert_eq!(p.segments[0].text, "");
}

#[test]
fn test_pattern_parsing_text_with_asterisk() {
    let p = pattern("foo*bar");
    assert_eq!(p.segments.len(), 2);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, "foo");
    assert_eq!(p.segments[1].wild, WildSpec { many: true, min: 0 });
    assert_eq!(p.segments[1].text, "bar");
}

#[test]
fn test_pattern_parsing_text_with_question() {
    let p = pattern("foo?bar");
    assert_eq!(p.segments.len(), 2);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, "foo");
    assert_eq!(p.segments[1].wild, WildSpec { many: false, min: 1 });
    assert_eq!(p.segments[1].text, "bar");
}

#[test]
fn test_pattern_parsing_escaped_asterisk() {
    let p = pattern(r"foo\*bar");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, "foo*bar");
}

#[test]
fn test_pattern_parsing_escaped_question() {
    let p = pattern(r"foo\?bar");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, "foo?bar");
}

#[test]
fn test_pattern_parsing_escaped_backslash() {
    let p = pattern(r"foo\\bar");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, r"foo\bar");
}

#[test]
fn test_pattern_parsing_trailing_backslash() {
    let p = pattern(r"foo\");
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, r"foo\");
}

#[test]
fn test_pattern_parsing_trailing_backslash_after_wildcard() {
    let p = pattern(r"foo*\");
    assert_eq!(p.segments.len(), 2);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, "foo");
    assert_eq!(p.segments[1].wild, WildSpec { many: true, min: 0 });
    assert_eq!(p.segments[1].text, r"\");
}

#[test]
fn test_trailing_backslash_matches() {
    assert!(matches(r"foo\", r"foo\"));
    assert!(matches(r"*\", r"test\"));
    assert!(matches(r"\", r"\"));
    assert!(!matches(r"foo\", "foo"));
    assert!(!matches(r"\", ""));
}

#[test]
fn test_pattern_parsing_complex() {
    let p = pattern(r"foo*bar??baz\*qux");
    assert_eq!(p.segments.len(), 3);
    assert_eq!(p.segments[0].wild, WildSpec { many: false, min: 0 });
    assert_eq!(p.segments[0].text, "foo");
    assert_eq!(p.segments[1].wild, WildSpec { many: true, min: 0 });
    assert_eq!(p.segments[1].text, "bar");
    assert_eq!(p.segments[2].wild, WildSpec { many: false, min: 2 });
    assert_eq!(p.segments[2].text, "baz*qux");
}

#[rstest]
#[case("hello", "hello", true)]
#[case("hello", "world", false)]
#[case("hello", "hell", false)]
#[case("hello", "helloo", false)]
fn test_exact_match(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("*", "")]
#[case("*", "anything")]
#[case("*", "multiple words")]
fn test_asterisk_match_any(#[case] pattern: &str, #[case] text: &str) {
    assert!(matches(pattern, text));
}

#[rstest]
#[case("*world", "world", true)]
#[case("*world", "hello world", true)]
#[case("*world", "xxxworld", true)]
#[case("*world", "world!", false)]
#[case("*world", "wor", false)]
fn test_asterisk_prefix(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("hello*", "hello", true)]
#[case("hello*", "hello world", true)]
#[case("hello*", "helloxxx", true)]
#[case("hello*", "hell", false)]
#[case("hello*", "xhello", false)]
fn test_asterisk_suffix(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("foo*bar", "foobar", true)]
#[case("foo*bar", "fooxbar", true)]
#[case("foo*bar", "fooxxxbar", true)]
#[case("foo*bar", "foo and bar", true)]
#[case("foo*bar", "foobarx", false)]
#[case("foo*bar", "xfoobar", false)]
#[case("foo*bar", "foo", false)]
#[case("foo*bar", "bar", false)]
fn test_asterisk_middle(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("*foo*bar*", "foobar", true)]
#[case("*foo*bar*", "xxxfooxbarxxx", true)]
#[case("*foo*bar*", "foo and bar", true)]
#[case("*foo*bar*", "prefix foo middle bar suffix", true)]
#[case("*foo*bar*", "foo", false)]
#[case("*foo*bar*", "bar", false)]
#[case("*foo*bar*", "barfoo", false)]
fn test_multiple_asterisks(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("?", "a", true)]
#[case("?", "x", true)]
#[case("?", "", false)]
#[case("?", "ab", false)]
fn test_question_mark_single(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("???", "abc", true)]
#[case("???", "xyz", true)]
#[case("???", "ab", false)]
#[case("???", "abcd", false)]
fn test_question_mark_multiple(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("a?c", "abc", true)]
#[case("a?c", "axc", true)]
#[case("a?c", "ac", false)]
#[case("a?c", "abbc", false)]
fn test_question_mark_with_text(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("a*b?c", "abXc", true)]
#[case("a*b?c", "aXbYc", true)]
#[case("a*b?c", "aXXXbYc", true)]
#[case("a*b?c", "abc", false)]
#[case("a*b?c", "abYYc", false)]
fn test_mixed_wildcards(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case(r"\*", "*", true)]
#[case(r"\?", "?", true)]
#[case(r"\\", r"\", true)]
#[case(r"\*", "anything", false)]
#[case(r"\?", "a", false)]
fn test_escaped_special_chars(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("", "", true)]
#[case("", "anything", false)]
fn test_empty_pattern(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[rstest]
#[case("?", "a")]
#[case("?", "ä")]
#[case("?", "世")]
#[case("?", "🔥")]
#[case("???", "abc")]
#[case("???", "äöü")]
#[case("???", "世界語")]
#[case("???", "🔥💧🌊")]
fn test_utf8_question_mark(#[case] pattern: &str, #[case] text: &str) {
    assert!(matches(pattern, text));
}

#[rstest]
#[case("*世界*", "hello世界world")]
#[case("🔥*💧", "🔥test💧")]
#[case("a*ö*z", "aäöüz")]
fn test_utf8_with_asterisk(#[case] pattern: &str, #[case] text: &str) {
    assert!(matches(pattern, text));
}

#[rstest]
#[case("世界", "世界", true)]
#[case("🔥💧🌊", "🔥💧🌊", true)]
#[case("世界", "世", false)]
#[case("🔥", "💧", false)]
fn test_utf8_exact_match(#[case] pattern: &str, #[case] text: &str, #[case] expected: bool) {
    assert_eq!(matches(pattern, text), expected);
}

#[test]
fn test_complex_patterns() {
    assert!(matches("*.txt", "file.txt"));
    assert!(matches("*.txt", "path/to/file.txt"));
    assert!(!matches("*.txt", "file.pdf"));

    assert!(matches("test_*.log", "test_debug.log"));
    assert!(matches("test_*.log", "test_error.log"));
    assert!(!matches("test_*.log", "debug.log"));

    assert!(matches("????-??-??", "2024-01-15"));
    assert!(!matches("????-??-??", "2024-1-15"));
}

#[test]
fn test_greedy_matching() {
    assert!(matches("a*a*a", "aaa"));
    assert!(matches("a*a*a", "aXaYa"));
    assert!(matches("a*a*a", "aXXXaYYYa"));
    assert!(matches("a*a", "aa"));
    assert!(matches("a*a", "aXa"));
    assert!(matches("a*a", "aXXXa"));
}

#[test]
fn test_adjacent_text_patterns() {
    assert!(matches("*foo*foo*", "foofoo"));
    assert!(matches("*foo*foo*", "xfooyfoo"));
    assert!(matches("*foo*foo*", "fooXfooYfoo"));
}

#[test]
fn test_pattern_with_only_wildcards() {
    assert!(matches("*?*", "x"));
    assert!(matches("*?*", "anything"));
    assert!(!matches("*?*", ""));

    assert!(matches("?*?", "ab"));
    assert!(matches("?*?", "aXb"));
    assert!(!matches("?*?", "a"));
}

#[test]
fn test_edge_cases() {
    assert!(matches("a", "a"));
    assert!(!matches("a", ""));
    assert!(!matches("", "a"));

    assert!(matches("*a*", "a"));
    assert!(matches("*a*", "ba"));
    assert!(matches("*a*", "ab"));
    assert!(matches("*a*", "bac"));
}

#[test]
fn test_backtracking() {
    assert!(matches("*.*.*", "a.b.c"));
    assert!(matches("*.*", "a.b"));
    assert!(matches("*a*a*a*", "aaaa"));
    assert!(matches("*a*a*a*", "XaYaZa"));
}

#[test]
fn test_backtracking_first_match_fails() {
    assert!(matches("*ab*cd", "ababcd"));
    assert!(matches("*ab*cd", "abXabcd"));
    assert!(matches("*foo*bar", "foofoofoobar"));
    assert!(matches("*foo*bar", "xfooxfooxbar"));
}

#[test]
fn test_backtracking_multiple_candidates() {
    assert!(matches("a*a*a", "aXaYaZa"));
    assert!(matches("a*a*a", "aaXaaYaa"));
    assert!(matches("x*y*z", "xAyByByCz"));
    assert!(matches("*test*end", "testXtestYtestend"));
}

#[test]
fn test_backtracking_greedy_first_fails() {
    assert!(matches("*.*", "a.b.c"));
    assert!(matches("*.c", "a.b.c"));
    assert!(matches("a*c*e", "abcde"));
    assert!(matches("*o*o", "foobar o"));
}

#[test]
fn test_backtracking_nested_patterns() {
    assert!(matches("*a*b*c*d", "XaYbZcWd"));
    assert!(matches("*a*b*c*d", "aabbccdd"));
    assert!(matches("*a*a*a*a", "aaaaa"));
    assert!(matches("*1*2*3", "X1Y1Z2W3"));
}

#[test]
fn test_backtracking_no_match_after_retries() {
    assert!(!matches("*ab*xy", "ababab"));
    assert!(!matches("*foo*baz", "foofoofoobar"));
    assert!(!matches("*a*b*c*d", "abca"));
    assert!(!matches("*test*end", "testXtestYtest"));
}

#[test]
fn test_backtracking_with_question_marks() {
    assert!(matches("*a?b", "XYZaXb"));
    assert!(matches("*a?b*c", "aXbYaZbWc"));
    assert!(matches("?*?*?", "abc"));
    assert!(matches("?*a*?", "XaYaZ"));
}

#[test]
fn test_no_match_scenarios() {
    assert!(matches("foo*bar*baz", "foobarbaz"));
    assert!(!matches("foo*bar*baz", "foobar"));
    assert!(!matches("foo*bar*baz", "barbaz"));
    assert!(!matches("a?c", "axYc"));
}

#[test]
fn test_special_chars_in_text() {
    assert!(matches("foo-bar", "foo-bar"));
    assert!(matches("foo_bar", "foo_bar"));
    assert!(matches("foo.bar", "foo.bar"));
    assert!(matches("foo@bar", "foo@bar"));
    assert!(matches("foo#bar", "foo#bar"));
}

#[test]
fn test_asterisk_between_same_text() {
    assert!(matches("a*a", "aa"));
    assert!(matches("a*a", "aba"));
    assert!(matches("a*a", "abba"));
    assert!(matches("ab*ab", "abab"));
    assert!(matches("ab*ab", "abXab"));
    assert!(matches("ab*ab", "abXYZab"));
}

#[test]
fn test_multiple_questions_with_asterisk() {
    assert!(matches("a??*b", "aXXb"));
    assert!(matches("a??*b", "aXXYb"));
    assert!(matches("a??*b", "aXXYZb"));
    assert!(!matches("a??*b", "aXb"));
    assert!(!matches("a??*b", "ab"));
}

#[test]
fn test_trailing_asterisk_after_question() {
    assert!(matches("a?*", "ab"));
    assert!(matches("a?*", "abc"));
    assert!(matches("a?*", "abcd"));
    assert!(!matches("a?*", "a"));
}

#[test]
fn test_leading_asterisk_before_question() {
    assert!(matches("*?b", "ab"));
    assert!(matches("*?b", "xab"));
    assert!(matches("*?b", "xyab"));
    assert!(!matches("*?b", "b"));
}

#[rstest]
#[case("hello", "hello")]
#[case("*", "*")]
#[case("***", "*")]
#[case("?", "?")]
#[case("???", "???")]
#[case("?*?", "??*")]
#[case("foo*bar", "foo*bar")]
#[case("foo?bar", "foo?bar")]
#[case("*hello*world*", "*hello*world*")]
#[case(r"foo\*bar", r"foo\*bar")]
#[case(r"foo\?bar", r"foo\?bar")]
#[case(r"foo\\bar", r"foo\\bar")]
#[case(r"foo\", r"foo\\")]
#[case(r"\", r"\\")]
#[case(r"*foo??bar\*baz*", r"*foo??bar\*baz*")]
fn test_display(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(pattern(input).to_string(), expected);
}

#[test]
fn test_display_roundtrip() {
    let patterns = vec![
        "hello",
        "*",
        "?",
        "foo*bar",
        "foo?bar",
        r"foo\*bar",
        r"foo\?bar",
        r"foo\\bar",
        "*foo*bar*",
        "??*",
    ];

    for p in patterns {
        let pattern = Pattern::new(p);
        let displayed = pattern.to_string();
        let reparsed = Pattern::new(&displayed);
        assert_eq!(pattern, reparsed, "Pattern '{}' failed roundtrip", p);
    }
}

#[test]
fn test_backtracking_overlapping_needles() {
    assert!(matches("*aba*aba", "abaaba"));
    assert!(matches("*aba*aba", "XabaYaba"));
    assert!(matches("*aba*aba", "abaabaaba"));
    assert!(matches("*aa*aa", "aaaa"));
    assert!(matches("*aa*aa", "XaaYaa"));
}

#[test]
fn test_backtracking_partial_needle_matches() {
    assert!(matches("*abc*def", "abcabcdef"));
    assert!(matches("*abc*def", "ababcdef"));
    assert!(matches("*test*ing", "testesttesting"));
    assert!(!matches("*abc*def", "abcabc"));
    assert!(!matches("*abc*def", "defdef"));
}

#[test]
fn test_consecutive_asterisks_with_text() {
    assert!(matches("a**b", "ab"));
    assert!(matches("a**b", "aXb"));
    assert!(matches("a***b", "ab"));
    assert!(matches("***a***", "a"));
    assert!(matches("***a***", "XaY"));
}

#[test]
fn test_pattern_longer_than_text() {
    assert!(!matches("abcdef", "abc"));
    assert!(!matches("a?c", "ac"));
    assert!(!matches("???", "ab"));
    assert!(!matches("hello world", "hello"));
}

#[test]
fn test_alternating_wildcards_and_text() {
    assert!(matches("a*b?c*d", "abXcYd"));
    assert!(matches("a*b?c*d", "aXXbYcZZd"));
    assert!(matches("?*?*?*?", "abcd"));
    assert!(!matches("a*b?c*d", "abc"));
}

#[test]
fn test_utf8_backtracking() {
    assert!(matches("*世*界", "世世界"));
    assert!(matches("*世*界", "hello世test界"));
    assert!(matches("*🔥*💧", "🔥🔥💧"));
    assert!(matches("ä*ö*ü", "äXöYü"));
}

#[test]
fn test_single_char_patterns() {
    assert!(matches("a", "a"));
    assert!(matches("?", "a"));
    assert!(matches("*", "a"));
    assert!(!matches("a", "b"));
    assert!(!matches("a", ""));
}

#[test]
fn test_needle_at_boundaries() {
    assert!(matches("*end", "end"));
    assert!(matches("start*", "start"));
    assert!(matches("*middle*", "middle"));
    assert!(matches("*x*", "x"));
}

#[test]
fn test_multiple_same_needles() {
    assert!(matches("*a*a*a", "aaa"));
    assert!(matches("*a*a*a", "XaYaZa"));
    assert!(matches("*x*x*x", "xxx"));
    assert!(matches("*x*x*x", "AxBxCx"));
    assert!(!matches("*a*a*a", "aa"));
}

#[test]
fn test_empty_wildcards_between_text() {
    assert!(matches("a*b", "ab"));
    assert!(matches("a**b", "ab"));
    assert!(matches("a*?*b", "aXb"));
    assert!(!matches("a*?*b", "ab"));
}
