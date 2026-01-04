use super::*;
use rstest::rstest;

#[rstest]
#[case::empty(b"", false)]
#[case::zero(b"0", true)]
#[case::positive_integer(b"+1", true)]
#[case::negative_integer(b"-1", true)]
#[case::decimal(b"1.1", true)]
#[case::invalid_double_dot(b"1.1.0", false)]
#[case::invalid_with_equals(b"a=1", false)]
#[case::scientific_positive_exponent(b"3.787e+04", true)]
#[case::scientific_negative_exponent(b"3.787e-04", true)]
#[case::scientific_negative_base_negative_exponent(b"-3.787e-04", true)]
#[case::scientific_no_decimal(b"1e10", true)]
#[case::scientific_uppercase_e(b"1E10", true)]
#[case::scientific_negative_base(b"-1e10", true)]
#[case::scientific_small_exponent(b"1e-10", true)]
#[case::scientific_explicit_plus(b"1e+10", true)]
#[case::scientific_zero(b"0e0", true)]
#[case::scientific_large_exponent(b"123e456", true)]
#[case::scientific_with_decimal(b"1.0e0", true)]
#[case::invalid_missing_exponent(b"1e", false)]
#[case::invalid_missing_base(b"e10", false)]
#[case::invalid_double_e(b"1ee10", false)]
#[case::invalid_decimal_in_exponent(b"1e10.5", false)]
fn test_looks_like_number(#[case] input: &[u8], #[case] expected: bool) {
    assert_eq!(looks_like_number(input), expected);
}
