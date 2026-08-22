//! Clock abstraction. Production code uses [`SystemClock`]; tests use
//! [`FixedClock`] to make time-dependent logic deterministic.

use std::sync::Arc;
use time::OffsetDateTime;

/// Abstract clock so `core` and `domain` never touch `std::time` directly.
pub trait Clock: Send + Sync {
    /// Current time in UTC.
    fn now(&self) -> OffsetDateTime;
}

/// Real wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Test clock that returns a fixed value.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    instant: OffsetDateTime,
}

impl FixedClock {
    /// Build a fixed clock at the given instant.
    #[must_use]
    pub fn new(instant: OffsetDateTime) -> Self {
        Self { instant }
    }

    /// Return a clock pinned to the Unix epoch.
    #[must_use]
    pub fn epoch() -> Self {
        Self::new(OffsetDateTime::UNIX_EPOCH)
    }

    /// Return a shared `Arc<dyn Clock>` pinned to the Unix epoch. Useful
    /// for `AppState::default_minimal()` and tests.
    #[must_use]
    pub fn epoch_arc() -> Arc<dyn Clock> {
        Arc::new(Self::epoch())
    }

    /// Move the clock forward.
    #[must_use]
    pub fn advance(self, delta: time::Duration) -> Self {
        Self::new(self.instant + delta)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.instant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_returns_instant() {
        let instant = OffsetDateTime::UNIX_EPOCH;
        let clock = FixedClock::new(instant);
        assert_eq!(clock.now(), instant);
    }

    #[test]
    fn fixed_clock_advance() {
        let clock = FixedClock::epoch().advance(time::Duration::seconds(60));
        assert_eq!(clock.now(), OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(60));
    }

    #[test]
    fn system_clock_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SystemClock>();
    }
}