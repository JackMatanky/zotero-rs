//! Optimistic concurrency versioning for the Zotero Local API.
//!
//! [`LibraryVersion`] wraps the monotonically increasing version counter
//! Zotero attaches to libraries, items, and collections, used to detect
//! concurrent modification via the `If-Unmodified-Since-Version` request
//! header.

use serde::{Deserialize, Serialize};

/// A monotonically increasing library version counter for optimistic
/// concurrency control.
///
/// Pass the current version in an `If-Unmodified-Since-Version` request header.
/// If another client has modified the library since this version, the Zotero
/// server responds with `412 Precondition Failed`. Re-fetch the current
/// version before retrying the write.
///
/// # Examples
///
/// ```
/// use zotero_api::LibraryVersion;
///
/// let v = LibraryVersion::from(42_u64);
/// assert_eq!(v.as_u64(), 42);
/// // Use as the If-Unmodified-Since-Version header value:
/// assert_eq!(v.to_string(), "42");
/// ```
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Deserialize,
    Serialize,
)]
#[serde(transparent)]
pub struct LibraryVersion(u64);

impl LibraryVersion {
    /// Wraps a raw `u64` version counter.
    #[inline]
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying `u64` value.
    #[inline]
    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::ops::Deref for LibraryVersion {
    type Target = u64;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for LibraryVersion {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for LibraryVersion {
    #[inline]
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<LibraryVersion> for u64 {
    #[inline]
    fn from(value: LibraryVersion) -> Self {
        value.0
    }
}
