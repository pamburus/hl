use super::*;
use std::thread::sleep;

fn floor() -> Duration {
    Duration::from_secs_f64(FLOOR_SECS)
}
fn ceiling() -> Duration {
    Duration::from_secs_f64(CEILING_SECS)
}

#[test]
fn default_sleep_is_half_default() {
    let est = IntervalEstimator::new();
    // 100 ms average → 50 ms sleep.
    assert_eq!(est.sleep_duration(), Duration::from_secs_f64(DEFAULT_SECS / 2.0));
}

#[test]
fn sleep_duration_stays_within_clamp() {
    let est = IntervalEstimator::new();
    let d = est.sleep_duration();
    assert!(
        d >= floor() && d <= ceiling(),
        "{d:?} out of [{:?}, {:?}]",
        floor(),
        ceiling()
    );
}

#[test]
fn first_change_sets_baseline_without_updating_average() {
    let mut est = IntervalEstimator::new();
    let before = est.avg_secs;
    est.on_change(); // no previous change yet → no EMA update
    assert_eq!(est.avg_secs, before);
}

#[test]
fn rapid_changes_pull_average_toward_floor() {
    let mut est = IntervalEstimator::new();
    est.on_change();
    for _ in 0..20 {
        sleep(Duration::from_millis(1));
        est.on_change();
    }
    // Frequent sub-100 ms changes must lower the average below the default.
    assert!(est.avg_secs < DEFAULT_SECS, "avg {} not below default", est.avg_secs);
    let d = est.sleep_duration();
    assert!(d >= floor() && d <= ceiling());
}

#[test]
fn idle_drifts_average_upward() {
    let mut est = IntervalEstimator::new();
    est.on_change();
    // Reduce the average first so a modest idle period exceeds it.
    sleep(Duration::from_millis(2));
    est.on_change();
    let active = est.avg_secs;
    // Now stay idle well beyond the current average and relax upward.
    sleep(Duration::from_millis(active.mul_add(1000.0, 30.0) as u64));
    est.on_no_change();
    assert!(
        est.avg_secs > active,
        "idle did not relax average: {} !> {}",
        est.avg_secs,
        active
    );
    assert!(est.avg_secs <= CEILING_SECS * 2.0, "drift exceeded cap");
}

#[test]
fn no_change_before_any_change_is_noop() {
    let mut est = IntervalEstimator::new();
    let before = est.avg_secs;
    est.on_no_change(); // last_change is None → early return
    assert_eq!(est.avg_secs, before);
}

#[test]
fn min_sleep_of_empty_is_half_default() {
    let est: [IntervalEstimator; 0] = [];
    assert_eq!(
        min_sleep_duration(est.iter()),
        Duration::from_secs_f64(DEFAULT_SECS / 2.0)
    );
}

#[test]
fn min_sleep_picks_smallest_average() {
    let mut fast = IntervalEstimator::new();
    fast.on_change();
    for _ in 0..10 {
        sleep(Duration::from_millis(1));
        fast.on_change();
    }
    let idle = IntervalEstimator::new();
    let estimators = [fast, idle];
    let d = min_sleep_duration(estimators.iter());
    // Driven by the faster source, below the idle source's 50 ms sleep.
    assert!(d < Duration::from_secs_f64(DEFAULT_SECS / 2.0));
    assert!(d >= floor());
}
