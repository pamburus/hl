use std::time::Duration;

// ---

/// Settings of a [`Follow`](crate::Follow) request.
///
/// Every setting has a default, so most callers need not set any.
#[derive(Clone, Debug)]
pub struct FollowOptions {
    /// The longest interval between two checks of a path that is polled.
    ///
    /// A polled path is checked more often while it is written to, down to every 10 milliseconds,
    /// and the interval grows back to this one while it stays idle. A shorter interval is taken as 10 milliseconds.
    pub poll_interval: Duration,
    /// How often a path is checked for replacement even without a notification,
    /// which catches a replacement by a file of the same size. A shorter interval is taken as 10 milliseconds.
    pub recheck_cadence: Duration,
    /// Whether a path that is missing or cannot be opened is waited for.
    ///
    /// Without it, following a path stops once no file can be opened at it.
    pub retry_missing: bool,
    /// Which paths may be followed with native notifications of the operating system.
    pub fallback_policy: FallbackPolicy,
}

impl Default for FollowOptions {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            recheck_cadence: Duration::from_secs(5),
            retry_missing: true,
            fallback_policy: FallbackPolicy::Conservative,
        }
    }
}

// ---

/// Decides which paths are followed with native notifications and which are polled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Native notifications are used only for paths on a file system known to be local,
    /// and any other path is polled, as notifications may not come for changes made over a network.
    #[default]
    Conservative,
    /// Native notifications are used for any path where they can be set up,
    /// for file systems that are local but not recognized as such.
    Optimistic,
    /// Every path is polled, as `tail -F` does with `---disable-inotify`,
    /// for systems where native notifications cannot be relied on at all.
    Poll,
}
