//! Deleted library object synchronization.
//!
//! Provides response structures and client methods for querying deleted items,
//! collections, searches, and tags since a given library version.

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
    keys::{CollectionKey, ItemKey, SearchKey, TagName},
};

/// Deleted object keys returned by Zotero's incremental sync protocol.
///
/// `GET /deleted?since=<version>` returns objects deleted after the supplied
/// library version. Use the highest `Last-Modified-Version` seen from an
/// earlier API response as the next `since` value, then remove the returned
/// keys from local caches before applying newer object data.
///
/// The `since` parameter is exclusive: objects deleted at or before that
/// version are not included.
///
/// # Examples
///
/// ```rust
/// use zotero_api::DeletedObjectsResponse;
///
/// let deleted: DeletedObjectsResponse = serde_json::from_value(serde_json::json!({
///     "collections": ["C1"],
///     "items": ["I1", "I2"],
///     "searches": [],
///     "tags": ["obsolete"]
/// }))?;
///
/// assert_eq!(deleted.collections, ["C1"]);
/// assert_eq!(deleted.items, ["I1", "I2"]);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct DeletedObjectsResponse {
    /// Deleted collection keys.
    #[serde(default)]
    pub collections: Vec<CollectionKey>,
    /// Deleted saved search keys.
    #[serde(default)]
    pub searches: Vec<SearchKey>,
    /// Deleted item keys.
    #[serde(default)]
    pub items: Vec<ItemKey>,
    /// Deleted tag names.
    #[serde(default)]
    pub tags: Vec<TagName>,
}

impl ZoteroClient {
    /// Retrieves deleted library object keys after `since`.
    ///
    /// Zotero filters deletions by library version. Pass the last known
    /// `Last-Modified-Version` value from a previous read or write response to
    /// receive only later deletions. Store the newest `Last-Modified-Version`
    /// from each sync response and use it for the next incremental sync call.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn get_deleted<K: Into<u64>>(
        &self,
        since: K,
    ) -> Result<DeletedObjectsResponse, ZoteroApiError> {
        let res: ZoteroResponse<DeletedObjectsResponse> = self
            .get("/deleted")
            .query("since", since.into().to_string())
            .send()
            .await?;
        Ok(res.data)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        client::test_http::{MockServer, http_response},
        version::LibraryVersion,
    };

    #[tokio::test]
    async fn parses_deleted_objects_response() {
        let json_resp = serde_json::json!({
            "collections": ["C1"],
            "searches": [],
            "items": ["I1", "I2"],
            "tags": ["tag1"]
        })
        .to_string();

        let server = MockServer::new(vec![http_response("200 OK", &json_resp)]);
        let client = ZoteroClient::new(server.url());

        let deleted =
            client.get_deleted(LibraryVersion::new(10)).await.unwrap();
        assert_eq!(deleted.items, vec!["I1", "I2"]);
        assert_eq!(deleted.collections, vec!["C1"]);
        assert_eq!(deleted.tags, vec!["tag1"]);
    }
}
