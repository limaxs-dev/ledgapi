//! UUIDv7 identifier and generator trait.
//!
//! Use [`Id`] instead of raw [`uuid::Uuid`] so the type system guarantees
//! we never mix a v4 random ID (from a peer system) with our time-ordered
//! v7 IDs.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Newtype around [`Uuid`] constrained to the v7 variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(Uuid);

impl Id {
    /// Generate a new UUIDv7 identifier using the global default generator.
    #[must_use]
    pub fn new() -> Self {
        Self::generate(&SystemIdGenerator)
    }

    /// Generate a new identifier using the provided generator.
    pub fn generate<G: IdGenerator>(generator: &G) -> Self {
        Self(generator.next_v7())
    }

    /// Wrap a raw [`Uuid`], panicking if it is not v7.
    ///
    /// # Panics
    /// Panics if `value` is not a UUIDv7.
    #[must_use]
    pub fn new_v7(value: Uuid) -> Self {
        assert_eq!(
            value.get_version_num(),
            7,
            "Id requires UUIDv7, got version {}",
            value.get_version_num()
        );
        Self(value)
    }

    /// Try to wrap a raw [`Uuid`], returning `None` if not v7.
    #[must_use]
    pub fn try_new_v7(value: Uuid) -> Option<Self> {
        if value.get_version_num() == 7 { Some(Self(value)) } else { None }
    }

    /// Borrow the inner [`Uuid`].
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Construct from a string. Returns `None` for invalid input or non-v7
    /// UUIDs.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let uuid = Uuid::parse_str(s).ok()?;
        Self::try_new_v7(uuid)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Id> for Uuid {
    fn from(value: Id) -> Self {
        value.0
    }
}

impl From<Uuid> for Id {
    fn from(value: Uuid) -> Self {
        // We accept any UUID and assert v7; callers needing to handle
        // non-v7 should use `Id::try_new_v7`.
        Self::new_v7(value)
    }
}

impl AsRef<Uuid> for Id {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

/// Generator trait for UUIDv7 identifiers.
pub trait IdGenerator: Send + Sync {
    /// Return the next UUIDv7.
    fn next_v7(&self) -> Uuid;
}

/// Default generator used when none is provided.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemIdGenerator;

impl IdGenerator for SystemIdGenerator {
    fn next_v7(&self) -> Uuid {
        Uuid::now_v7()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_v7() {
        let id = Id::new();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn parse_rejects_non_v7() {
        // v4 UUID
        let v4 = Uuid::from_bytes([0x55; 16]);
        assert!(Id::try_new_v7(v4).is_none());
        assert!(Id::parse(&v4.to_string()).is_none());
    }

    #[test]
    fn parse_accepts_v7() {
        let id = Id::new();
        let parsed = Id::parse(&id.to_string()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn generate_with_explicit_generator() {
        let id = Id::generate(&SystemIdGenerator);
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn new_v7_panics_on_non_v7() {
        let v4 = Uuid::from_bytes([0x55; 16]);
        let result = std::panic::catch_unwind(|| Id::new_v7(v4));
        assert!(result.is_err());
    }

    #[test]
    fn display_round_trip() {
        let id = Id::new();
        let s = id.to_string();
        let parsed = Id::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn ordering_is_monotonic() {
        let a = Id::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = Id::new();
        assert!(a < b, "UUIDv7 ordering must be monotonic: {a} >= {b}");
    }
}
