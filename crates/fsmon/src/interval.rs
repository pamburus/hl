use std::time::{Duration, Instant};

// ---

/// The weight of the latest interval in the average.
const ALPHA: f64 = 0.25;
/// The shortest interval between two checks.
pub(crate) const FLOOR: Duration = Duration::from_millis(10);
/// The average interval between changes assumed before any are seen, which makes the first checks 50 ms apart.
const DEFAULT_AVERAGE: Duration = Duration::from_millis(100);

// ---

/// Adapts the interval between checks of a polled file to how often it changes.
///
/// It keeps an exponential moving average of the intervals between changes, and checks twice per average interval,
/// within the floor of 10 ms and the ceiling it is given. While the file stays idle for longer than the average,
/// the average grows, so a quiet file is checked less and less often, down to once per ceiling.
pub(crate) struct IntervalEstimator {
    average: f64,
    ceiling: f64,
    /// When the checks started, which is where the idle time is counted from until the first change.
    start: Instant,
    last_change: Option<Instant>,
}

impl IntervalEstimator {
    /// Creates an estimator for checks starting at `start`, whose interval grows to `ceiling` at most, which is raised
    /// to the floor if below it.
    pub fn new(ceiling: Duration, start: Instant) -> Self {
        Self {
            average: DEFAULT_AVERAGE.as_secs_f64(),
            ceiling: ceiling.max(FLOOR).as_secs_f64(),
            start,
            last_change: None,
        }
    }

    /// Records that a check made at `now` found a change.
    pub fn on_change(&mut self, now: Instant) {
        if let Some(prev) = self.last_change.replace(now) {
            let interval = now.saturating_duration_since(prev).as_secs_f64();
            self.update(interval);
        }
    }

    /// Records that a check made at `now` found no change.
    pub fn on_no_change(&mut self, now: Instant) {
        let idle = now
            .saturating_duration_since(self.last_change.unwrap_or(self.start))
            .as_secs_f64();
        if idle > self.average {
            self.update(idle);
        }
    }

    /// Adds an interval to the average.
    fn update(&mut self, interval: f64) {
        // Capped, so that it does not take long to adapt once changes come again after a long idle time.
        self.average = (ALPHA * interval + (1.0 - ALPHA) * self.average).min(self.ceiling * 2.0);
    }

    /// Returns how long to wait before the next check.
    pub fn interval(&self) -> Duration {
        Duration::from_secs_f64((self.average / 2.0).clamp(FLOOR.as_secs_f64(), self.ceiling))
    }
}

// ---

#[cfg(test)]
mod tests;
