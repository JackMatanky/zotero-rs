//! Core item lifecycle operations for the Zotero Local HTTP API.

use std::{fmt::Write, path::Path};

use md5::Digest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
    objects::{BatchWriteResponse, ItemDraft, ZoteroItem},
    types::{ItemType, LinkMode},
};

/// Requested trash state transition for a library item.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum TrashAction {
    /// Move the item into the library trash.
    MoveToTrash,
    /// Restore the item from the trash back to the active library.
    Restore,
}

impl TrashAction {
    /// Returns `true` if this action represents a deletion to trash.
    pub(crate) fn is_deleted(self) -> bool {
        matches!(self, Self::MoveToTrash)
    }
}

/// Phase-1 response payload from Zotero's file-upload endpoint.
#[derive(Deserialize)]
struct UploadTicket {
    /// Signed upload URL to `POST` the raw file bytes to.
    url: String,
    /// Upload key replayed in the finalize request.
    #[serde(rename = "uploadKey")]
    upload_key: String,
}

impl ZoteroClient {
    /// Fetches recent items sorted by `dateModified` descending.
    ///
    /// Notes and annotations are excluded. The `limit` parameter caps the
    /// number of items returned (capped server-side by Zotero).
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
    pub async fn get_recent_items(
        &self,
        limit: usize,
    ) -> Result<Vec<ZoteroItem>, ZoteroApiError> {
        let res: ZoteroResponse<Vec<ZoteroItem>> = self
            .get("/items")
            .query("limit", limit.to_string())
            .query("sort", "dateModified")
            .query("direction", "desc")
            .query("itemType", "-note")
            .send()
            .await?;
        Ok(res.data)
    }

    /// Fetches all top-level items across the library with automatic
    /// pagination.
    ///
    /// Pages through the Zotero API in batches of 100, concatenating results.
    /// Notes and annotations are excluded. Returns the full set of matching
    /// items as a single `Vec`.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if any page request returns a non-2xx status
    /// - [`Network`] on connection failure
    /// - [`Json`] if any page cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn get_all_items(
        &self,
    ) -> Result<Vec<ZoteroItem>, ZoteroApiError> {
        let url = format!(
            "{}{}/items?itemType=-note&sort=dateModified&direction=desc",
            self.base_url.trim_end_matches('/'),
            self.target_prefix()
        );
        self.get_all_json(&url, 100).await
    }

    /// Fetches a single [`ZoteroItem`] by its key.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if no item with `item_key` exists
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    ///
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn get_item<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let key = item_key.as_ref();
        let res: ZoteroResponse<ZoteroItem> = self
            .get(format!("/items/{key}"))
            .send_or_not_found(format!("Item {key}"))
            .await?;
        Ok(res.data)
    }

    /// Fetches top-level items not filed in any collection.
    ///
    /// Returns items from [`/items/top`](ZoteroClient) filtered client-side to
    /// those with empty `collections` arrays. The `limit` parameter caps the
    /// initial page size.
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
    pub async fn get_unfiled_items(
        &self,
        limit: usize,
    ) -> Result<Vec<ZoteroItem>, ZoteroApiError> {
        let res: ZoteroResponse<Vec<ZoteroItem>> = self
            .get("/items/top")
            .query("limit", limit.to_string())
            .send()
            .await?;
        Ok(res
            .data
            .into_iter()
            .filter(|i| i.data.collections.is_empty())
            .collect())
    }

    /// Fetches child items of the given [`ZoteroItem`].
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
    pub async fn get_item_children<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<Vec<ZoteroItem>, ZoteroApiError> {
        let key = item_key.as_ref();
        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.get(format!("/items/{key}/children")).send().await?;
        Ok(res.data)
    }

    /// Patches metadata fields of an existing [`ZoteroItem`].
    ///
    /// Sends a `PATCH` with the provided JSON fields. If Zotero returns an
    /// empty body, the item is re-fetched automatically.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the patch payload or the item version
    ///   is stale
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn update_item<K: AsRef<str>>(
        &self,
        item_key: K,
        fields: serde_json::Value,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let key = item_key.as_ref();
        let resp = crate::client::ensure_success(
            self.patch(format!("/items/{key}")).json(fields).send_raw().await?,
        )
        .await?;
        crate::client::decode_or_refetch(resp, || self.get_item(key)).await
    }

    /// Permanently deletes an item from the library.
    ///
    /// Fetches the current version first to send an `If-Unmodified-Since`
    /// header, preventing concurrent modification.
    /// # Errors
    ///
    /// - [`NotFound`] if no item exists with `item_key`
    /// - [`LocalApi`] if Zotero rejects the deletion (e.g. version conflict)
    /// - [`Network`] on connection failure
    ///
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn delete_item<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<(), ZoteroApiError> {
        let key = item_key.as_ref();
        let item = self.get_item(key).await?;
        self.delete_req(format!("/items/{key}"))
            .unmodified_since_version(item.version.as_u64())
            .send_unit()
            .await?;
        Ok(())
    }

    /// Moves an item to trash or restores it from trash.
    ///
    /// Uses [`TrashAction`] to select the direction. Fetches the current
    /// version to avoid stale writes.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if no item exists with `item_key`
    /// - [`LocalApi`] if Zotero rejects the update (e.g. version conflict)
    /// - [`Network`] on connection failure
    ///
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn set_item_deleted<K: AsRef<str>>(
        &self,
        item_key: K,
        action: TrashAction,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let key = item_key.as_ref();
        let item = self.get_item(key).await?;
        self.update_item(
            key,
            serde_json::json!({
                "deleted": action.is_deleted(),
                "version": item.version
            }),
        )
        .await
    }

    /// Creates a new [`ZoteroItem`] from an [`ItemDraft`].
    ///
    /// Sends the draft as a single-element JSON array to `POST /items` and
    /// returns the created item.
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
    pub async fn create_item_from_metadata(
        &self,
        draft: ItemDraft,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.post("/items").json(json!([draft])).send().await?;
        crate::client::first_created(res.data, "item")
    }

    /// Batch-creates multiple items from a JSON array.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the batch payload
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn create_items(
        &self,
        items: &[serde_json::Value],
    ) -> Result<BatchWriteResponse, ZoteroApiError> {
        let res: ZoteroResponse<BatchWriteResponse> =
            self.post("/items").json(json!(items)).send().await?;
        Ok(res.data)
    }

    /// Batch-updates multiple items.
    ///
    /// Delegates to [`create_items`](ZoteroClient::create_items). Zotero's
    /// `POST /items` endpoint handles both creates and updates based on whether
    /// each item has a `key`.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the batch payload
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn update_items(
        &self,
        items: &[serde_json::Value],
    ) -> Result<BatchWriteResponse, ZoteroApiError> {
        self.create_items(items).await
    }

    /// Batch-deletes multiple items by key.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the deletion request
    /// - [`Network`] on connection failure
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn delete_items<K: AsRef<str>, V: Into<u64>>(
        &self,
        keys: &[K],
        version: V,
    ) -> Result<(), ZoteroApiError> {
        self.delete_by_keys("/items", "itemKey", keys, version).await
    }

    /// Returns the local file view URL for an attachment [`ZoteroItem`].
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn get_item_file_view_url<K: AsRef<str>>(
        &self,
        key: K,
    ) -> Result<String, ZoteroApiError> {
        let key_str = key.as_ref();
        let resp = self
            .get(format!("/items/{key_str}/file/view/url"))
            .send_raw()
            .await?;
        let resp = crate::client::ensure_success(resp).await?;
        Ok(resp.text().await?)
    }

    /// Returns Zotero's indexed full-text content for an item.
    ///
    /// Returns an empty string if the item has no indexed content.
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
    pub async fn get_item_fulltext<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<String, ZoteroApiError> {
        let key = item_key.as_ref();
        let res: ZoteroResponse<serde_json::Value> =
            self.get(format!("/items/{key}/fulltext")).send().await?;
        let content = res
            .data
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        Ok(content)
    }

    /// Attaches a linked file or URL to a parent item without uploading data.
    ///
    /// Creates an attachment item with `linkMode` set to
    /// [`LinkMode::ImportedFile`], storing only the `path` reference. For
    /// uploading actual file bytes, use
    /// [`import_pdf_file`](ZoteroClient::import_pdf_file) instead.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the request
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn attach_file_link<K: AsRef<str>>(
        &self,
        parent_item_key: K,
        title: &str,
        file_path_or_url: &str,
        content_type: Option<&str>,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let payload = serde_json::json!([{
            "itemType": ItemType::Attachment,
            "parentItem": parent_item_key.as_ref(),
            "title": title,
            "linkMode": LinkMode::ImportedFile,
            "path": file_path_or_url,
            "contentType": content_type.unwrap_or("application/pdf"),
        }]);

        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.post("/items").json(payload).send().await?;
        crate::client::first_created(res.data, "attachment")
    }

    /// Imports a local file into Zotero storage using the 3-phase MD5 upload
    /// protocol.
    ///
    /// # Upload protocol
    ///
    /// 1. **Metadata record.** `POST /items` creates an attachment item with
    ///    `linkMode` set to [`LinkMode::ImportedFile`], the filename, content
    ///    type, and optional `parentItem`.
    /// 2. **Upload ticket.** `POST /items/{key}/file` sends the MD5 hash,
    ///    filename, filesize, and mtime. If the server responds with
    ///    `{"exists": true}`, the file already matches and upload is
    ///    **skipped**. Otherwise it returns a signed upload URL and an
    ///    `uploadKey`.
    /// 3. **Raw bytes.** `POST` the file bytes to the signed URL. On 201
    ///    Created, finalize with `POST /items/{key}/file` passing the
    ///    `uploadKey`.
    ///
    /// # Errors
    ///
    /// - [`InputRejected`] if the path has no valid UTF-8 filename
    /// - [`Io`] if reading the local file fails
    /// - [`LocalApi`] if Zotero rejects any phase of the upload (metadata,
    ///   ticket, or finalization)
    /// - [`Network`] on connection failure
    /// - [`Json`] if a response cannot be deserialized
    ///
    /// [`InputRejected`]: ZoteroApiError::InputRejected
    /// [`Io`]: ZoteroApiError::Io
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn import_pdf_file<K: AsRef<str>>(
        &self,
        parent_item_key: Option<K>,
        title: &str,
        path: &Path,
        content_type: Option<&str>,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let bytes = fs::read(path).await?;

        let mut hasher = md5::Md5::new();
        hasher.update(&bytes);
        let mut md5 = String::with_capacity(32);
        for byte in hasher.finalize() {
            let _ = write!(md5, "{byte:02x}");
        }

        let filename =
            path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
                ZoteroApiError::InputRejected(
                    "path has no valid UTF-8 filename".into(),
                )
            })?;

        let metadata = fs::metadata(path).await?;
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));

        let mut attachment = serde_json::Map::new();
        attachment.insert("itemType".into(), json!(ItemType::Attachment));
        attachment.insert("title".into(), json!(title));
        attachment.insert("linkMode".into(), json!(LinkMode::ImportedFile));
        attachment.insert("filename".into(), json!(filename));
        attachment.insert(
            "contentType".into(),
            json!(content_type.unwrap_or("application/pdf")),
        );
        if let Some(parent) = parent_item_key {
            attachment.insert("parentItem".into(), json!(parent.as_ref()));
        }

        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.post("/items").json(json!([attachment])).send().await?;
        let item = crate::client::first_created(res.data, "attachment")?;

        let file_url = format!(
            "{}{}/items/{}/file",
            self.base_url.trim_end_matches('/'),
            self.target_prefix(),
            item.data.key
        );
        let filesize_text = bytes.len().to_string();
        let mtime_text = modified_ms.to_string();
        let req = self
            .http
            .post(&file_url)
            .form(&[
                ("md5", md5.as_str()),
                ("filename", filename),
                ("filesize", filesize_text.as_str()),
                ("mtime", mtime_text.as_str()),
            ])
            .header("If-None-Match", "*");
        let resp = crate::client::ensure_success(req.send().await?).await?;
        let body: serde_json::Value = resp.json().await?;
        if body.as_object().is_some_and(|object| object.contains_key("exists"))
        {
            return Ok(item);
        }
        let ticket: UploadTicket = serde_json::from_value(body)?;

        let upload = self.http.post(&ticket.url).body(bytes).send().await?;
        if upload.status().as_u16() != 201 {
            return Err(ZoteroApiError::LocalApi {
                status: upload.status().as_u16(),
                message: upload.text().await.unwrap_or_default(),
            });
        }

        crate::client::ensure_success(
            self.http
                .post(&file_url)
                .form(&[("upload", ticket.upload_key.as_str())])
                .header("If-None-Match", "*")
                .send()
                .await?,
        )
        .await?;
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::client::test_http::{MockServer, http_response};

    fn item_json(
        key: &str,
        deleted: bool,
        collections: &serde_json::Value,
    ) -> String {
        json!({
            "key": key,
            "version": 7,
            "data": {
                "key": key,
                "version": 7,
                "itemType": "journalArticle",
                "deleted": deleted,
                "collections": collections.clone(),
            },
        })
        .to_string()
    }

    mod get_item {
        use super::*;

        #[tokio::test]
        async fn returns_not_found_on_404() {
            let server =
                MockServer::new(vec![http_response("404 Not Found", "")]);
            let client = ZoteroClient::new(server.url());

            let result = client.get_item("ITEM0001").await;

            assert!(
                matches!(result, Err(ZoteroApiError::NotFound(_))),
                "404 should map to NotFound: {result:?}"
            );
        }
    }

    mod get_unfiled_items {
        use pretty_assertions::assert_eq;

        use super::*;

        #[tokio::test]
        async fn filters_items_with_collections() {
            let body = format!(
                "[{},{}]",
                item_json("ITEM0001", false, &json!([])),
                item_json("ITEM0002", false, &json!(["COLL0001"]))
            );
            let server = MockServer::new(vec![http_response("200 OK", &body)]);
            let client = ZoteroClient::new(server.url());

            let result = client.get_unfiled_items(10).await;

            assert!(
                result.is_ok(),
                "unfiled items response should decode: {result:?}"
            );
            let items = result.unwrap_or_default();
            assert_eq!(
                items.iter().map(|item| item.key.as_str()).collect::<Vec<_>>(),
                vec!["ITEM0001"]
            );
        }
    }

    mod update_item {
        use pretty_assertions::assert_eq;

        use super::*;

        #[tokio::test]
        async fn refetches_item_when_patch_response_is_empty() {
            let (server, recorded) = MockServer::recording(vec![
                http_response("200 OK", ""),
                http_response(
                    "200 OK",
                    &item_json("ITEM0001", false, &json!([])),
                ),
            ]);
            let client = ZoteroClient::new(server.url());

            let result = client
                .update_item("ITEM0001", json!({"title": "Updated"}))
                .await;

            assert!(
                result.is_ok(),
                "empty PATCH response should refetch item: {result:?}"
            );
            assert_eq!(
                result.expect("asserted Ok above").key.as_str(),
                "ITEM0001"
            );
            let requests = recorded.lock().expect("request log lock");
            assert_eq!(requests.len(), 2);
            assert!(
                requests.first().is_some_and(|request| request
                    .starts_with("PATCH /users/0/items/ITEM0001")),
                "first request should PATCH item: {requests:?}"
            );
            assert!(
                requests.get(1).is_some_and(|request| request
                    .starts_with("GET /users/0/items/ITEM0001")),
                "second request should refetch item: {requests:?}"
            );
        }
    }

    mod fulltext {
        use pretty_assertions::assert_eq;

        use super::*;

        #[tokio::test]
        async fn returns_content_field() {
            let server = MockServer::new(vec![http_response(
                "200 OK",
                r#"{"content":"paper text"}"#,
            )]);
            let client = ZoteroClient::new(server.url());

            let result = client.get_item_fulltext("ITEM0001").await;

            assert_eq!(result.unwrap_or_default(), "paper text");
        }
    }
}
