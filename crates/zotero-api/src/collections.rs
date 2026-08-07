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
    /// Add items to the target collection.
    Add,
    /// Remove items from the target collection.
    Remove,
}

impl ZoteroClient {
    /// Fetches all collections defined in the library scope, returning the full
    /// collection tree.
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

    /// Searches collections by matching `query` against collection names
    /// case-insensitively.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError`] if fetching collections fails.
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

    /// Fetches every item contained within the collection identified by
    /// `collection_key`.
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

    /// Creates a new collection with the given `name` and optional
    /// `parent_key`.
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
        res.data.into_iter().next().ok_or_else(|| ZoteroApiError::LocalApi {
            status: 500,
            message: "Created collection array was empty".to_owned(),
        })
    }

    /// Adds items to or removes items from a collection without modifying item
    /// metadata.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`] if Zotero rejects the request, or
    /// [`ZoteroApiError::Network`] if the request fails.
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
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`] if
    /// fetching or deleting the collection fails.
    pub async fn delete_collection<K: AsRef<str>>(
        &self,
        collection_key: K,
    ) -> Result<(), ZoteroApiError> {
        let key = collection_key.as_ref();
        let path = format!("/collections/{key}");
        let res: ZoteroResponse<ZoteroCollection> =
            self.get(&path).send().await?;
        self.delete_req(&path)
            .unmodified_since_version(res.version.as_u64())
            .send_unit()
            .await?;
        Ok(())
    }

    /// Renames a collection and/or moves it to a new parent collection
    /// location.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if fetching or updating the collection fails.
    pub async fn update_collection<K: AsRef<str>>(
        &self,
        collection_key: K,
        name: Option<&str>,
        parent: Option<&CollectionParent>,
    ) -> Result<ZoteroCollection, ZoteroApiError> {
        let key = collection_key.as_ref();
        let path = format!("/collections/{key}");
        let res: ZoteroResponse<ZoteroCollection> =
            self.get(&path).send().await?;
        let current = res.data;

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
        let resp = self.put(&path).json(payload).send_raw().await?;
        if !resp.status().is_success() {
            return Err(ZoteroApiError::LocalApi {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        if let Ok(col) = resp.json::<ZoteroCollection>().await {
            Ok(col)
        } else {
            let refetch: ZoteroResponse<ZoteroCollection> =
                self.get(&path).send().await?;
            Ok(refetch.data)
        }
    }

    #[inline]
    /// Batch-deletes multiple collections by key in a single request.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`] if
    /// Zotero rejects the deletion request.
    pub async fn delete_collections<K: AsRef<str>, V: Into<u64>>(
        &self,
        keys: &[K],
        version: V,
    ) -> Result<(), ZoteroApiError> {
        let keys_str = keys
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(",");
        self.delete_req("/collections")
            .query("collectionKey", keys_str)
            .unmodified_since_version(version.into())
            .send_unit()
            .await?;
        Ok(())
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
}
