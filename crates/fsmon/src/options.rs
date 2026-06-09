use std::time::Duration;

// ---

/// Consumer-tunable settings for following a file. Every field has a sensible
/// default so most callers need not set anything.
#[derive(Clone, Debug)]
pub struct FollowOptions {
    /// How often to poll paths on unreliable (non-local) filesystems.
    pub poll_interval: Duration,
    /// Safety-net re-check cadence: re-stat all sources even without an event
    /// to catch same-size replacements (FR-006).
    pub recheck_cadence: Duration,
    /// When true, keep retrying a missing path until it reappears (FR-002,
    /// FR-007). When false, return `None` from `next_chunk` once a path
    /// permanently disappears.
    pub retry_missing: bool,
    /// Controls how aggressively native notifications are used (FR-009–FR-011,
    /// FR-023).
    pub fallback_policy: FallbackPolicy,
    /// Read buffer size (bytes). Data is never buffered beyond this in memory;
    /// unconsumed bytes for regular files stay on disk (FR-015a).
    pub read_buffer: usize,
}

impl Default for FollowOptions {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            recheck_cadence: Duration::from_secs(5),
            retry_missing: true,
            fallback_policy: FallbackPolicy::Conservative,
            read_buffer: 64 * 1024,
        }
    }
}

// ---

/// Controls when native filesystem notifications are used vs. polling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Any path not affirmatively known-local is polled. This is the safe
    /// default and the only policy exercised by hl.
    #[default]
    Conservative,
    /// Trust native notifications even for paths that could not be classified
    /// as local. Useful on exotic mounts where classification is wrong.
    Optimistic,
}
