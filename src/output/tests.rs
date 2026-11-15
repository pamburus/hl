use std::sync::Arc;

use rstest::rstest;

use super::*;

#[test]
fn test_buf_factory_recycle() {
    let factory = Arc::new(BufFactory::new(10));
    let mut buf = factory.new_buf();
    buf.extend_from_slice(b"test");
    let ptr = buf.as_ptr();
    drop(buf);
    let buf = factory.new_buf();
    assert_eq!(buf.as_ptr(), ptr);
    assert_eq!(*buf, b"");
}

#[rstest]
#[case::empty(0, false)]
#[case::smaller(9, false)]
#[case::configured(10, true)]
#[case::expanded(160, true)]
#[case::largest(10 * OUTPUT_BUF_KEEP_FACTOR, true)]
#[case::larger(10 * OUTPUT_BUF_KEEP_FACTOR + 1, false)]
fn test_buf_recycler_keeps_capacity_range(#[case] capacity: usize, #[case] kept: bool) {
    // A buffer much larger than any formatted block, grown by a large entry, is not kept for later entries,
    // and a buffer smaller than the configured size is never reused.
    let recycler = BufRecycler { buf_size: 10 };
    assert_eq!(recycler.keep(&Vec::with_capacity(capacity)), kept);
}
