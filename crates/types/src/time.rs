//! Time, deadlines, revisions, and fencing tokens.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// UTC timestamp with fixed-offset serialization in RFC 3339.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UtcTimestamp(OffsetDateTime);

impl UtcTimestamp {
    pub const fn new(dt: OffsetDateTime) -> Self {
        Self(dt)
    }

    pub const fn as_offset(&self) -> OffsetDateTime {
        self.0
    }

    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    pub fn add_duration(self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }

    /// Adds a `std::time::Duration`, returning `None` on overflow.
    pub fn add_std_duration(self, duration: std::time::Duration) -> Option<Self> {
        let secs = i64::try_from(duration.as_secs()).ok()?;
        let nanos = i32::try_from(duration.subsec_nanos()).ok()?;
        let d = Duration::new(secs, nanos);
        self.add_duration(d)
    }

    pub fn is_before(&self, other: UtcTimestamp) -> bool {
        self.0 < other.0
    }

    /// Milliseconds since the Unix epoch.
    pub fn unix_millis(&self) -> i64 {
        (self.0.unix_timestamp_nanos() / 1_000_000).try_into().unwrap_or(i64::MAX)
    }

    pub fn is_after(&self, other: UtcTimestamp) -> bool {
        self.0 > other.0
    }
}

impl fmt::Display for UtcTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|_| fmt::Error)?
        )
    }
}

impl FromStr for UtcTimestamp {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dt = OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).map_err(
            |_| crate::Error::invalid_argument(format!("invalid RFC3339 timestamp: {}", s)),
        )?;
        Ok(Self(dt))
    }
}

/// Clock abstraction for deterministic tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> UtcTimestamp;
}

/// System clock.
#[derive(Debug, Clone, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> UtcTimestamp {
        UtcTimestamp::now()
    }
}

/// Fixed clock for tests.
#[derive(Debug, Clone)]
pub struct StaticClock {
    time: UtcTimestamp,
}

impl StaticClock {
    pub const fn new(time: UtcTimestamp) -> Self {
        Self { time }
    }

    pub fn set(&mut self, time: UtcTimestamp) {
        self.time = time;
    }
}

impl Clock for StaticClock {
    fn now(&self) -> UtcTimestamp {
        self.time
    }
}

/// Absolute deadline after which an operation should be cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Deadline(UtcTimestamp);

impl Deadline {
    pub fn new(ts: UtcTimestamp) -> Self {
        Self(ts)
    }

    pub fn after(clock: &dyn Clock, duration: Duration) -> Option<Self> {
        clock.now().add_duration(duration).map(Self)
    }

    /// Creates a deadline from a `std::time::Duration` relative to the clock.
    pub fn after_std(clock: &dyn Clock, duration: std::time::Duration) -> Option<Self> {
        let secs = i64::try_from(duration.as_secs()).ok()?;
        let nanos = i32::try_from(duration.subsec_nanos()).ok()?;
        let d = Duration::new(secs, nanos);
        Self::after(clock, d)
    }

    pub fn is_expired_at(&self, now: UtcTimestamp) -> bool {
        now.0 >= (self.0).0
    }

    pub const fn timestamp(&self) -> UtcTimestamp {
        self.0
    }
}

/// Monotonic revision for optimistic concurrency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(u64);

impl Revision {
    pub const fn initial() -> Self {
        Self(0)
    }

    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Returns the revision as an `i64` for PostgreSQL `BIGINT` columns.
    /// Errors if the revision is larger than `i64::MAX`, which would wrap on cast.
    pub fn as_i64(&self) -> Result<i64, crate::Error> {
        i64::try_from(self.0)
            .map_err(|_| crate::Error::invalid_argument("revision out of i64 range"))
    }
}

impl Default for Revision {
    fn default() -> Self {
        Self::initial()
    }
}

/// Fencing token used by background reconcilers to detect stale leaders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FencingToken(Uuid);

impl FencingToken {
    pub fn new_v7(generator: &dyn crate::IdGenerator) -> Self {
        Self(generator.v7())
    }

    pub fn new_v4(generator: &dyn crate::IdGenerator) -> Self {
        Self(generator.v4())
    }

    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for FencingToken {
    fn default() -> Self {
        Self(Uuid::nil())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_round_trip() {
        let ts = UtcTimestamp::now();
        let s = ts.to_string();
        let parsed: UtcTimestamp = s.parse().unwrap();
        assert_eq!(ts, parsed);
    }

    #[test]
    fn deadline_expiration() {
        let clock = StaticClock::new(UtcTimestamp::new(
            OffsetDateTime::from_unix_timestamp(0).unwrap(),
        ));
        let deadline = Deadline::after(&clock, Duration::seconds(10)).unwrap();
        assert!(!deadline.is_expired_at(clock.now()));
        let later = UtcTimestamp::new(OffsetDateTime::from_unix_timestamp(20).unwrap());
        assert!(deadline.is_expired_at(later));
    }

    #[test]
    fn revision_increments() {
        let rev = Revision::initial().next().next();
        assert_eq!(rev.as_u64(), 2);
    }

    #[test]
    fn revision_as_i64_roundtrip_and_overflow() {
        assert_eq!(Revision::from_u64(42).as_i64().unwrap(), 42);
        assert!(
            Revision::from_u64(u64::try_from(i64::MAX).unwrap().wrapping_add(1))
                .as_i64()
                .is_err()
        );
    }
}
