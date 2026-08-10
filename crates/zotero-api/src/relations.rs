//! Related-item relations for the Zotero Local HTTP API.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    client::ZoteroClient, errors::ZoteroApiError, keys::ItemKey,
    types::ItemType,
};

/// A minimal reference to a related item, resolved from a `dc:relation` URI.
///
/// Each [`RelatedItem`] corresponds to one URI in an item's `relations` map.
/// The title and item type are fetched from the related item's data during
/// resolution in [`ZoteroClient::get_related_items`].
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RelatedItem {
    /// Item key of the related item.
    pub key: ItemKey,
    /// Item title, if present.
    pub title: Option<String>,
    /// Item type of the related item.
    pub item_type: ItemType,
}

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
/// - `RelationUriError` if the URI lacks an `/items/` segment
/// - `RelationUriError` if the trailing key is not 8 ASCII-alphanumeric
///   characters
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

/// Error returned when a [`RelationUri`] does not contain a valid Zotero item
/// key as its trailing segment.
///
/// The URI must end with `/items/` followed by exactly 8 ASCII-alphanumeric
/// characters (e.g. `http://zotero.org/users/0/items/ABC12345`).
#[derive(Debug)]
pub(crate) struct RelationUriError;

impl std::fmt::Display for RelationUriError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a Zotero item URI")
    }
}

/// Direction for updating a bidirectional `dc:relation` link between two
/// items.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum RelationAction {
    /// Add each item's URI to the other's `dc:relation` map.
    Add,
    /// Remove each item's URI from the other's `dc:relation` map.
    Remove,
}

impl ZoteroClient {
    /// Fetches all related items linked to `item_key` via `dc:relation` URIs.
    ///
    /// Each URI in the item's `relations` map is resolved by fetching the
    /// corresponding item. URIs that don't match the Zotero item key format
    /// (e.g. external URLs) are silently skipped, as are items that return 404.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if `item_key` does not exist
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    ///
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn get_related_items<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<Vec<RelatedItem>, ZoteroApiError> {
        let item = self.get_item(item_key.as_ref()).await?;
        let mut related = Vec::new();
        for uri in parse_relation_keys(&item.data.relations) {
            let Ok(key) = ItemKey::try_from(&uri) else {
                continue;
            };
            match self.get_item(&key).await {
                Ok(related_item) => related.push(RelatedItem {
                    key: related_item.key,
                    title: related_item.data.title,
                    item_type: related_item.data.item_type,
                }),
                Err(ZoteroApiError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(related)
    }

    /// Links item `a` and item `b` by adding each item's URI to the other's
    /// `dc:relation` map.
    ///
    /// The two PATCH requests are issued sequentially. If the second fails, the
    /// first has already succeeded, resulting in a one-directional relation.
    /// Callers should retry or reconcile on error.
    ///
    /// # Errors
    ///
    /// - [`InputRejected`] if `a` and `b` are the same key
    /// - [`NotFound`] if either item does not exist
    /// - [`LocalApi`] if Zotero rejects either update
    /// - [`Network`] on connection failure
    ///
    /// [`InputRejected`]: ZoteroApiError::InputRejected
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn add_item_relation<K: AsRef<str>, V: AsRef<str>>(
        &self,
        a: K,
        b: V,
    ) -> Result<(), ZoteroApiError> {
        let key_a = a.as_ref();
        let key_b = b.as_ref();
        if key_a == key_b {
            return Err(ZoteroApiError::InputRejected(
                "cannot relate an item to itself".to_owned(),
            ));
        }
        self.set_relation(key_a, key_b, RelationAction::Add).await
    }

    /// Removes the relation between item `a` and item `b`.
    ///
    /// Each item's `dc:relation` URI pointing to the other is removed. Like
    /// [`ZoteroClient::add_item_relation`], the two updates are non-atomic.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if either item does not exist
    /// - [`LocalApi`] if Zotero rejects either update
    /// - [`Network`] on connection failure
    ///
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn remove_item_relation<K: AsRef<str>, V: AsRef<str>>(
        &self,
        a: K,
        b: V,
    ) -> Result<(), ZoteroApiError> {
        self.set_relation(a.as_ref(), b.as_ref(), RelationAction::Remove).await
    }

    /// Adds or removes the bidirectional `dc:relation` link between items
    /// `key_a` and `key_b`, depending on `action`.
    async fn set_relation(
        &self,
        key_a: &str,
        key_b: &str,
        action: RelationAction,
    ) -> Result<(), ZoteroApiError> {
        let item_key_a = ItemKey::from(key_a);
        let item_key_b = ItemKey::from(key_b);
        let a_item = self.get_item(key_a).await?;
        let b_item = self.get_item(key_b).await?;
        let a_relations = apply_relations(
            &a_item.data.relations,
            action,
            &RelationUri::from(&item_key_b),
        );
        let b_relations = apply_relations(
            &b_item.data.relations,
            action,
            &RelationUri::from(&item_key_a),
        );
        self.update_item(
            key_a,
            serde_json::json!({
                "relations": a_relations,
                "version": a_item.version,
            }),
        )
        .await?;
        self.update_item(
            key_b,
            serde_json::json!({
                "relations": b_relations,
                "version": b_item.version,
            }),
        )
        .await?;
        Ok(())
    }
}

/// Extracts `dc:relation` URIs from an item's `relations` JSON value.
///
/// Handles both single-string and array forms. Returns an empty vec if the
/// key is missing or the value type is unexpected.
pub(crate) fn parse_relation_keys(
    relations: &serde_json::Value,
) -> Vec<RelationUri> {
    let Some(dc_relation) = relations.get("dc:relation") else {
        return Vec::new();
    };
    match dc_relation {
        serde_json::Value::String(uri) => vec![RelationUri::from(uri.as_str())],
        serde_json::Value::Array(uris) => uris
            .iter()
            .filter_map(|v| v.as_str().map(RelationUri::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// Returns a new `relations` JSON value with `uri` added or removed,
/// depending on `action`.
pub(crate) fn apply_relations(
    current: &serde_json::Value,
    action: RelationAction,
    uri: &RelationUri,
) -> serde_json::Value {
    let mut uris: BTreeSet<String> = parse_relation_keys(current)
        .into_iter()
        .map(|u| u.as_str().to_owned())
        .collect();
    match action {
        RelationAction::Add => {
            uris.insert(uri.as_str().to_owned());
        }
        RelationAction::Remove => {
            uris.remove(uri.as_str());
        }
    }
    let mut result: serde_json::Map<String, serde_json::Value> =
        current.as_object().cloned().unwrap_or_default();
    result.insert(
        "dc:relation".to_owned(),
        serde_json::Value::Array(
            uris.into_iter().map(serde_json::Value::String).collect(),
        ),
    );
    serde_json::Value::Object(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const URI_A: &str = "http://zotero.org/users/0/items/AAAAAAAA";
    const URI_B: &str = "http://zotero.org/users/0/items/BBBBBBBB";

    mod parse_relation_keys {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn extracts_uris_from_array_form() {
            let relations = serde_json::json!({
                "dc:relation": [URI_A, URI_B]
            });

            let uris = super::parse_relation_keys(&relations);

            assert_eq!(uris, vec![
                RelationUri::from(URI_A),
                RelationUri::from(URI_B)
            ]);
        }

        #[test]
        fn extracts_uri_from_single_string_form() {
            let relations = serde_json::json!({ "dc:relation": URI_A });

            let uris = super::parse_relation_keys(&relations);

            assert_eq!(uris, vec![RelationUri::from(URI_A)]);
        }

        #[test]
        fn returns_empty_when_relations_missing() {
            let empty = super::parse_relation_keys(&serde_json::json!({}));
            assert!(empty.is_empty());
        }
    }

    mod apply_relations {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn adds_new_uri() {
            let result = super::apply_relations(
                &serde_json::json!({}),
                RelationAction::Add,
                &RelationUri::from(URI_B),
            );

            assert_eq!(result, serde_json::json!({ "dc:relation": [URI_B] }));
        }

        #[test]
        fn removes_existing_uri() {
            let result = super::apply_relations(
                &serde_json::json!({ "dc:relation": [URI_A, URI_B] }),
                RelationAction::Remove,
                &RelationUri::from(URI_B),
            );

            assert_eq!(result, serde_json::json!({ "dc:relation": [URI_A] }));
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

    mod fixtures {
        pub(super) use crate::client::test_http::{MockServer, http_response};

        pub(super) fn item_json(
            key: &str,
            relations: &serde_json::Value,
        ) -> String {
            serde_json::json!({
                "key": key,
                "version": 1,
                "data": {
                    "key": key,
                    "version": 1,
                    "itemType": "journalArticle",
                    "relations": relations.clone(),
                },
            })
            .to_string()
        }
    }

    mod get_related_items {
        use pretty_assertions::assert_eq;

        use super::{
            fixtures::{MockServer, http_response},
            *,
        };
        use crate::{keys::ItemKey, types::ItemType};

        #[tokio::test]
        async fn resolves_related_items_and_skips_unresolvable_keys() {
            let source = serde_json::json!({
                "key": "ITEM0001",
                "version": 1,
                "data": {
                    "key": "ITEM0001",
                    "version": 1,
                    "itemType": "journalArticle",
                    "relations": {
                        "dc:relation": [
                            "http://zotero.org/users/0/items/ITEM0002",
                            "http://zotero.org/groups/1/items/ITEM0003",
                            "https://example.com/not-a-zotero-uri",
                        ],
                    },
                },
            });
            let related_book = serde_json::json!({
                "key": "ITEM0002",
                "version": 1,
                "data": {
                    "key": "ITEM0002",
                    "version": 1,
                    "itemType": "book",
                    "title": "Related Book",
                },
            });
            let server = MockServer::new(vec![
                http_response("200 OK", &source.to_string()),
                http_response("200 OK", &related_book.to_string()),
                http_response("404 Not Found", ""),
            ]);
            let client = ZoteroClient::new(server.url());

            let related = client.get_related_items("ITEM0001").await.unwrap();

            assert_eq!(related, vec![RelatedItem {
                key: ItemKey::from("ITEM0002"),
                title: Some("Related Book".to_owned()),
                item_type: ItemType::Book,
            }]);
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "test assertions index recorded requests by fixed position"
    )]
    mod add_item_relation {
        use super::{
            fixtures::{MockServer, http_response, item_json},
            *,
        };

        #[tokio::test]
        async fn patches_both_items_with_each_others_uri() {
            let (server, recorded) = MockServer::recording(vec![
                http_response(
                    "200 OK",
                    &item_json("ITEM0001", &serde_json::json!({})),
                ),
                http_response(
                    "200 OK",
                    &item_json("ITEM0002", &serde_json::json!({})),
                ),
                http_response(
                    "200 OK",
                    &item_json(
                        "ITEM0001",
                        &serde_json::json!({
                            "dc:relation": [
                                "http://zotero.org/users/0/items/ITEM0002",
                            ],
                        }),
                    ),
                ),
                http_response(
                    "200 OK",
                    &item_json(
                        "ITEM0002",
                        &serde_json::json!({
                            "dc:relation": [
                                "http://zotero.org/users/0/items/ITEM0001",
                            ],
                        }),
                    ),
                ),
            ]);
            let client = ZoteroClient::new(server.url());

            let result = client.add_item_relation("ITEM0001", "ITEM0002").await;

            assert!(result.is_ok());
            let requests = recorded.lock().expect("request log lock");
            assert_eq!(requests.len(), 4);
            assert!(requests[0].starts_with("GET /users/0/items/ITEM0001"));
            assert!(requests[1].starts_with("GET /users/0/items/ITEM0002"));
            assert!(requests[2].starts_with("PATCH /users/0/items/ITEM0001"));
            assert!(requests[3].starts_with("PATCH /users/0/items/ITEM0002"));
        }

        #[tokio::test]
        async fn rejects_self_relation() {
            let client = ZoteroClient::new("http://127.0.0.1:23119/api");

            let err = client
                .add_item_relation("ITEM0001", "ITEM0001")
                .await
                .unwrap_err();

            assert!(matches!(err, ZoteroApiError::InputRejected(_)));
        }
    }
}
