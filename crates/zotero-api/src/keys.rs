//! Strongly-typed identifiers for Zotero objects.
//!
//! [`ItemKey`] and [`CollectionKey`] are 8-character alphanumeric keys that are
//! structurally identical but semantically distinct, so the compiler rejects
//! accidental transposition. [`TagName`] and [`CitationKey`] wrap plain strings
//! for the same reason. [`LibraryVersion`] is a monotonically increasing
//! counter used for [optimistic concurrency control](LibraryVersion#examples).
//!
//! # Examples
//!
//! ```
//! use zotero_api::{CollectionKey, ItemKey};
//!
//! let item = ItemKey::from("ABC12345");
//! let col = CollectionKey::from("COL12345");
//! assert_eq!(item.as_str(), "ABC12345");
//! assert_eq!(col.as_str(), "COL12345");
//! ```

use serde::{Deserialize, Serialize};

string_newtype!(
    pub ItemKey,
    "An 8-character alphanumeric key that identifies a Zotero item within a \
     library. Structurally identical to [`CollectionKey`] but type-distinct \
     to prevent accidental transposition.",
);
string_newtype!(
    pub CollectionKey,
    "An 8-character alphanumeric key that identifies a Zotero collection \
     within a library. Structurally identical to [`ItemKey`] but type-distinct \
     to prevent accidental transposition.",
);
string_newtype!(
    pub TagName,
    "A Zotero tag name, wrapped to prevent transposition with item keys or \
     free-text query strings.",
);
string_newtype!(
    pub CitationKey,
    "A Zotero citation key, wrapped to enforce type safety across search \
     and item metadata.",
);

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

#[cfg(test)]
mod tests {
    use super::*;

    mod string_key {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn implements_display_from_and_equality_comparisons() {
            let item_key = ItemKey::from("ITEM123");
            assert_eq!(item_key.to_string(), "ITEM123");
            assert_eq!(item_key.as_ref(), "ITEM123");
            assert_eq!(item_key, "ITEM123");
            assert_eq!(item_key.to_string(), "ITEM123".to_owned());

            let col_key = CollectionKey::from("COL123".to_owned());
            assert_eq!(col_key.to_string(), "COL123");
            assert_eq!(col_key, "COL123");
        }
    }
}
