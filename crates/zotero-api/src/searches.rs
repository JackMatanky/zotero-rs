//! Saved search management and execution.

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
    keys::LibraryVersion,
    objects::{BatchWriteResponse, ZoteroItem},
};

/// A saved search that stores query conditions server-side.
///
/// Saved searches persist filter criteria on the Zotero server, allowing
/// repeated execution without resending the query.
///
/// # Examples
///
/// ```ignore
/// use zotero_api::searches::SavedSearch;
///
/// let search = SavedSearch {
///     key: "ABC12345".into(),
///     version: 1.into(),
///     name: "Quantum Papers".into(),
///     conditions: vec![],
/// };
/// assert_eq!(search.name, "Quantum Papers");
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SavedSearch {
    /// 8-character search key identifier.
    pub key: String,
    /// Library version counter.
    pub version: LibraryVersion,
    /// Human-readable search name.
    pub name: String,
    /// Query condition definitions as an array of JSON objects.
    #[serde(default)]
    pub conditions: Vec<serde_json::Value>,
}

impl ZoteroClient {
    /// Lists all [`SavedSearch`] objects in the target library.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`], [`ZoteroApiError::Network`], or
    /// [`ZoteroApiError::Json`] if the request fails.
    #[inline]
    pub async fn list_searches(
        &self,
    ) -> Result<Vec<SavedSearch>, ZoteroApiError> {
        let res: ZoteroResponse<Vec<SavedSearch>> =
            self.get("/searches").send().await?;
        Ok(res.data)
    }

    /// Fetches a single [`SavedSearch`] by its key.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`], [`ZoteroApiError::Network`], or
    /// [`ZoteroApiError::Json`] if the request fails.
    #[inline]
    pub async fn get_search<K: AsRef<str>>(
        &self,
        key: K,
    ) -> Result<SavedSearch, ZoteroApiError> {
        let key_str = key.as_ref();
        let res: ZoteroResponse<SavedSearch> =
            self.get(format!("/searches/{key_str}")).send().await?;
        Ok(res.data)
    }

    /// Executes a [`SavedSearch`] server-side and returns matching
    /// [`ZoteroItem`]s.
    ///
    /// The server evaluates the stored query conditions against the library and
    /// returns all items that match.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`], [`ZoteroApiError::Network`],
    /// or [`ZoteroApiError::Json`] if the request fails.
    #[inline]
    pub async fn execute_saved_search<K: AsRef<str>>(
        &self,
        key: K,
    ) -> Result<Vec<ZoteroItem>, ZoteroApiError> {
        let key_str = key.as_ref();
        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.get(format!("/searches/{key_str}/items")).send().await?;
        Ok(res.data)
    }

    /// Batch-creates new saved searches in the library.
    ///
    /// Each element of `searches` must be a JSON object with `"name"` and
    /// `"conditions"` fields matching the Zotero search format.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`] if Zotero rejects the creation
    /// request, or [`ZoteroApiError::Network`] if the request fails.
    #[inline]
    pub async fn create_searches(
        &self,
        searches: &[serde_json::Value],
    ) -> Result<BatchWriteResponse, ZoteroApiError> {
        let res: ZoteroResponse<BatchWriteResponse> = self
            .post("/searches")
            .json(serde_json::json!(searches))
            .send()
            .await?;
        Ok(res.data)
    }

    /// Batch-deletes saved searches by key.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`] or [`ZoteroApiError::Network`] if
    /// Zotero rejects the deletion request.
    #[inline]
    pub async fn delete_searches<K: AsRef<str>, V: Into<u64>>(
        &self,
        keys: &[K],
        version: V,
    ) -> Result<(), ZoteroApiError> {
        self.delete_by_keys("/searches", "searchKey", keys, version).await
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::client::test_http::{MockServer, http_response};

    #[tokio::test]
    async fn parses_list_searches_response() {
        let json_resp = serde_json::json!([
            {
                "key": "SEARCH01",
                "version": 1,
                "name": "Recent Quantum Papers",
                "conditions": [{"field": "title", "operator": "contains", "value": "quantum"}]
            }
        ])
        .to_string();

        let server = MockServer::new(vec![http_response("200 OK", &json_resp)]);
        let client = ZoteroClient::new(server.url());

        let searches = client.list_searches().await.unwrap();
        assert_eq!(searches.len(), 1);
        assert_eq!(searches.first().unwrap().name, "Recent Quantum Papers");
    }
}
