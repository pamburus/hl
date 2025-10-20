use super::*;
use rstest::rstest;

#[rstest]
#[case(b"", false)] // 1
#[case(b"0", true)] // 2
#[case(b"+1", true)] // 3
#[case(b"-1", true)] // 4
#[case(b"1.1", true)] // 5
#[case(b"1.1.0", false)] // 6
#[case(b"a=1", false)] // 7
#[case(b"3.787e+04", true)] // 8
#[case(b"3.787e-04", true)] // 9
#[case(b"-3.787e-04", true)] // 10
fn test_looks_like_number(#[case] input: &[u8], #[case] expected: bool) {
    assert_eq!(looks_like_number(input), expected);
}
