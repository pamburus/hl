use super::*;

#[test]
fn test_optimized_buf_push() {
    let mut buf = OptimizedBuf::<u8, 4>::new();
    assert_eq!(buf.len(), 0);
    buf.push(1);
    assert_eq!(buf.len(), 1);
    buf.push(2);
    assert_eq!(buf.len(), 2);
    buf.push(3);
    assert_eq!(buf.len(), 3);
    buf.push(4);
    assert_eq!(buf.len(), 4);
    buf.push(5);
    assert_eq!(buf.len(), 5);
    assert_eq!(buf.as_slices().0, &[1, 2, 3, 4]);
    assert_eq!(buf.as_slices().1, &[5]);
}

#[test]
fn test_optimized_buf_extend() {
    let mut buf = OptimizedBuf::<u8, 4>::new();
    assert_eq!(buf.len(), 0);
    buf.extend_from_slice(&[]);
    assert_eq!(buf.len(), 0);
    buf.extend_from_slice(&[1]);
    assert_eq!(buf.len(), 1);
    buf.extend_from_slice(&[2, 3]);
    assert_eq!(buf.len(), 3);
    buf.extend_from_slice(&[4, 5, 6]);
    assert_eq!(buf.len(), 6);
    assert_eq!(buf.as_slices().0, &[1, 2, 3, 4]);
    assert_eq!(buf.as_slices().1, &[5, 6]);
}

#[test]
fn test_optimized_buf_truncate() {
    let mut buf = OptimizedBuf::<u8, 4>::new();
    assert_eq!(buf.len(), 0);
    buf.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(buf.len(), 7);
    buf.truncate(8);
    assert_eq!(buf.len(), 7);
    assert_eq!(buf.as_slices().0, &[1, 2, 3, 4]);
    assert_eq!(buf.as_slices().1, &[5, 6, 7]);
    buf.truncate(7);
    assert_eq!(buf.len(), 7);
    assert_eq!(buf.as_slices().0, &[1, 2, 3, 4]);
    assert_eq!(buf.as_slices().1, &[5, 6, 7]);
    buf.truncate(6);
    assert_eq!(buf.len(), 6);
    assert_eq!(buf.as_slices().0, &[1, 2, 3, 4]);
    assert_eq!(buf.as_slices().1, &[5, 6]);
    buf.truncate(4);
    assert_eq!(buf.len(), 4);
    assert_eq!(buf.as_slices().0, &[1, 2, 3, 4]);
    assert_eq!(buf.as_slices().1.len(), 0);
    buf.truncate(4);
    buf.extend_from_slice(&[8, 9]);
    assert_eq!(buf.len(), 6);
    assert_eq!(buf.as_slices().0, &[1, 2, 3, 4]);
    assert_eq!(buf.as_slices().1, &[8, 9]);
    buf.truncate(3);
    assert_eq!(buf.len(), 3);
    assert_eq!(buf.as_slices().0, &[1, 2, 3]);
    assert_eq!(buf.as_slices().1.len(), 0);
    buf.truncate(0);
    assert_eq!(buf.len(), 0);
    assert_eq!(buf.as_slices().0.len(), 0);
    assert_eq!(buf.as_slices().1.len(), 0);
}

#[test]
fn test_aligner_disabled() {
    let mut buf = Vec::new();
    aligned(&mut buf, None, |mut aligner| {
        aligner.push(1);
        aligner.push(2);
        aligner.push(3);
    });
    assert_eq!(buf, vec![1, 2, 3]);

    let mut buf = Vec::new();
    aligned(&mut buf, None, |mut aligner| {
        aligner.extend_from_slice(&[1, 2, 3]);
    });
    assert_eq!(buf, vec![1, 2, 3]);
}

#[test]
fn test_aligner_left() {
    let mut buf = Vec::new();
    aligned(
        &mut buf,
        Some(Adjustment::new(Alignment::Left, Padding::new(0, 5))),
        |mut aligner| {
            aligner.push(1);
            aligner.push(2);
            aligner.push(3);
        },
    );
    assert_eq!(buf, vec![1, 2, 3, 0, 0]);

    let mut buf = Vec::new();
    aligned(
        &mut buf,
        Some(Adjustment::new(Alignment::Left, Padding::new(0, 5))),
        |mut aligner| {
            aligner.extend_from_slice(&[1, 2, 3]);
        },
    );
    assert_eq!(buf, vec![1, 2, 3, 0, 0]);
}

#[test]
fn test_aligner_center() {
    let mut buf = Vec::new();
    aligned(
        &mut buf,
        Some(Adjustment::new(Alignment::Center, Padding::new(0, 5))),
        |mut aligner| {
            aligner.push(1);
            aligner.push(2);
            aligner.push(3);
        },
    );
    assert_eq!(buf, vec![0, 1, 2, 3, 0]);

    let mut buf = Vec::new();
    aligned(
        &mut buf,
        Some(Adjustment::new(Alignment::Center, Padding::new(0, 5))),
        |mut aligner| {
            aligner.extend_from_slice(&[1, 2, 3]);
        },
    );
    assert_eq!(buf, vec![0, 1, 2, 3, 0]);
}

#[test]
fn test_aligner_right() {
    let mut buf = Vec::new();
    aligned(
        &mut buf,
        Some(Adjustment::new(Alignment::Right, Padding::new(0, 5))),
        |mut aligner| {
            aligner.push(1);
            aligner.push(2);
            aligner.push(3);
        },
    );
    assert_eq!(buf, vec![0, 0, 1, 2, 3]);

    let mut buf = Vec::new();
    aligned(
        &mut buf,
        Some(Adjustment::new(Alignment::Right, Padding::new(0, 5))),
        |mut aligner| {
            aligner.extend_from_slice(&[1, 2, 3]);
        },
    );
    assert_eq!(buf, vec![0, 0, 1, 2, 3]);
}

#[test]
fn test_counter_default() {
    let counter1 = Counter::default();
    let counter2 = Counter::new();

    // Both should have the same initial state
    assert_eq!(counter1.result(), counter2.result());
    assert_eq!(counter1.result(), 0);
}
