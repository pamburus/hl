// std imports
use std::ops::{Bound, Range, RangeBounds, RangeTo};

// ---

pub fn prefix_lines<BR, LR>(buf: &mut Vec<u8>, bytes: BR, lines: LR, prefix: &[u8])
where
    BR: RangeBounds<usize>,
    LR: RangeBounds<usize>,
{
    let mut o = OffsetBuf::new();
    let (bytes, lines, offsets) = prepare_for_prefix(bytes, buf, &mut o, lines);
    xfix_lines(
        buf,
        bytes,
        lines.end - lines.start,
        offsets,
        prefix.len(),
        |buf, range| {
            buf[range].copy_from_slice(prefix);
        },
    );
}

pub fn prefix_lines_within<BR, LR>(buf: &mut Vec<u8>, bytes: BR, lines: LR, prefix: Range<usize>)
where
    BR: RangeBounds<usize>,
    LR: RangeBounds<usize>,
{
    if prefix.end < prefix.start {
        panic!(
            "prefix end ({1}) should be greater or equal than start ({0})",
            prefix.start, prefix.end
        );
    }

    let mut o = OffsetBuf::new();
    let (bytes, lines, offsets) = prepare_for_prefix(bytes, buf, &mut o, lines);

    if prefix.end > bytes.start {
        panic!("prefix {prefix:?} should go before bytes range {bytes:?}");
    }

    xfix_lines(
        buf,
        bytes,
        lines.end - lines.start,
        offsets,
        prefix.clone().count(),
        |buf, range| {
            debug_assert!(prefix.clone().count() == range.clone().count());
            buf.copy_within(prefix.clone(), range.start);
        },
    );
}

pub fn suffix_lines<BR, LR>(buf: &mut Vec<u8>, bytes: BR, lines: LR, suffix: &[u8])
where
    BR: RangeBounds<usize>,
    LR: RangeBounds<usize>,
{
    let mut o = OffsetBuf::new();
    let (bytes, lines, offsets) = prepare_for_suffix(bytes, buf, &mut o, lines);
    xfix_lines(
        buf,
        bytes,
        lines.end - lines.start,
        offsets,
        suffix.len(),
        |buf, range| {
            buf[range].copy_from_slice(suffix);
        },
    );
}

pub fn suffix_lines_within<BR, LR>(buf: &mut Vec<u8>, bytes: BR, lines: LR, suffix: Range<usize>)
where
    BR: RangeBounds<usize>,
    LR: RangeBounds<usize>,
{
    if suffix.end < suffix.start {
        panic!(
            "suffix end ({1}) should be greater or equal than start ({0})",
            suffix.start, suffix.end
        );
    }

    let mut o = OffsetBuf::new();
    let (bytes, lines, offsets) = prepare_for_suffix(bytes, buf, &mut o, lines);

    if suffix.end > bytes.start {
        panic!("suffix {suffix:?} should go before bytes range {bytes:?}");
    }

    xfix_lines(
        buf,
        bytes,
        lines.end - lines.start,
        offsets,
        suffix.clone().count(),
        |buf, range| {
            debug_assert!(suffix.clone().count() == range.clone().count());
            buf.copy_within(suffix.clone(), range.start);
        },
    );
}

// ---

fn adjust_ranges<R>(offsets: &OffsetBuf, bytes: Range<usize>, lines: R) -> (Range<usize>, Range<usize>)
where
    R: RangeBounds<usize>,
{
    let os = match lines.start_bound() {
        Bound::Included(&start) => start,
        Bound::Excluded(start) => start + 1,
        Bound::Unbounded => 0,
    };

    let oe = match lines.end_bound() {
        Bound::Included(end) => end + 1,
        Bound::Excluded(&end) => end,
        Bound::Unbounded => offsets.len(),
    };

    let os = os.min(offsets.len());

    let bs = if os == 0 { bytes.start } else { offsets[os - 1] };
    let be = if oe >= offsets.len() { bytes.end } else { offsets[oe] };

    (bs..be, os..oe)
}

fn prepare_for_prefix<'a, BR, LR>(
    bytes: BR,
    buf: &mut Vec<u8>,
    o: &'a mut heapopt::Vec<usize, 128>,
    lines: LR,
) -> (Range<usize>, Range<usize>, impl Iterator<Item = usize> + 'a)
where
    BR: RangeBounds<usize>,
    LR: RangeBounds<usize>,
{
    let bytes = range(bytes, ..buf.len());

    o.push(bytes.start);
    for i in bytes.clone() {
        if buf[i] == b'\n' && i != bytes.end - 1 {
            o.push(i + 1);
        }
    }

    let (bytes, lines) = adjust_ranges(&o, bytes, lines);
    let ol = o.len();
    let offsets = o.iter().copied().rev().take(ol - lines.start).skip(ol - lines.end);
    (bytes, lines, offsets)
}

fn prepare_for_suffix<'a, BR, LR>(
    bytes: BR,
    buf: &mut Vec<u8>,
    o: &'a mut heapopt::Vec<usize, 128>,
    lines: LR,
) -> (Range<usize>, Range<usize>, impl Iterator<Item = usize> + 'a)
where
    BR: RangeBounds<usize>,
    LR: RangeBounds<usize>,
{
    let bytes = range(bytes, ..buf.len());

    for i in bytes.clone() {
        if buf[i] == b'\n' {
            o.push(i);
        }
    }

    let (bytes, lines) = adjust_ranges(&o, bytes, lines);
    let ol = o.len();
    let offsets = o.iter().copied().rev().take(ol - lines.start).skip(ol - lines.end);
    (bytes, lines, offsets)
}

fn xfix_lines<OI, F>(buf: &mut Vec<u8>, range: Range<usize>, n: usize, offsets: OI, xfl: usize, f: F)
where
    OI: IntoIterator<Item = usize>,
    F: Fn(&mut [u8], Range<usize>),
{
    let m = buf.len();
    let xl = n * xfl;
    let sl = m - range.end;

    buf.resize(m + xl, 0);
    buf[range.end..].rotate_left(sl);

    let mut ks = range.end;
    let mut ke = buf.len() - sl;
    for o in offsets {
        let l = ks - o;
        ks -= l;
        buf[ks..ke].rotate_left(l);
        ke -= l;
        f(buf, ke - xfl..ke);
        ke -= xfl;
    }
}

#[must_use]
pub fn range<R>(range: R, bounds: RangeTo<usize>) -> Range<usize>
where
    R: RangeBounds<usize>,
{
    let len = bounds.end;

    let start = match range.start_bound() {
        Bound::Included(&start) => start,
        Bound::Unbounded => 0,
        _ => panic!("range start must be inclusive or unbounded"),
    };

    let end = match range.end_bound() {
        Bound::Included(end) => end
            .checked_add(1)
            .unwrap_or_else(|| panic!("attempted to index slice up to maximum usize")),
        Bound::Excluded(&end) => end,
        Bound::Unbounded => len,
    };

    if start > end {
        panic!("slice index starts at {start} but ends at {end}");
    }
    if end > len {
        panic!("range end index {end} out of range for slice of length {len}");
    }

    Range { start, end }
}

// ---

type OffsetBuf = heapopt::Vec<usize, PREALLOC>;

const PREALLOC: usize = 128;

// ---

#[cfg(test)]
mod tests {
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
        suffix_lines_within(&mut buf, 10.., 1.., 4..1);
    }

    #[test]
    #[should_panic]
    fn test_suffix_lines_within_4() {
        let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
        suffix_lines_within(&mut buf, 12..10, 1..3, 1..3);
    }

    #[test]
    #[should_panic]
    fn test_suffix_lines_within_5() {
        let mut buf: Vec<_> = "abc\ndefg\nhi\n\njkl\n> ".bytes().collect();
        suffix_lines_within(&mut buf, 10..40, 1..3, 1..3);
    }
}
