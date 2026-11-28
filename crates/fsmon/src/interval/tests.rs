use rstest::rstest;

use super::*;

/// Returns an estimator with a ceiling of 1 s, and the instant of its last change.
fn changed() -> (IntervalEstimator, Instant) {
    let start = Instant::now();
    let mut estimator = IntervalEstimator::new(Duration::from_secs(1), start);
    estimator.on_change(start);
    (estimator, start)
}

#[test]
fn test_first_interval_is_half_the_default_average() {
    let estimator = IntervalEstimator::new(Duration::from_secs(1), Instant::now());
    assert_eq!(estimator.interval(), DEFAULT_AVERAGE / 2);
}

#[test]
fn test_first_change_does_not_change_the_interval() {
    let (estimator, _) = changed();
    assert_eq!(estimator.interval(), DEFAULT_AVERAGE / 2);
}

#[test]
fn test_frequent_changes_shorten_the_interval_to_the_floor() {
    let (mut estimator, start) = changed();
    for i in 1..=50 {
        estimator.on_change(start + Duration::from_millis(i));
    }
    assert_eq!(estimator.interval(), FLOOR);
}

#[rstest]
#[case::idle_longer_than_average(Duration::from_millis(400), true)]
#[case::idle_shorter_than_average(Duration::from_millis(50), false)]
fn test_idle_time_lengthens_the_interval(#[case] idle: Duration, #[case] longer: bool) {
    let (mut estimator, start) = changed();
    let before = estimator.interval();
    estimator.on_no_change(start + idle);
    assert_eq!(estimator.interval() > before, longer);
}

/// A file that stays idle from the start is checked less and less often too (FR-015b).
#[test]
fn test_idle_time_before_any_change_lengthens_the_interval_to_the_ceiling() {
    let start = Instant::now();
    let mut estimator = IntervalEstimator::new(Duration::from_secs(1), start);
    estimator.on_no_change(start + Duration::from_secs(3600));
    assert_eq!(estimator.interval(), Duration::from_secs(1));
}

#[test]
fn test_long_idle_time_lengthens_the_interval_to_the_ceiling() {
    let (mut estimator, start) = changed();
    let idle = start + Duration::from_secs(3600);
    estimator.on_no_change(idle);
    assert_eq!(estimator.interval(), Duration::from_secs(1));
    // The first change comes after the long idle time, and the average is capped, so the next one shortens the interval.
    estimator.on_change(idle);
    estimator.on_change(idle + Duration::from_millis(10));
    assert!(estimator.interval() < Duration::from_secs(1));
}

#[rstest]
#[case::below_floor(Duration::from_millis(1), FLOOR)]
#[case::above_default(Duration::from_secs(10), DEFAULT_AVERAGE / 2)]
fn test_ceiling_bounds_the_interval(#[case] ceiling: Duration, #[case] expected: Duration) {
    let estimator = IntervalEstimator::new(ceiling, Instant::now());
    assert_eq!(estimator.interval(), expected);
}
