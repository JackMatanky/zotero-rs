//! Strongly-typed identifiers for Zotero objects.
//!
//! [`ItemKey`] and [`CollectionKey`] are 8-character alphanumeric keys that are
//! structurally identical but semantically distinct — the compiler rejects
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
string_newtype!(
    pub(crate) RelationUri,
    "A Zotero item relation URI of the form \
     `http://zotero.org/users/0/items/{KEY}` or \
     `http://zotero.org/groups/{ID}/items/{KEY}`. \
     [`From<&ItemKey>`](ItemKey) builds a `/users/0` URI for writes; \
     [`ItemKey::try_from`](ItemKey) recovers the trailing key on reads.",
);

/// Base URI for item relations in the Local API's `/users/0` namespace.
const ITEM_RELATION_URI_BASE: &str = "http://zotero.org/users/0/items/";

/// Error returned when a [`RelationUri`] does not contain a valid Zotero item
/// key as its trailing segment.
///
/// The URI must end with `/items/` followed by exactly 8 ASCII-alphanumeric
/// characters (e.g. `http://zotero.org/users/0/items/ABC12345`).
#[derive(Debug)]
pub(crate) struct RelationUriError;

impl From<&ItemKey> for RelationUri {
    #[inline]
    fn from(key: &ItemKey) -> Self {
        Self::new(format!("{ITEM_RELATION_URI_BASE}{}", key.as_str()))
    }
}

/// Extracts an [`ItemKey`] from a `RelationUri`.
///
/// Accepts both user-library (`/users/0/items/{KEY}`) and group-library
/// (`/groups/{ID}/items/{KEY}`) URI forms. The trailing segment must be exactly
/// 8 ASCII-alphanumeric characters.
///
/// # Errors
///
/// Returns `RelationUriError` if the URI lacks an `/items/` segment or the
/// trailing key is not 8 ASCII-alphanumeric characters.
impl TryFrom<&RelationUri> for ItemKey {
    type Error = RelationUriError;

    #[inline]
    fn try_from(uri: &RelationUri) -> Result<Self, Self::Error> {
        let value = uri.as_str();
        if !value.contains("/items/") {
            return Err(RelationUriError);
        }
        let Some(key) = value.rsplit('/').next() else {
            return Err(RelationUriError);
        };
        if key.len() == 8 && key.chars().all(|c| c.is_ascii_alphanumeric()) {
            Ok(ItemKey::from(key))
        } else {
            Err(RelationUriError)
        }
    }
}

impl std::fmt::Display for RelationUriError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a Zotero item URI")
    }
}

/// A monotonically increasing library version counter for optimistic
/// concurrency control.
///
/// Pass the current version in an `If-Unmodified-Since-Version` request header.
/// If another client has modified the library since this version, the Zotero
/// server responds with `412 Precondition Failed` — the caller must re-fetch
/// and retry.
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

    mod relation_uri {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn from_item_key_round_trips() {
            let key = ItemKey::from("ABC12345");
            let uri = RelationUri::from(&key);
            assert_eq!(
                uri.to_string(),
                "http://zotero.org/users/0/items/ABC12345"
            );

            let result = ItemKey::try_from(&uri);

            assert_eq!(result.as_ref().ok(), Some(&key));
        }

        #[test]
        fn try_from_extracts_key_from_user_library_uri() {
            let uri =
                RelationUri::from("http://zotero.org/users/0/items/ABC12345");
            let result = ItemKey::try_from(&uri);

            assert_eq!(
                result.ok().as_ref().map(ItemKey::as_str),
                Some("ABC12345")
            );
        }

        #[test]
        fn try_from_extracts_key_from_group_library_uri() {
            let uri = RelationUri::from(
                "http://zotero.org/groups/36222/items/E6IGUT5Z",
            );
            let result = ItemKey::try_from(&uri);

            assert_eq!(
                result.ok().as_ref().map(ItemKey::as_str),
                Some("E6IGUT5Z")
            );
        }

        #[test]
        fn try_from_rejects_bare_item_key_string() {
            let uri = RelationUri::from("ITEM123");
            assert!(
                ItemKey::try_from(&uri).is_err(),
                "bare non-URI key must be rejected"
            );

            let full_length_key = RelationUri::from("ABCDEFGH");
            assert!(
                ItemKey::try_from(&full_length_key).is_err(),
                "bare eight-character key must be rejected"
            );
        }

        #[test]
        fn try_from_rejects_malformed_uris() {
            let empty = RelationUri::from("");
            assert!(
                ItemKey::try_from(&empty).is_err(),
                "empty URI must be rejected"
            );

            let no_items_segment =
                RelationUri::from("http://zotero.org/users/0/ABC12345");
            assert!(
                ItemKey::try_from(&no_items_segment).is_err(),
                "URI without /items/ segment must be rejected"
            );

            let bad_key_shape =
                RelationUri::from("http://zotero.org/users/0/items/ABC");
            assert!(
                ItemKey::try_from(&bad_key_shape).is_err(),
                "short trailing key must be rejected"
            );

            let no_trailing_key =
                RelationUri::from("http://zotero.org/users/0/items/");
            assert!(
                ItemKey::try_from(&no_trailing_key).is_err(),
                "URI without trailing key must be rejected"
            );
        }
    }
}
