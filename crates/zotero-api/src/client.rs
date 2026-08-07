//! Async HTTP client for the Zotero Local API.
//!
//! Defines [`ZoteroClient`], the primary HTTP request builder and dispatcher
//! for Zotero Local API operations. The client handles target library scoping,
//! authentication headers, error conversion, and response decoding.

use std::time::Duration;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    errors::ZoteroApiError,
    keys::LibraryVersion,
    objects::{LocalApiStatus, ZoteroItem},
};

/// Generic envelope wrapping API response payloads alongside Zotero response
/// headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoteroResponse<T> {
    /// Deserialized response body payload.
    pub data: T,
    /// Total matching items count from the `Total-Results` header, if present.
    pub total_results: Option<usize>,
    /// Server library version from `Last-Modified-Version` header, if present.
    pub last_modified_version: Option<u64>,
    /// Unique server identifier from `Zotero-Server-ID` header, if present.
    pub server_id: Option<String>,
}

impl<T> std::ops::Deref for ZoteroResponse<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// One page of Zotero items and the optional `Total-Results` header count.
pub(super) struct ItemsPage {
    /// Fetched items for the requested page.
    pub(super) items: Vec<ZoteroItem>,
    /// Total number of matching items across all pages, if provided by Zotero.
    pub(super) total: Option<usize>,
}

/// Target Zotero library (User or Group).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LibraryTarget {
    /// User library with ID (default `User(0)` for active local user).
    User(u64),
    /// Group library with group ID.
    Group(u64),
}

impl Default for LibraryTarget {
    #[inline]
    fn default() -> Self {
        Self::User(0)
    }
}

impl LibraryTarget {
    /// Returns the URL path prefix for this library target (e.g. `/users/0` or
    /// `/groups/12345`).
    #[must_use]
    #[inline]
    pub fn target_prefix(&self) -> String {
        match self {
            Self::User(id) => format!("/users/{id}"),
            Self::Group(id) => format!("/groups/{id}"),
        }
    }
}

/// Response payload returned by `POST /api/local/authorize`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalAuthResponse {
    /// Generated local API write key / token.
    pub secret: String,
    /// Optional backoff delay in seconds if user interaction is pending.
    pub backoff: Option<u64>,
}

/// Owned, `'static` async client for the Zotero Local HTTP API.
#[derive(Clone, Debug)]
pub struct ZoteroClient {
    pub(super) http: reqwest::Client,
    pub(super) base_url: String,
    pub(super) api_key: Option<String>,
    pub(super) server_id: Option<String>,
    pub(super) target: LibraryTarget,
}

impl Default for ZoteroClient {
    #[inline]
    fn default() -> Self {
        Self::new("http://127.0.0.1:23119/api")
    }
}

impl ZoteroClient {
    /// Creates a new [`ZoteroClient`] with the specified base URL.
    #[inline]
    pub fn new<S: Into<String>>(base_url: S) -> Self {
        let base_url = base_url.into();
        Self {
            http: reqwest::Client::new(),
            base_url: if base_url.is_empty() {
                "http://127.0.0.1:23119/api".to_owned()
            } else {
                base_url
            },
            api_key: None,
            server_id: None,
            target: LibraryTarget::default(),
        }
    }

    /// Configures a custom [`reqwest::Client`] HTTP client pool.
    #[must_use]
    #[inline]
    pub fn with_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    /// Configures the API key (`Zotero-API-Key` or `Zotero-Write-Key`).
    #[must_use]
    #[inline]
    pub fn with_api_key<S: Into<String>>(mut self, key: S) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Configures the expected server ID (`Zotero-Server-ID`).
    #[must_use]
    #[inline]
    pub fn with_server_id<S: Into<String>>(mut self, server_id: S) -> Self {
        self.server_id = Some(server_id.into());
        self
    }

    /// Scopes the client to a specific [`LibraryTarget`] (User or Group).
    #[must_use]
    #[inline]
    pub fn with_target(mut self, target: LibraryTarget) -> Self {
        self.target = target;
        self
    }

    /// Returns a reference to the inner [`reqwest::Client`].
    #[must_use]
    #[inline]
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Returns the configured base URL string.
    #[must_use]
    #[inline]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the active [`LibraryTarget`].
    #[must_use]
    #[inline]
    pub fn target(&self) -> LibraryTarget {
        self.target
    }

    /// Returns the target library URL prefix (e.g. `/users/0` or
    /// `/groups/12345`).
    #[must_use]
    #[inline]
    pub fn target_prefix(&self) -> String {
        self.target.target_prefix()
    }

    /// Creates a fluent request builder for a `GET` request.
    #[inline]
    pub fn get<K: Into<String>>(&self, path: K) -> ApiRequestBuilder<'_> {
        ApiRequestBuilder::new(self, reqwest::Method::GET, path)
    }

    /// Creates a fluent request builder for a `POST` request.
    #[inline]
    pub fn post<K: Into<String>>(&self, path: K) -> ApiRequestBuilder<'_> {
        ApiRequestBuilder::new(self, reqwest::Method::POST, path)
    }

    /// Creates a fluent request builder for a `PUT` request.
    #[inline]
    pub fn put<K: Into<String>>(&self, path: K) -> ApiRequestBuilder<'_> {
        ApiRequestBuilder::new(self, reqwest::Method::PUT, path)
    }

    /// Creates a fluent request builder for a `PATCH` request.
    #[inline]
    pub fn patch<K: Into<String>>(&self, path: K) -> ApiRequestBuilder<'_> {
        ApiRequestBuilder::new(self, reqwest::Method::PATCH, path)
    }

    /// Creates a fluent request builder for a `DELETE` request.
    #[inline]
    pub fn delete_req<K: Into<String>>(
        &self,
        path: K,
    ) -> ApiRequestBuilder<'_> {
        ApiRequestBuilder::new(self, reqwest::Method::DELETE, path)
    }

    /// Probes the Zotero Local API for availability.
    #[inline]
    pub async fn check_status(&self) -> LocalApiStatus {
        match self.get("/items").query("limit", "1").send_raw().await {
            Ok(resp) => {
                let version = resp
                    .headers()
                    .get("zotero-api-version")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned);
                let status = resp.status();
                if status.is_success() {
                    LocalApiStatus {
                        online: true,
                        url: self.base_url.clone(),
                        version,
                        error: None,
                    }
                } else {
                    LocalApiStatus {
                        online: false,
                        url: self.base_url.clone(),
                        version: None,
                        error: Some(format!("HTTP status {status}")),
                    }
                }
            }
            Err(e) => LocalApiStatus {
                online: false,
                url: self.base_url.clone(),
                version: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// Requests local API write authorization via `POST /api/local/authorize`.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`] if Zotero rejects the request, or
    /// [`ZoteroApiError::Network`] if the request fails.
    #[inline]
    pub async fn request_local_authorization(
        &self,
        app_name: &str,
    ) -> Result<LocalAuthResponse, ZoteroApiError> {
        let res: ZoteroResponse<LocalAuthResponse> = self
            .post("/local/authorize")
            .target_scoped(false)
            .json(serde_json::json!({ "appName": app_name }))
            .send()
            .await?;
        Ok(res.data)
    }

    /// Helper: GET request returning decoded JSON payload.
    pub(super) async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, ZoteroApiError> {
        let res: ZoteroResponse<T> =
            self.get(url).target_scoped(false).send().await?;
        Ok(res.data)
    }

    /// Helper: Fetches every page of a paginated list endpoint.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if any page request fails.
    #[inline]
    pub async fn get_all_json<T: DeserializeOwned>(
        &self,
        url: &str,
        page_size: usize,
    ) -> Result<Vec<T>, ZoteroApiError> {
        if page_size == 0 {
            return Ok(Vec::new());
        }
        let mut all = Vec::new();
        let mut start = 0_usize;
        loop {
            let page_url = add_pagination(url, start, page_size);
            let page: Vec<T> = self.get_json(&page_url).await?;
            let len = page.len();
            all.extend(page);
            if len < page_size {
                break;
            }
            start = start.saturating_add(page_size);
        }
        Ok(all)
    }

    /// Helper: Fetches one page of items alongside the `Total-Results` count.
    pub(super) async fn get_items_with_total(
        &self,
        url: &str,
    ) -> Result<ItemsPage, ZoteroApiError> {
        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.get(url).target_scoped(false).send().await?;
        Ok(ItemsPage {
            items: res.data,
            total: res.total_results,
        })
    }

    /// Fetches the current library version counter via `Last-Modified-Version`
    /// header.
    pub(super) async fn get_library_version(
        &self,
    ) -> Result<LibraryVersion, ZoteroApiError> {
        let res: ZoteroResponse<serde_json::Value> =
            self.get("/items").query("limit", "1").send().await?;
        res.last_modified_version.map(LibraryVersion::from).ok_or_else(|| {
            ZoteroApiError::LocalApi {
                status: 0,
                message: "Missing or invalid Last-Modified-Version header"
                    .to_owned(),
            }
        })
    }
}

/// Fluent builder for HTTP requests to Zotero API endpoints.
pub struct ApiRequestBuilder<'a> {
    client: &'a ZoteroClient,
    method: reqwest::Method,
    path: String,
    target_scoped: bool,
    query: Vec<(String, String)>,
    unmodified_since_version: Option<u64>,
    json_body: Option<serde_json::Value>,
}

impl<'a> ApiRequestBuilder<'a> {
    /// Creates a new request builder.
    #[inline]
    pub fn new<K: Into<String>>(
        client: &'a ZoteroClient,
        method: reqwest::Method,
        path: K,
    ) -> Self {
        Self {
            client,
            method,
            path: path.into(),
            target_scoped: true,
            query: Vec::new(),
            unmodified_since_version: None,
            json_body: None,
        }
    }

    /// Sets whether the request path is automatically prefixed with target
    /// library (default `true`).
    #[must_use]
    #[inline]
    pub fn target_scoped(mut self, scoped: bool) -> Self {
        self.target_scoped = scoped;
        self
    }

    /// Appends a query parameter key-value pair.
    #[must_use]
    #[inline]
    pub fn query<K: Into<String>, V: Into<String>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    /// Appends an optional query parameter key-value pair if value is `Some`.
    #[must_use]
    #[inline]
    pub fn query_opt<K: Into<String>, V: Into<String>>(
        mut self,
        key: K,
        value: Option<V>,
    ) -> Self {
        if let Some(v) = value {
            self.query.push((key.into(), v.into()));
        }
        self
    }

    /// Sets the `If-Unmodified-Since-Version` header.
    #[must_use]
    #[inline]
    pub fn unmodified_since_version(mut self, version: u64) -> Self {
        self.unmodified_since_version = Some(version);
        self
    }

    /// Sets a JSON body payload.
    #[must_use]
    #[inline]
    pub fn json(mut self, body: serde_json::Value) -> Self {
        self.json_body = Some(body);
        self
    }

    /// Sends the request, returning the raw [`reqwest::Response`].
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::Network`] if every retry attempt fails at the
    /// transport level.
    #[inline]
    pub async fn send_raw(&self) -> Result<reqwest::Response, ZoteroApiError> {
        let full_url = if self.path.starts_with("http://")
            || self.path.starts_with("https://")
        {
            self.path.clone()
        } else {
            let base = self.client.base_url.trim_end_matches('/');
            if self.target_scoped {
                format!("{base}{}{}", self.client.target_prefix(), self.path)
            } else {
                format!("{base}{}", self.path)
            }
        };

        let mut req = self.client.http.request(self.method.clone(), &full_url);
        req = req.header("Zotero-API-Version", "3");
        if let Some(key) = &self.client.api_key {
            req = req.header("Zotero-API-Key", key);
            req = req.header("Zotero-Write-Key", key);
        }
        if let Some(server_id) = &self.client.server_id {
            req = req.header("Zotero-Server-ID", server_id);
        }
        if let Some(version) = self.unmodified_since_version {
            req =
                req.header("If-Unmodified-Since-Version", version.to_string());
        }
        if !self.query.is_empty() {
            req = req.query(&self.query);
        }
        if let Some(body) = &self.json_body {
            req = req.json(body);
        }

        let mut attempts = 0_u32;
        loop {
            attempts = attempts.saturating_add(1);
            let req_builder = req.try_clone().unwrap_or_else(|| {
                self.client.http.request(self.method.clone(), &full_url)
            });
            match req_builder.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if (status.is_server_error()
                        || status == reqwest::StatusCode::TOO_MANY_REQUESTS)
                        && attempts < 3
                    {
                        tokio::time::sleep(Duration::from_millis(
                            retry_delay_ms(attempts),
                        ))
                        .await;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(_e) if attempts < 3 => {
                    tokio::time::sleep(Duration::from_millis(retry_delay_ms(
                        attempts,
                    )))
                    .await;
                }
                Err(e) => return Err(ZoteroApiError::Network(e)),
            }
        }
    }

    /// Sends the request and deserializes the JSON response body into
    /// [`ZoteroResponse<T>`].
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`] if the response status is not 2xx,
    /// or [`ZoteroApiError::Network`]/[`ZoteroApiError::Json`] if the request
    /// fails or the body cannot be decoded.
    #[inline]
    pub async fn send<T: DeserializeOwned>(
        &self,
    ) -> Result<ZoteroResponse<T>, ZoteroApiError> {
        let resp = self.send_raw().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(ZoteroApiError::LocalApi {
                status: status.as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let total_results = resp
            .headers()
            .get("Total-Results")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok());
        let last_modified_version = resp
            .headers()
            .get("Last-Modified-Version")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let server_id = resp
            .headers()
            .get("Zotero-Server-ID")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        let data: T = resp.json().await?;
        Ok(ZoteroResponse {
            data,
            total_results,
            last_modified_version,
            server_id,
        })
    }

    /// Sends the request and checks for a successful (2xx) status without
    /// attempting to decode a response body.
    ///
    /// Use for endpoints that return `204 No Content` (or any other body
    /// that is not a `ZoteroResponse` envelope), such as `DELETE` requests.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`] if the response status is not
    /// 2xx, or [`ZoteroApiError::Network`] if the request fails.
    #[inline]
    pub async fn send_unit(&self) -> Result<(), ZoteroApiError> {
        let resp = self.send_raw().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(ZoteroApiError::LocalApi {
                status: status.as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }
}

/// Computes an exponential backoff delay in milliseconds for retry `attempt`
/// (1-indexed): 200ms, 400ms, 800ms, ...
fn retry_delay_ms(attempt: u32) -> u64 {
    200_u64.saturating_mul(1_u64 << attempt.saturating_sub(1).min(16))
}

/// Appends `start` and `limit` query parameters to `url`, preserving any
/// existing query string.
pub(super) fn add_pagination(url: &str, start: usize, limit: usize) -> String {
    let sep = if url.contains('?') {
        '&'
    } else {
        '?'
    };
    format!("{url}{sep}start={start}&limit={limit}")
}

#[cfg(test)]
mod tests {

    mod check_status {
        use pretty_assertions::assert_eq;

        use super::super::*;
        use crate::client::test_http::{
            MockServer, http_response, http_response_with_headers,
        };

        #[tokio::test]
        async fn returns_online_true_on_200_ok() {
            let server = MockServer::new(vec![http_response("200 OK", "[]")]);
            let client = ZoteroClient::new(server.url());

            let status = client.check_status().await;

            assert!(status.online);
            assert_eq!(status.error, None);
        }

        #[tokio::test]
        async fn returns_online_false_with_error_on_500() {
            let server = MockServer::new(vec![
                http_response("500 Internal Error", ""),
                http_response("500 Internal Error", ""),
                http_response("500 Internal Error", ""),
            ]);
            let client = ZoteroClient::new(server.url());

            let status = client.check_status().await;

            assert!(!status.online);
            assert_eq!(
                status.error,
                Some("HTTP status 500 Internal Server Error".to_owned())
            );
        }

        #[tokio::test]
        async fn check_status_captures_api_version_header() {
            let server = MockServer::new(vec![http_response_with_headers(
                "200 OK",
                &[("zotero-api-version", "7.0.0")],
                "[]",
            )]);
            let client = ZoteroClient::new(server.url());

            let status = client.check_status().await;

            assert!(status.online);
            assert_eq!(status.version.as_deref(), Some("7.0.0"));
        }

        #[tokio::test]
        async fn returns_online_false_on_connection_failure() {
            let client = ZoteroClient::new("http://127.0.0.1:1");

            let status = client.check_status().await;

            assert!(!status.online);
            assert!(status.error.is_some());
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub mod test_http {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        thread::{self, JoinHandle},
    };

    pub type RequestLog = Arc<Mutex<Vec<String>>>;

    pub struct MockServer {
        base_url: String,
        stop: Arc<AtomicBool>,
        handle: Option<JoinHandle<()>>,
    }

    impl MockServer {
        #[must_use]
        #[inline]
        pub fn new(responses: Vec<String>) -> Self {
            Self::with_log(responses, None)
        }

        #[must_use]
        #[inline]
        pub fn recording(responses: Vec<String>) -> (Self, RequestLog) {
            let recorded = Arc::new(Mutex::new(Vec::new()));
            let server = Self::with_log(responses, Some(Arc::clone(&recorded)));
            (server, recorded)
        }

        #[must_use]
        #[inline]
        pub fn url(&self) -> &str {
            &self.base_url
        }

        fn with_log(
            responses: Vec<String>,
            recorded: Option<RequestLog>,
        ) -> Self {
            #[expect(clippy::expect_used, reason = "test-only mock server")]
            let listener =
                TcpListener::bind("127.0.0.1:0").expect("bind test listener");
            #[expect(clippy::expect_used, reason = "test-only mock server")]
            let addr = listener.local_addr().expect("test listener address");
            let stop = Arc::new(AtomicBool::new(false));
            let thread_stop = Arc::clone(&stop);
            let handle = thread::spawn(move || {
                serve_responses(
                    &listener,
                    &responses,
                    recorded.as_ref(),
                    &thread_stop,
                );
            });

            Self {
                base_url: format!("http://{addr}"),
                stop,
                handle: Some(handle),
            }
        }
    }

    impl Drop for MockServer {
        #[inline]
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(addr) = self.base_url.strip_prefix("http://") {
                let _ = TcpStream::connect(addr);
            }
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    #[must_use]
    #[inline]
    pub fn http_response(status: &str, body: &str) -> String {
        http_response_with_headers(status, &[], body)
    }

    #[must_use]
    #[inline]
    pub fn http_response_with_headers(
        status: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> String {
        let mut response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: \
             application/json\r\n",
            body.len()
        );
        for (name, value) in headers {
            response.push_str(name);
            response.push_str(": ");
            response.push_str(value);
            response.push_str("\r\n");
        }
        response.push_str("Connection: close\r\n\r\n");
        response.push_str(body);
        response
    }

    fn serve_responses(
        listener: &TcpListener,
        responses: &[String],
        recorded: Option<&RequestLog>,
        stop: &AtomicBool,
    ) {
        for response in responses {
            if !serve_response(listener, response, recorded, stop) {
                break;
            }
        }
    }

    fn serve_response(
        listener: &TcpListener,
        response: &str,
        recorded: Option<&RequestLog>,
        stop: &AtomicBool,
    ) -> bool {
        let Ok((mut stream, _)) = listener.accept() else {
            return false;
        };
        if stop.load(Ordering::Acquire) {
            return false;
        }
        record_or_drain_request(&mut stream, recorded);
        let _ = stream.write_all(response.as_bytes());
        true
    }

    fn record_or_drain_request(
        stream: &mut TcpStream,
        recorded: Option<&RequestLog>,
    ) {
        if let Some(recorded) = recorded {
            #[expect(clippy::expect_used, reason = "test-only mock server")]
            let mut log = recorded.lock().expect("request log lock");
            log.push(read_request(stream));
            return;
        }
        let mut buf = [0_u8; 1024];
        let _ = stream.read(&mut buf);
    }

    /// # Errors
    ///
    /// Returns an error if the request body is not valid JSON.
    #[inline]
    pub fn request_body(
        raw: &str,
    ) -> Result<serde_json::Value, serde_json::Error> {
        let body = raw.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        serde_json::from_str(body)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut buf = [0_u8; 1024];
        let mut data = Vec::new();
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            data.extend_from_slice(buf.get(..n).unwrap_or_default());
            if request_complete(&data) {
                break;
            }
        }
        String::from_utf8_lossy(&data).into_owned()
    }

    fn request_complete(data: &[u8]) -> bool {
        let Some((head_end, content_length)) = request_meta(data) else {
            return false;
        };
        data.len() >= head_end.saturating_add(content_length)
    }

    fn request_meta(data: &[u8]) -> Option<(usize, usize)> {
        let head_end =
            data.windows(4).position(|w| w == b"\r\n\r\n")?.saturating_add(4);
        let head =
            String::from_utf8_lossy(data.get(..head_end).unwrap_or_default());
        let content_length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        Some((head_end, content_length))
    }
}
