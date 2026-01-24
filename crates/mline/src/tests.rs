use super::*;

#[test]
fn test_prefix_lines_1() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n".bytes().collect();
    prefix_lines(&mut buf, .., .., b"> ");
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "> abc\n> defg\n> hi\n> \n> jkl\n");
}

#[test]
fn test_prefix_lines_2() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl".bytes().collect();
    prefix_lines(&mut buf, .., .., b"> ");
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "> abc\n> defg\n> hi\n> \n> jkl");
}

#[test]
fn test_prefix_lines_3() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl".bytes().collect();
    prefix_lines(&mut buf, 4.., .., b"> ");
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "abc\n> defg\n> hi\n> \n> jkl");
}

#[test]
fn test_prefix_lines_4() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl".bytes().collect();
    prefix_lines(&mut buf, .., 1.., b"> ");
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "abc\n> defg\n> hi\n> \n> jkl");
}

#[test]
fn test_prefix_lines_excluded_start() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl".bytes().collect();
    prefix_lines(
        &mut buf,
        ..,
        (std::ops::Bound::Excluded(0), std::ops::Bound::Unbounded),
        b"> ",
    );
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "abc\n> defg\n> hi\n> \n> jkl");
}

#[test]
fn test_prefix_lines_within_1() {
    let mut buf: Vec<_> = "> abc\ndefg\nhi\n\njkl".bytes().collect();
    prefix_lines_within(&mut buf, 2.., 1.., 0..2);
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "> abc\n> defg\n> hi\n> \n> jkl");
}

#[test]
#[should_panic]
fn test_prefix_lines_within_2() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
    prefix_lines_within(&mut buf, .., 1.., 18..20);
}

#[test]
#[should_panic]
fn test_prefix_lines_within_3() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
    #[allow(clippy::reversed_empty_ranges)]
    prefix_lines_within(&mut buf, 15.., 1.., 3..2);
}

#[test]
fn test_prefix_lines_within_4() {
    let mut buf: Vec<_> = "> abc defg hi jkl".bytes().collect();
    prefix_lines_within(&mut buf, 2.., 1.., 0..2);
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "> abc defg hi jkl");
}

#[test]
fn test_suffix_lines_1() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n".bytes().collect();
    suffix_lines(&mut buf, .., .., b": ");
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "abc: \ndefg: \nhi: \n: \njkl: \n");
}

#[test]
fn test_suffix_lines_2() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n".bytes().collect();
    suffix_lines(&mut buf, ..=11, .., b": ");
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "abc: \ndefg: \nhi: \n\njkl\n");
}

#[test]
fn test_suffix_lines_3() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n".bytes().collect();
    suffix_lines(&mut buf, .., 1..5, b": ");
    assert_eq!(std::str::from_utf8(&buf).unwrap(), "abc\ndefg: \nhi: \n: \njkl: \n");
}

#[test]
fn test_suffix_lines_within_1() {
    let mut buf: Vec<_> = "(!) - abc\ndefg\nhi\n\njkl".bytes().collect();
    suffix_lines_within(&mut buf, 4.., 1..=3, 0..4);
    assert_eq!(
        std::str::from_utf8(&buf).unwrap(),
        "(!) - abc\ndefg(!) \nhi(!) \n(!) \njkl"
    );
}

#[test]
#[should_panic]
fn test_suffix_lines_within_2() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
    suffix_lines_within(&mut buf, .., 1.., 18..20);
}

#[test]
#[should_panic]
fn test_suffix_lines_within_3() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
    #[allow(clippy::reversed_empty_ranges)]
    suffix_lines_within(&mut buf, 10.., 1.., 4..1);
}

#[test]
#[should_panic]
fn test_suffix_lines_within_4() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
    #[allow(clippy::reversed_empty_ranges)]
    suffix_lines_within(&mut buf, 12..10, 1..3, 1..3);
}

#[test]
#[should_panic]
fn test_suffix_lines_within_5() {
    let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
    suffix_lines_within(&mut buf, 10..40, 1..3, 1..3);
}

#[test]
#[should_panic(expected = "range start must be inclusive or unbounded")]
fn test_prefix_lines_invalid_exclusive_start() {
    let mut buf: Vec<_> = "abc\ndefg\nhi".bytes().collect();
    prefix_lines(
        &mut buf,
        (std::ops::Bound::Excluded(0), std::ops::Bound::Excluded(5)),
        ..,
        b"> ",
    );
}
