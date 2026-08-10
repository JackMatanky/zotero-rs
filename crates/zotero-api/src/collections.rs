//! Collection operations for the Zotero Local HTTP API.

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
    keys::CollectionKey,
    objects::{ZoteroCollection, ZoteroItem},
    types::CollectionParent,
};

/// Action for adding or removing items to or from a collection.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum CollectionItemAction {
    /// Add items to the target collection. Items already in the collection
    /// are ignored.
    Add,
    /// Remove items from the target collection. Items not in the collection
    /// are ignored.
    Remove,
}

impl ZoteroClient {
    /// Returns all collections as a flat list.
    ///
    /// Collections form a tree via [`CollectionParent`], but this method does
    /// not nest them. Every collection in the library is returned regardless
    /// of depth.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    pub async fn get_collections(
        &self,
    ) -> Result<Vec<ZoteroCollection>, ZoteroApiError> {
        let res: ZoteroResponse<Vec<ZoteroCollection>> =
            self.get("/collections").send().await?;
        Ok(res.data)
    }

    /// Returns a single collection by its key.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::NotFound`] if no collection with
    /// `collection_key` exists, or
    /// [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    pub async fn get_collection<K: AsRef<str>>(
        &self,
        collection_key: K,
    ) -> Result<ZoteroCollection, ZoteroApiError> {
        let key = collection_key.as_ref();
        let res: ZoteroResponse<ZoteroCollection> = self
            .get(format!("/collections/{key}"))
            .send_or_not_found(format!("Collection {key}"))
            .await?;
        Ok(res.data)
    }

    /// Searches collections by case-insensitive substring match on the name.
    ///
    /// Fetches all collections and filters locally. Returns only collections
    /// whose name contains `query`.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if fetching collections fails.
    pub async fn search_collections(
        &self,
        query: &str,
    ) -> Result<Vec<ZoteroCollection>, ZoteroApiError> {
        let collections = self.get_collections().await?;
        let query_lc = query.to_lowercase();
        let filtered = collections
            .into_iter()
            .filter(|c| c.data.name.to_lowercase().contains(&query_lc))
            .collect();
        Ok(filtered)
    }

    /// Returns all items in the collection identified by `collection_key`.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    pub async fn get_collection_items<K: AsRef<str>>(
        &self,
        collection_key: K,
    ) -> Result<Vec<ZoteroItem>, ZoteroApiError> {
        let key = collection_key.as_ref();
        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.get(format!("/collections/{key}/items")).send().await?;
        Ok(res.data)
    }

    /// Creates a new collection with the given `name`.
    ///
    /// Pass `None` for `parent_key` to create a top-level collection, or
    /// `Some(key)` to create a child of the specified collection.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`] if Zotero rejects the creation
    /// request, or [`ZoteroApiError::Network`] if the request fails.
    pub async fn create_collection(
        &self,
        name: &str,
        parent_key: Option<&CollectionKey>,
    ) -> Result<ZoteroCollection, ZoteroApiError> {
        let parent_val = parent_key.map_or(CollectionParent::TopLevel, |key| {
            CollectionParent::Parent(key.clone())
        });
        let payload = serde_json::json!([{
            "name": name,
            "parentCollection": parent_val,
        }]);

        let res: ZoteroResponse<Vec<ZoteroCollection>> =
            self.post("/collections").json(payload).send().await?;
        crate::client::first_created(res.data, "collection")
    }

    /// Adds or removes items from a collection.
    ///
    /// With [`CollectionItemAction::Add`], items are added to the collection;
    /// items already in the collection are ignored. With
    /// [`CollectionItemAction::Remove`], items are removed; items not in the
    /// collection are ignored. Item metadata is not modified.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`] if the
    /// request fails.
    pub async fn manage_collection_items<K: AsRef<str>, V: AsRef<str>>(
        &self,
        collection_key: K,
        item_keys: &[V],
        action: CollectionItemAction,
    ) -> Result<(), ZoteroApiError> {
        let key = collection_key.as_ref();
        let path = format!("/collections/{key}/items");
        let body_str = item_keys
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(" ");

        let req = match action {
            CollectionItemAction::Remove => self.delete_req(path),
            CollectionItemAction::Add => self.post(path),
        };
        req.json(serde_json::Value::String(body_str)).send_unit().await?;
        Ok(())
    }

    /// Permanently deletes the collection identified by `collection_key`.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::NotFound`] if no collection with
    /// `collection_key` exists, or
    /// [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`] if the
    /// request fails.
    pub async fn delete_collection<K: AsRef<str>>(
        &self,
        collection_key: K,
    ) -> Result<(), ZoteroApiError> {
        let key = collection_key.as_ref();
        let current = self.get_collection(key).await?;
        self.delete_req(format!("/collections/{key}"))
            .unmodified_since_version(current.version.as_u64())
            .send_unit()
            .await?;
        Ok(())
    }

    /// Updates a collection's name and/or parent.
    ///
    /// Pass `None` for `name` to keep the current name, or `None` for
    /// `parent` to keep the current parent. Pass
    /// [`CollectionParent::TopLevel`] to move to the root.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::NotFound`] if no collection with
    /// `collection_key` exists, or
    /// [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    pub async fn update_collection<K: AsRef<str>>(
        &self,
        collection_key: K,
        name: Option<&str>,
        parent: Option<&CollectionParent>,
    ) -> Result<ZoteroCollection, ZoteroApiError> {
        let key = collection_key.as_ref();
        let current = self.get_collection(key).await?;

        let new_name = name.unwrap_or(&current.data.name);
        let new_parent = parent
            .cloned()
            .unwrap_or_else(|| current.data.parent_collection.clone());
        let payload = serde_json::json!({
            "key": key,
            "version": current.version,
            "name": new_name,
            "parentCollection": new_parent,
        });
        let resp = crate::client::ensure_success(
            self.put(format!("/collections/{key}"))
                .json(payload)
                .send_raw()
                .await?,
        )
        .await?;
        crate::client::decode_or_refetch(resp, || self.get_collection(key))
            .await
    }

    /// Batch-deletes multiple collections by key.
    ///
    /// Pass `version` as the current library version for optimistic
    /// concurrency.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`] if
    /// Zotero rejects the deletion request.
    pub async fn delete_collections<K: AsRef<str>, V: Into<u64>>(
        &self,
        keys: &[K],
        version: V,
    ) -> Result<(), ZoteroApiError> {
        self.delete_by_keys("/collections", "collectionKey", keys, version)
            .await
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;
    use crate::client::test_http::{MockServer, http_response, request_body};

    fn collection_json(
        key: &str,
        name: &str,
        version: u64,
        parent: &serde_json::Value,
    ) -> String {
        json!({"key":key,"version":version,"data":{"key":key,"name":name,"parentCollection":parent.clone()}}).to_string()
    }

    #[tokio::test]
    async fn search_collections_matches_names_case_insensitively() {
        let body = format!(
            "[{},{}]",
            collection_json("COLL0001", "Machine Learning", 1, &json!(false)),
            collection_json("COLL0002", "Other", 1, &json!(false))
        );
        let server = MockServer::new(vec![http_response("200 OK", &body)]);
        let client = ZoteroClient::new(server.url());

        let result = client.search_collections("machine").await;

        assert!(result.is_ok(), "collections should decode: {result:?}");
        let collections = result.unwrap_or_default();
        assert_eq!(
            collections
                .iter()
                .map(|collection| collection.key.as_str())
                .collect::<Vec<_>>(),
            vec!["COLL0001"]
        );
    }

    #[tokio::test]
    async fn create_collection_serializes_top_level_and_parent_collection() {
        let (server, recorded) = MockServer::recording(vec![
            http_response(
                "200 OK",
                &format!(
                    "[{}]",
                    collection_json("TOP00001", "Top", 1, &json!(false))
                ),
            ),
            http_response(
                "200 OK",
                &format!(
                    "[{}]",
                    collection_json("CHILD001", "Child", 1, &json!("PARENT01"))
                ),
            ),
        ]);
        let client = ZoteroClient::new(server.url());

        let top = client.create_collection("Top", None).await;
        let child = client
            .create_collection("Child", Some(&CollectionKey::from("PARENT01")))
            .await;

        assert!(top.is_ok(), "top-level collection should be created: {top:?}");
        assert!(child.is_ok(), "child collection should be created: {child:?}");
        let requests = recorded.lock().expect("request log lock");
        let top_body = requests
            .first()
            .and_then(|request| request_body(request).ok())
            .unwrap_or_default();
        let child_body = requests
            .get(1)
            .and_then(|request| request_body(request).ok())
            .unwrap_or_default();
        assert_eq!(
            top_body.get(0).and_then(|item| item.get("parentCollection")),
            Some(&json!(false))
        );
        assert_eq!(
            child_body.get(0).and_then(|item| item.get("parentCollection")),
            Some(&json!("PARENT01"))
        );
    }

    mod get_collection {
        use pretty_assertions::assert_eq;

        use super::*;

        #[tokio::test]
        async fn returns_not_found_on_404() {
            let server =
                MockServer::new(vec![http_response("404 Not Found", "")]);
            let client = ZoteroClient::new(server.url());

            let result = client.get_collection("COLL0001").await;

            assert!(
                matches!(result, Err(ZoteroApiError::NotFound(_))),
                "404 should map to NotFound: {result:?}"
            );
        }

        #[tokio::test]
        async fn decodes_successful_response() {
            let body = collection_json(
                "COLL0001",
                "Machine Learning",
                1,
                &json!(false),
            );
            let server = MockServer::new(vec![http_response("200 OK", &body)]);
            let client = ZoteroClient::new(server.url());

            let result = client.get_collection("COLL0001").await;

            assert!(result.is_ok(), "collection should decode: {result:?}");
            assert_eq!(
                result.expect("asserted Ok above").data.name,
                "Machine Learning"
            );
        }
    }

    mod delete_collection {
        use super::*;

        #[tokio::test]
        async fn returns_not_found_for_nonexistent_key() {
            let server =
                MockServer::new(vec![http_response("404 Not Found", "")]);
            let client = ZoteroClient::new(server.url());

            let result = client.delete_collection("MISSING1").await;

            assert!(
                matches!(result, Err(ZoteroApiError::NotFound(_))),
                "deleting a nonexistent collection should be NotFound, not a \
                 generic LocalApi error: {result:?}"
            );
        }
    }

    mod update_collection {
        use super::*;

        #[tokio::test]
        async fn returns_not_found_for_nonexistent_key() {
            let server =
                MockServer::new(vec![http_response("404 Not Found", "")]);
            let client = ZoteroClient::new(server.url());

            let result = client
                .update_collection("MISSING1", Some("New Name"), None)
                .await;

            assert!(
                matches!(result, Err(ZoteroApiError::NotFound(_))),
                "updating a nonexistent collection should be NotFound, not a \
                 generic LocalApi error: {result:?}"
            );
        }
    }
}
