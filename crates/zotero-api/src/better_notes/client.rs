//! Async client for the Better Notes plugin HTTP companion API.

use serde::Serialize;
use serde_json::Value;

use crate::{
    better_notes::models::{
        NoteExportFormat, NoteExportResponse, NoteItemResponse, NoteRelations,
        NoteTreeResponse, RelationsResponse, TemplateName, TemplateResponse,
    },
    errors::ZoteroApiError,
    keys::ItemKey,
};

/// Async HTTP client for the Better Notes companion API.
#[derive(Clone, Debug)]
pub struct BetterNotesClient {
    http: reqwest::Client,
    base_url: String,
}

impl Default for BetterNotesClient {
    #[inline]
    fn default() -> Self {
        Self::new("http://127.0.0.1:23119/better-notes")
    }
}

impl BetterNotesClient {
    /// Creates a new [`BetterNotesClient`] with the specified base URL.
    #[inline]
    pub fn new<S: Into<String>>(base_url: S) -> Self {
        let base_url = base_url.into();
        Self {
            http: reqwest::Client::new(),
            base_url: if base_url.is_empty() {
                "http://127.0.0.1:23119/better-notes".to_owned()
            } else {
                base_url
            },
        }
    }

    /// Configures a custom [`reqwest::Client`] HTTP client pool.
    #[must_use]
    #[inline]
    pub fn with_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    /// Exports an existing Zotero note as Markdown or HTML.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::BetterNotes`] if the bridge responds with a
    /// non-2xx status, or [`ZoteroApiError`] if the request fails.
    #[inline]
    pub async fn export<K: AsRef<str>>(
        &self,
        item_key: K,
        format: Option<NoteExportFormat>,
    ) -> Result<String, ZoteroApiError> {
        let format = format.unwrap_or_default();
        let payload = serde_json::json!({
            "itemKey": item_key.as_ref(),
            "format": format.as_str(),
        });
        let res: NoteExportResponse =
            self.post_json("/notes/export", payload).await?;
        Ok(res.content)
    }

    /// Converts Markdown content into a Zotero HTML note attached to
    /// `parent_key`.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::BetterNotes`] if the bridge responds with a
    /// non-2xx status, or [`ZoteroApiError`] if the request fails.
    #[inline]
    pub async fn convert_from_markdown<K: AsRef<str>>(
        &self,
        parent_key: Option<K>,
        markdown: &str,
    ) -> Result<ItemKey, ZoteroApiError> {
        let payload = serde_json::json!({
            "parentKey": parent_key.map(|k| k.as_ref().to_owned()),
            "markdown": markdown,
        });
        let res: NoteItemResponse =
            self.post_json("/notes/from-markdown", payload).await?;
        Ok(res.item_key)
    }

    /// Executes a named Better Notes template against `item_key`.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::BetterNotes`] if the bridge responds with a
    /// non-2xx status, or [`ZoteroApiError`] if the request fails.
    #[inline]
    pub async fn run_template<K: AsRef<str>>(
        &self,
        name: &TemplateName,
        item_key: K,
    ) -> Result<String, ZoteroApiError> {
        let payload = serde_json::json!({
            "name": name,
            "itemKey": item_key.as_ref(),
        });
        let res: TemplateResponse =
            self.post_json("/templates/run", payload).await?;
        Ok(res.result)
    }

    /// Fetches relations for `item_key`.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::BetterNotes`] if the bridge responds with a
    /// non-2xx status, or [`ZoteroApiError`] if the request fails.
    #[inline]
    pub async fn get_relations<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<NoteRelations, ZoteroApiError> {
        let payload = serde_json::json!({
            "itemKey": item_key.as_ref(),
        });
        let res: RelationsResponse =
            self.post_json("/relations/get", payload).await?;
        Ok(res.relations)
    }

    /// Fetches the full Better Notes hierarchy tree rooted at `item_key`.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::BetterNotes`] if the bridge responds with a
    /// non-2xx status, or [`ZoteroApiError`] if the request fails.
    #[inline]
    pub async fn get_tree<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<Value, ZoteroApiError> {
        let payload = serde_json::json!({
            "itemKey": item_key.as_ref(),
        });
        let res: NoteTreeResponse =
            self.post_json("/notes/tree", payload).await?;
        Ok(res.tree)
    }

    async fn post_json<P: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        payload: P,
    ) -> Result<R, ZoteroApiError> {
        let url = format!("{}{endpoint}", self.base_url.trim_end_matches('/'));
        let resp = self.http.post(&url).json(&payload).send().await?;

        if !resp.status().is_success() {
            return Err(ZoteroApiError::BetterNotes(format!(
                "HTTP {} calling {}",
                resp.status(),
                endpoint
            )));
        }

        let data: R = resp.json().await?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    mod fixtures {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };

        pub(super) fn http_response(status: &str, body: &str) -> String {
            format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: \
                 application/json\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
        }

        pub(super) fn mock_server(responses: Vec<String>) -> String {
            let listener =
                TcpListener::bind("127.0.0.1:0").expect("bind listener");
            let addr = listener.local_addr().expect("local addr");
            std::thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) =
                        listener.accept().expect("accept connection");
                    let mut buf = [0_u8; 1024];
                    let _ = stream.read(&mut buf);
                    let _ = stream.write_all(response.as_bytes());
                }
            });
            format!("http://{addr}")
        }
    }

    mod post_json {
        use super::{
            super::*,
            fixtures::{http_response, mock_server},
        };

        #[tokio::test]
        async fn returns_better_notes_error_when_response_is_non_success() {
            let base = mock_server(vec![http_response("400 Bad Request", "")]);
            let client = BetterNotesClient::new(base);

            let err = client.export("NOTE1", None).await.unwrap_err();

            assert!(matches!(
                &err,
                ZoteroApiError::BetterNotes(msg) if msg.contains("400") && msg.contains("/notes/export")
            ));
        }
    }
}
