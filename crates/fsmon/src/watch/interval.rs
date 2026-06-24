// IIR-based adaptive poll-interval estimator.
//
// Tracks the exponential moving average of observed inter-change intervals and
// uses half that average as the next sleep duration, clamped to [floor(), ceiling()].
// When the file is idle the estimate drifts upward so a quiet source relaxes
// toward the ceiling rather than pinning the sleep at its last active rate.
//
// Ported verbatim from src/fsmon/windows.rs (commit 3c8e6162) and made
// cross-platform so the same logic serves both the Windows hybrid backend and
// the adaptive poll backend used for network shares and other unreliable sources.

use std::time::{Duration, Instant};

// ---

const IIR_ALPHA: f64 = 0.25;
const FLOOR_SECS: f64 = 0.010; // 10 ms
const DEFAULT_SECS: f64 = 0.100; // 100 ms → initial sleep of 50 ms
const CEILING_SECS: f64 = 1.0; // 1 s

// ---

pub struct IntervalEstimator {
    avg_secs: f64,
    last_change: Option<Instant>,
}

impl IntervalEstimator {
    pub fn new() -> Self {
        Self {
            avg_secs: DEFAULT_SECS,
            last_change: None,
        }
    }

    pub fn on_change(&mut self) {
        let now = Instant::now();
        if let Some(prev) = self.last_change.replace(now) {
            let dt = now.duration_since(prev).as_secs_f64();
            self.avg_secs = IIR_ALPHA * dt + (1.0 - IIR_ALPHA) * self.avg_secs;
        }
    }

    // When idle time exceeds the current average, drift the estimate upward so
    // that a quiet file relaxes toward the ceiling rather than keeping the
    // shared sleep interval pinned at its last active rate.
    pub fn on_no_change(&mut self) {
        let Some(last) = self.last_change else { return };
        let idle = last.elapsed().as_secs_f64();
        if idle > self.avg_secs {
            self.avg_secs = IIR_ALPHA * idle + (1.0 - IIR_ALPHA) * self.avg_secs;
            // Cap so recovery from a long idle period isn't too slow.
            self.avg_secs = self.avg_secs.min(CEILING_SECS * 2.0);
        }
    }

    #[allow(dead_code)]
    pub fn sleep_duration(&self) -> Duration {
        Duration::from_secs_f64((self.avg_secs / 2.0).clamp(FLOOR_SECS, CEILING_SECS))
    }
}

pub fn min_sleep_duration<'a>(estimators: impl Iterator<Item = &'a IntervalEstimator>) -> Duration {
    let min = estimators.fold(f64::INFINITY, |acc, e| acc.min(e.avg_secs));
    let secs = if min.is_infinite() {
        DEFAULT_SECS / 2.0
    } else {
        (min / 2.0).clamp(FLOOR_SECS, CEILING_SECS)
    };
    Duration::from_secs_f64(secs)
}

// ---

#[cfg(test)]
mod tests;
