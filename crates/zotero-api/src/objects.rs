//! Zotero Local API JSON objects and payload data structures.
//!
//! Defines serde deserialization shapes for Zotero items, collections,
//! creators, tags, annotations, and local API availability status. These types
//! form the core data model returned by Zotero HTTP endpoints.
//!
//! # Main Types
//!
//! - [`ZoteroItem`]: A single library item with metadata envelope.
//! - [`ZoteroItemData`]: Bibliographic, attachment, note, and annotation
//!   fields.
//! - [`ZoteroCreator`]: Author or editor credited on an item.
//! - [`ZoteroTag`]: Tag attached to an item.
//! - [`ZoteroCollection`]: A collection hierarchy node.
//! - [`ZoteroCollectionData`]: Collection metadata payload.
//! - [`LocalApiStatus`]: Local API availability probe result.

use serde::{Deserialize, Serialize};

use crate::{
    keys::{CollectionKey, ItemKey, LibraryVersion, TagName},
    types::{CollectionParent, CreatorType, ItemType, LinkMode, TagOrigin},
};

/// Response returned by batch create and update operations.
///
/// Each field is a JSON value mapping payload indices or item keys to their
/// results. [`successful`](Self::successful) contains keys for items that were
/// created or updated, [`unchanged`](Self::unchanged) lists keys of items
/// whose data matched the existing version, and
/// [`failed`](Self::failed) maps keys to error details.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct BatchWriteResponse {
    /// Successful item keys mapped by payload index or temporary key.
    #[serde(default)]
    pub successful: serde_json::Value,
    /// Unchanged item keys.
    #[serde(default)]
    pub unchanged: serde_json::Value,
    /// Failed items mapped by key or index to error details.
    #[serde(default)]
    pub failed: serde_json::Value,
}

/// Payload for creating a new Zotero item.
///
/// Use directly or build via [`crate::metadata::resolve_metadata`] (behind the
/// `metadata` feature) to auto-populate fields from a DOI, arXiv ID, or ISBN.
/// Pass to [`ZoteroClient::create_item_from_metadata`](crate::ZoteroClient::create_item_from_metadata)
/// to persist.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDraft {
    /// Zotero item type (e.g. journal article, preprint, or book).
    #[serde(rename = "itemType")]
    pub item_type: ItemType,
    /// Title of the publication.
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<ZoteroCreator>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(rename = "DOI", default, skip_serializing_if = "String::is_empty")]
    pub doi: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publication_title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub abstract_note: String,
    #[serde(
        rename = "ISBN",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub isbn: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publisher: String,
    /// Collections that should contain the created item.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<CollectionKey>,
}

/// Library descriptor embedded in item and collection responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LibraryInfo {
    /// Numeric library identifier.
    #[serde(default)]
    pub id: u64,
    /// Library type: `"user"` or `"group"`.
    #[serde(rename = "type", default)]
    pub type_: String,
    /// Group library name, absent for personal libraries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// HATEOAS link objects for an item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ItemLinks {
    /// API endpoint for this item (`self`).
    #[serde(rename = "self", default, skip_serializing_if = "Option::is_none")]
    pub self_link: Option<serde_json::Value>,
    /// Alternate web view URL (`alternate`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternate: Option<serde_json::Value>,
    /// Enclosure link for attachment downloads (`enclosure`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enclosure: Option<serde_json::Value>,
}

impl ItemLinks {
    /// Returns the link object for `"self"`, `"alternate"`, or `"enclosure"`.
    #[must_use]
    #[inline]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        match key {
            "self" => self.self_link.as_ref(),
            "alternate" => self.alternate.as_ref(),
            "enclosure" => self.enclosure.as_ref(),
            _ => None,
        }
    }

    /// Returns the `href` URL string of the named link, if present.
    #[must_use]
    #[inline]
    pub fn href(&self, key: &str) -> Option<&str> {
        self.get(key)?.get("href")?.as_str()
    }
}

/// Summary metadata counters for an item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ItemMeta {
    /// Number of child items (notes, attachments, annotations).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_children: Option<usize>,
    /// Number of collections containing this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_collections: Option<usize>,
}

/// A single Zotero library item as returned by the Local API.
///
/// Wraps the item's [`key`](Self::key) and [`version`](Self::version) envelope
/// around its [`data`](Self::data) payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoteroItem {
    /// Unique 8-character item key.
    pub key: ItemKey,
    /// Item version for optimistic concurrency.
    pub version: LibraryVersion,
    #[serde(default)]
    pub library: Option<LibraryInfo>,
    /// HATEOAS link objects.
    #[serde(default)]
    pub links: Option<ItemLinks>,
    /// Summary metadata: child and collection counts.
    #[serde(default)]
    pub meta: Option<ItemMeta>,
    /// Bibliographic, attachment, note, and annotation fields.
    pub data: ZoteroItemData,
}

/// Core data payload for a Zotero item.
///
/// Contains the standard bibliographic fields ([`key`](Self::key),
/// [`item_type`](Self::item_type), [`title`](Self::title),
/// [`creators`](Self::creators), [`tags`](Self::tags)) plus item-type-specific
/// fields captured in [`extra_fields`](Self::extra_fields). Access any field
/// generically via [`get_str`](Self::get_str) or
/// [`set_field`](Self::set_field).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroItemData {
    /// Unique 8-character item key.
    pub key: ItemKey,
    #[serde(default)]
    pub version: LibraryVersion,
    #[serde(rename = "itemType", default)]
    pub item_type: ItemType,
    pub title: Option<String>,
    #[serde(default)]
    pub creators: Vec<ZoteroCreator>,
    #[serde(default)]
    pub tags: Vec<ZoteroTag>,
    #[serde(default)]
    pub collections: Vec<CollectionKey>,
    #[serde(default)]
    pub relations: serde_json::Value,
    #[serde(rename = "dateAdded", default)]
    pub date_added: Option<String>,
    #[serde(rename = "dateModified", default)]
    pub date_modified: Option<String>,
    #[serde(default)]
    pub deleted: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abstract_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub institution: Option<String>,
    #[serde(rename = "DOI", default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation_key: Option<String>,
    #[serde(rename = "ISBN", default, skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
    #[serde(rename = "ISSN", default, skip_serializing_if = "Option::is_none")]
    pub issn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_page_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Catch-all for item-type-specific fields not covered above.
    #[serde(flatten)]
    pub extra_fields: std::collections::HashMap<String, serde_json::Value>,
}

impl ZoteroItemData {
    /// Returns a string reference to a core field or dynamic extra field.
    ///
    /// Recognizes `"key"`, `"itemType"`, `"title"`, `"dateAdded"`, and
    /// `"dateModified"` as built-in fields. Anything else is looked up in
    /// [`extra_fields`](Self::extra_fields). Returns `None` if the field is
    /// absent or not string-valued.
    pub fn get_str(&self, field: &str) -> Option<&str> {
        match field {
            "key" => Some(self.key.as_str()),
            "itemType" => Some(self.item_type.as_str()),
            "title" => self.title.as_deref(),
            "dateAdded" => self.date_added.as_deref(),
            "dateModified" => self.date_modified.as_deref(),
            _ => self.extra_fields.get(field).and_then(|v| v.as_str()),
        }
    }

    /// Alias for [`get_str`](Self::get_str).
    pub fn get_field(&self, field: &str) -> Option<&str> {
        self.get_str(field)
    }

    /// Inserts or overwrites a dynamic field in
    /// [`extra_fields`](Self::extra_fields).
    pub fn set_field<K: Into<String>, V: Into<serde_json::Value>>(
        &mut self,
        key: K,
        value: V,
    ) {
        self.extra_fields.insert(key.into(), value.into());
    }

    /// Attachment storage mode from the `"linkMode"` extra field.
    ///
    /// Returns `None` if the field is absent or the value is not a recognized
    /// [`LinkMode`] variant.
    pub fn link_mode(&self) -> Option<LinkMode> {
        self.extra_fields
            .get("linkMode")
            .and_then(|v| v.as_str())
            .map(|s| LinkMode::from(s.to_owned()))
    }
}

/// An author, editor, or other creator credited on an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoteroCreator {
    /// Creator role (e.g. `"author"`, `"editor"`).
    #[serde(rename = "creatorType")]
    pub creator_type: Option<CreatorType>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    /// Single-field name for institutional or single-field creators.
    pub name: Option<String>,
}

/// A tag attached to an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoteroTag {
    /// Tag name.
    pub tag: TagName,
    /// Whether the tag was user-created or auto-assigned on import.
    #[serde(rename = "type", default)]
    pub origin: TagOrigin,
}
/// A Zotero collection as returned by the Local API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoteroCollection {
    /// Unique 8-character collection key.
    pub key: CollectionKey,
    pub(crate) version: LibraryVersion,
    pub(crate) data: ZoteroCollectionData,
}

/// Metadata payload for a [`ZoteroCollection`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ZoteroCollectionData {
    pub(crate) key: CollectionKey,
    /// Human-readable collection name.
    pub(crate) name: String,
    /// Parent collection state: top-level or a reference to a parent key.
    #[serde(rename = "parentCollection", default)]
    pub(crate) parent_collection: CollectionParent,
}

/// Result of probing the Zotero Local API for availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalApiStatus {
    /// Whether the Local API is reachable.
    pub online: bool,
    /// Base URL of the probed endpoint.
    pub url: String,
    /// Zotero data version string, if the server responded.
    pub version: Option<String>,
    /// Error message if the probe failed.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod deserialization {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn leaves_omitted_optional_fields_as_none() {
            let raw_json = serde_json::json!({
                "key": "ABC12345",
                "version": 42,
                "data": {
                    "key": "ABC12345",
                    "version": 42,
                    "itemType": "journalArticle",
                    "title": "Quantum Computing Advances"
                }
            });

            let result = serde_json::from_value(raw_json);
            assert!(result.is_ok(), "item JSON should deserialize: {result:?}");
            let item: ZoteroItem = result.expect("asserted Ok above");

            assert_eq!(item.key, "ABC12345");
            assert_eq!(
                item.data.title.as_deref(),
                Some("Quantum Computing Advances")
            );
            assert!(item.data.doi.is_none());
        }

        #[test]
        fn parses_creator_camel_case_names() {
            let raw_json = serde_json::json!({
                "creatorType": "author",
                "firstName": "Ada",
                "lastName": "Lovelace"
            });
            let result = serde_json::from_value(raw_json);
            assert!(
                result.is_ok(),
                "creator JSON should deserialize: {result:?}"
            );
            let creator: ZoteroCreator = result.expect("asserted Ok above");

            assert_eq!(creator.creator_type, Some(CreatorType::Author));
            assert_eq!(creator.first_name.as_deref(), Some("Ada"));
            assert_eq!(creator.last_name.as_deref(), Some("Lovelace"));
        }

        #[test]
        fn defaults_deleted_to_false_when_absent() {
            let raw_json = serde_json::json!({
                "key": "ABC12345",
                "version": 42,
                "data": {
                    "key": "ABC12345",
                    "version": 42,
                    "itemType": "journalArticle"
                }
            });

            let result = serde_json::from_value(raw_json);
            assert!(result.is_ok(), "item JSON should deserialize: {result:?}");
            let item: ZoteroItem = result.expect("asserted Ok above");

            assert!(!item.data.deleted);
        }

        #[test]
        fn round_trips_deleted_flag() {
            let raw_json = serde_json::json!({
                "key": "ABC12345",
                "version": 42,
                "itemType": "journalArticle",
                "deleted": true
            });

            let result = serde_json::from_value(raw_json);
            assert!(
                result.is_ok(),
                "item data JSON should deserialize: {result:?}"
            );
            let data: ZoteroItemData = result.expect("asserted Ok above");
            assert!(data.deleted, "deleted flag should deserialize as true");

            let serialized = serde_json::to_string(&data);
            assert!(
                serialized.is_ok(),
                "item data should serialize: {serialized:?}"
            );
            let serialized = serialized.unwrap_or_default();
            assert!(
                serialized.contains("\"deleted\":true"),
                "serialized data must contain deleted flag: {serialized}"
            );
        }

        #[test]
        fn promoted_metadata_fields_are_omitted_when_absent() {
            let raw_json = serde_json::json!({
                "key": "ABC12345",
                "version": 42,
                "itemType": "journalArticle"
            });

            let data: ZoteroItemData = serde_json::from_value(raw_json)
                .expect("item data should deserialize");
            let serialized = serde_json::to_string(&data)
                .expect("item data should serialize");

            for wire_key in [
                "abstractNote",
                "publicationTitle",
                "volume",
                "issue",
                "pages",
                "date",
                "publisher",
                "institution",
                "DOI",
                "citationKey",
                "ISBN",
                "ISSN",
                "url",
                "extra",
                "note",
                "parentItem",
                "annotationType",
                "annotationText",
                "annotationComment",
                "annotationColor",
                "annotationPageLabel",
                "contentType",
                "filename",
                "path",
            ] {
                assert!(
                    !serialized.contains(&format!("\"{wire_key}\"")),
                    "absent field {wire_key} must be omitted, not serialized \
                     as null: {serialized}"
                );
            }
        }

        #[test]
        fn parses_native_citation_key_field() {
            let raw_json = serde_json::json!({
                "key": "ABC12345",
                "version": 42,
                "itemType": "journalArticle",
                "citationKey": "smith2020deep"
            });

            let result = serde_json::from_value(raw_json);
            assert!(
                result.is_ok(),
                "item data JSON should deserialize: {result:?}"
            );
            let data: ZoteroItemData = result.expect("asserted Ok above");
            assert_eq!(data.citation_key.as_deref(), Some("smith2020deep"));
        }

        #[test]
        fn defaults_tag_origin_to_user_when_type_is_absent() {
            let raw_json = serde_json::json!({"tag": "rust"});

            let result = serde_json::from_value(raw_json);
            assert!(result.is_ok(), "tag JSON should deserialize: {result:?}");
            let tag: ZoteroTag = result.expect("asserted Ok above");

            assert_eq!(tag.origin, TagOrigin::User);
        }

        #[test]
        fn deserializes_collection_with_parent_collection_key() {
            let raw_json = serde_json::json!({
                "key": "COL12345",
                "version": 10,
                "data": {
                    "key": "COL12345",
                    "version": 10,
                    "name": "Machine Learning",
                    "parentCollection": "PARENT01"
                }
            });

            let result = serde_json::from_value(raw_json);
            assert!(
                result.is_ok(),
                "collection JSON should deserialize: {result:?}"
            );
            let col: ZoteroCollection = result.expect("asserted Ok above");
            assert_eq!(col.key, "COL12345");
            assert_eq!(col.data.name, "Machine Learning");
            assert_eq!(
                col.data.parent_collection,
                CollectionParent::Parent(CollectionKey::from("PARENT01"))
            );
        }

        #[test]
        fn deserializes_uppercase_doi_isbn_issn_fields() {
            let raw_json = serde_json::json!({
                "key": "ABC12345",
                "version": 42,
                "itemType": "journalArticle",
                "DOI": "10.1234/example",
                "ISBN": "978-0-13-468599-1",
                "ISSN": "1234-5678"
            });

            let result = serde_json::from_value(raw_json);
            assert!(
                result.is_ok(),
                "item data JSON should deserialize: {result:?}"
            );
            let data: ZoteroItemData = result.expect("asserted Ok above");

            assert_eq!(data.doi.as_deref(), Some("10.1234/example"));
            assert_eq!(data.isbn.as_deref(), Some("978-0-13-468599-1"));
            assert_eq!(data.issn.as_deref(), Some("1234-5678"));
        }
    }

    mod item_links {
        use pretty_assertions::assert_eq;

        use super::*;

        fn links_with_enclosure_href(href: &str) -> ItemLinks {
            ItemLinks {
                self_link: None,
                alternate: None,
                enclosure: Some(serde_json::json!({"href": href})),
            }
        }

        #[test]
        fn href_returns_enclosure_url() {
            let links = links_with_enclosure_href("file:///tmp/paper.pdf");

            assert_eq!(links.href("enclosure"), Some("file:///tmp/paper.pdf"));
        }

        #[test]
        fn href_returns_none_for_unset_link() {
            let links = links_with_enclosure_href("file:///tmp/paper.pdf");

            assert_eq!(links.href("self"), None);
        }

        #[test]
        fn href_returns_none_for_unknown_key() {
            let links = links_with_enclosure_href("file:///tmp/paper.pdf");

            assert_eq!(links.href("missing"), None);
        }
    }
}
