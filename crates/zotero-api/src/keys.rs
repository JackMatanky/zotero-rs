//! Strongly-typed identifiers for Zotero objects.
//!
//! [`ItemKey`] and [`CollectionKey`] are 8-character alphanumeric keys that are
//! structurally identical but semantically distinct, so the compiler rejects
//! accidental transposition. [`TagName`] wraps a plain string for the same
//! reason.
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
