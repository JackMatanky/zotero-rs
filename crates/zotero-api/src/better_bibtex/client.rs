//! Async HTTP client for the Better `BibTeX` Zotero plugin JSON-RPC 2.0 API.

use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    better_bibtex::models::{
        AutoExportAddRequest, AuxFilePath, BibliographyFormat, CitekeyMap,
        CollectionPath, JsonRpcRequest, JsonRpcResponse, RegenerateKeyMap,
        SearchQuery, TranslatorName,
    },
    errors::ZoteroApiError,
    keys::{CitationKey, ItemKey},
};

/// Client for issuing JSON-RPC 2.0 requests to the Better `BibTeX` plugin.
///
/// The client talks to the local Better `BibTeX` HTTP endpoint exposed by
/// Zotero, usually `http://127.0.0.1:23119/better-bibtex/json-rpc`. It can:
///
/// - map Zotero item keys to Better `BibTeX` citation keys
/// - export items with a Better `BibTeX` translator
/// - generate formatted bibliographies
/// - regenerate citation keys
/// - register Better `BibTeX` auto-export jobs
/// - scan `LaTeX` `.aux` files for cited references
/// - search Better `BibTeX` indexes
/// - fetch Pandoc citeproc filter metadata
///
/// # Examples
///
/// ```rust,no_run
/// # async fn run() -> Result<(), zotero_api::ZoteroApiError> {
/// use zotero_api::{
///     CitationKey,
///     better_bibtex::{BetterBibtexClient, TranslatorName},
/// };
///
/// let client = BetterBibtexClient::default();
/// let output = client
///     .export_items(
///         &[CitationKey::from("doe2020")],
///         &TranslatorName::from("Better BibTeX"),
///     )
///     .await?;
///
/// assert!(output.contains("doe2020"));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct BetterBibtexClient {
    http: reqwest::Client,
    base_url: String,
}

impl Default for BetterBibtexClient {
    #[inline]
    fn default() -> Self {
        Self::new("http://127.0.0.1:23119/better-bibtex")
    }
}

impl BetterBibtexClient {
    /// Creates a new [`BetterBibtexClient`] with the specified base URL.
    ///
    /// An empty `base_url` falls back to the local Better `BibTeX` default.
    #[inline]
    pub fn new<S: Into<String>>(base_url: S) -> Self {
        let base_url = base_url.into();
        Self {
            http: reqwest::Client::new(),
            base_url: if base_url.is_empty() {
                "http://127.0.0.1:23119/better-bibtex".to_owned()
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

    /// Maps Zotero `item_keys` to their current Better `BibTeX` citation keys.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn get_citekeys(
        &self,
        item_keys: &[ItemKey],
    ) -> Result<CitekeyMap, ZoteroApiError> {
        let params = vec![item_keys];
        self.call_rpc("item.citationkey", params).await
    }

    /// Exports items identified by `citekeys` formatted with `translator`.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn export_items(
        &self,
        citekeys: &[CitationKey],
        translator: &TranslatorName,
    ) -> Result<String, ZoteroApiError> {
        let params = (citekeys, translator);
        self.call_rpc("item.export", params).await
    }

    /// Generates a formatted bibliography string for `citekeys`.
    ///
    /// When `format` is [`None`], Better `BibTeX` uses its configured default
    /// bibliography output. Pass [`BibliographyFormat`] to choose the content
    /// type, CSL style, locale, or quick-copy behavior for this call.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Json`] if `citekeys` or `format` cannot be serialized
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Json`]: ZoteroApiError::Json
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn bibliography(
        &self,
        citekeys: &[CitationKey],
        format: Option<&BibliographyFormat>,
    ) -> Result<String, ZoteroApiError> {
        let mut params = vec![serde_json::to_value(citekeys)?];
        if let Some(format) = format {
            params.push(serde_json::to_value(format)?);
        }
        self.call_rpc("item.bibliography", params).await
    }

    /// Triggers citation key regeneration for `citekeys`.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn regenerate_keys(
        &self,
        citekeys: &[CitationKey],
    ) -> Result<RegenerateKeyMap, ZoteroApiError> {
        let params = vec![citekeys];
        self.call_rpc("item.pin", params).await
    }

    /// Registers a new automatic export task in the Better `BibTeX` plugin.
    ///
    /// Set [`AutoExportAddRequest::replace`] to [`Some`] to send the Better
    /// `BibTeX` `replace` flag. `Some(true)` replaces an existing matching
    /// auto-export configuration. `Some(false)` asks Better `BibTeX` not to
    /// replace one. [`None`] omits the flag and leaves the plugin default in
    /// control.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Json`] if `request` fields cannot be serialized
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Json`]: ZoteroApiError::Json
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn autoexport_add(
        &self,
        request: &AutoExportAddRequest,
    ) -> Result<Value, ZoteroApiError> {
        let mut params = vec![
            serde_json::to_value(&request.collection)?,
            serde_json::to_value(&request.translator)?,
            serde_json::to_value(&request.path)?,
        ];
        match (&request.display_options, request.replace) {
            (Some(display_options), _) => {
                params.push(serde_json::to_value(display_options)?);
            }
            (None, Some(_)) => params.push(json!({})),
            (None, None) => {}
        }
        if let Some(replace) = request.replace {
            params.push(json!(replace));
        }
        self.call_rpc("autoexport.add", params).await
    }

    /// Scans a `LaTeX` `.aux` file and imports cited references into
    /// `collection`.
    ///
    /// Better `BibTeX` reads `\citation{...}` entries from the `.aux` file at
    /// `aux_path`, resolves the citation keys, and adds matching Zotero items
    /// to `collection`.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn scan_aux(
        &self,
        collection: &CollectionPath,
        aux_path: &AuxFilePath,
    ) -> Result<Value, ZoteroApiError> {
        let params = (collection, aux_path);
        self.call_rpc("collection.scanAUX", params).await
    }

    /// Executes a search query string against Better `BibTeX` library indexes.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<Value, ZoteroApiError> {
        let params = vec![query];
        self.call_rpc("item.search", params).await
    }

    /// Fetches Pandoc citeproc filter metadata for `citekeys`.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if the JSON-RPC call fails or returns an RPC error
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn pandoc_filter(
        &self,
        citekeys: &[CitationKey],
        as_csl: bool,
    ) -> Result<Value, ZoteroApiError> {
        let params = (citekeys, as_csl);
        self.call_rpc("item.pandoc_filter", params).await
    }

    /// Sends a JSON-RPC request and decodes the typed result payload.
    ///
    /// # Errors
    ///
    /// - [`BetterBibTeX`] if Better `BibTeX` returns a non-success HTTP status,
    ///   an unsupported JSON-RPC version, an RPC error object, or no result
    /// - [`Network`] if the HTTP request fails or the response body cannot be
    ///   decoded
    ///
    /// [`BetterBibTeX`]: ZoteroApiError::BetterBibTeX
    /// [`Network`]: ZoteroApiError::Network
    async fn call_rpc<P: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: P,
    ) -> Result<R, ZoteroApiError> {
        let req_body = JsonRpcRequest {
            jsonrpc: "2.0",
            method,
            params,
            id: 1,
        };

        let url = format!("{}/json-rpc", self.base_url.trim_end_matches('/'));
        let resp = self.http.post(&url).json(&req_body).send().await?;

        if !resp.status().is_success() {
            return Err(ZoteroApiError::BetterBibTeX(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let rpc_resp: JsonRpcResponse<R> = resp.json().await?;
        if rpc_resp.jsonrpc != "2.0" {
            return Err(ZoteroApiError::BetterBibTeX(format!(
                "Unsupported JSON-RPC version {}",
                rpc_resp.jsonrpc
            )));
        }

        if let Some(err) = rpc_resp.error {
            let detail =
                err.data.map(|d| format!(" (data: {d})")).unwrap_or_default();
            return Err(ZoteroApiError::BetterBibTeX(format!(
                "RPC error {}: {}{detail}",
                err.code, err.message
            )));
        }

        rpc_resp.result.ok_or_else(|| {
            ZoteroApiError::BetterBibTeX(
                "JSON-RPC returned null result".to_owned(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    mod fixtures {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            sync::mpsc::{self, Receiver},
        };

        pub(super) fn http_response(status: &str, body: &str) -> String {
            format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: \
                 application/json\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
        }

        pub(super) fn mock_server(responses: Vec<String>) -> String {
            mock_server_with_requests(responses).0
        }

        pub(super) fn mock_server_with_requests(
            responses: Vec<String>,
        ) -> (String, Receiver<String>) {
            let listener =
                TcpListener::bind("127.0.0.1:0").expect("bind listener");
            let addr = listener.local_addr().expect("local addr");
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) =
                        listener.accept().expect("accept connection");
                    let mut buf = vec![0_u8; 4096];
                    let n = stream.read(&mut buf).expect("read request");
                    let _ = tx.send(
                        String::from_utf8_lossy(
                            buf.get(..n).unwrap_or_default(),
                        )
                        .into_owned(),
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            });
            (format!("http://{addr}"), rx)
        }
    }

    mod call_rpc {
        use super::{
            super::*,
            fixtures::{http_response, mock_server},
        };

        #[tokio::test]
        async fn returns_better_bibtex_error_when_http_status_is_non_success() {
            let base = mock_server(vec![http_response("404 Not Found", "")]);
            let client = BetterBibtexClient::new(base);

            let err = client
                .export_items(
                    &[CitationKey::from("KEY1")],
                    &TranslatorName::from("Better BibTeX"),
                )
                .await
                .unwrap_err();

            assert!(matches!(
                &err,
                ZoteroApiError::BetterBibTeX(msg) if msg.contains("404")
            ));
        }

        #[tokio::test]
        async fn returns_better_bibtex_error_when_response_carries_an_rpc_error()
         {
            let base = mock_server(vec![http_response(
                "200 OK",
                r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"boom"}}"#,
            )]);
            let client = BetterBibtexClient::new(base);

            let err = client
                .export_items(
                    &[CitationKey::from("KEY1")],
                    &TranslatorName::from("Better BibTeX"),
                )
                .await
                .unwrap_err();

            assert!(matches!(
                &err,
                ZoteroApiError::BetterBibTeX(msg) if msg.contains("-32600") && msg.contains("boom")
            ));
        }
    }
}
