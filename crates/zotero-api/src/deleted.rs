//! Deleted library object synchronization.
//!
//! Provides response structures and client methods for querying deleted items,
//! collections, searches, and tags since a given library version.

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
};

/// Response object from `GET <prefix>/deleted?since=<version>`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct DeletedObjectsResponse {
    /// Deleted collection keys.
    #[serde(default)]
    pub collections: Vec<String>,
    /// Deleted saved search keys.
    #[serde(default)]
    pub searches: Vec<String>,
    /// Deleted item keys.
    #[serde(default)]
    pub items: Vec<String>,
    /// Deleted tag strings.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ZoteroClient {
    /// Retrieves deleted library objects (items, collections, searches, tags)
    /// since `since`.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
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
        keys::LibraryVersion,
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
        assert_eq!(deleted.items, vec!["I1".to_owned(), "I2".to_owned()]);
        assert_eq!(deleted.collections, vec!["C1".to_owned()]);
        assert_eq!(deleted.tags, vec!["tag1".to_owned()]);
    }
}
