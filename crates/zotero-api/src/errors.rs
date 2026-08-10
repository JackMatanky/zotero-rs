//! Unified error type for all Zotero API operations.
//!
//! [`ZoteroApiError`] wraps HTTP, network, serialization, and Zotero-specific
//! errors into a single type returned by fallible operations in this crate.

use thiserror::Error;

/// Unified error type for all Zotero API operations.
///
/// Wraps HTTP, network, serialization, and Zotero-specific errors.
///
/// # Examples
///
/// ```rust
/// use zotero_api::ZoteroApiError;
///
/// match result {
///     Err(ZoteroApiError::NotFound(id)) => println!("not found: {id}"),
///     Err(ZoteroApiError::VersionConflict(_)) => println!("conflict, retry"),
///     Err(ZoteroApiError::LocalApi {
///         status,
///         ..
///     }) => println!("HTTP {status}"),
///     _ => {}
/// }
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ZoteroApiError {
    /// Network or HTTP transport failure from [`reqwest`].
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Zotero Local API returned a non-2xx HTTP status.
    #[error("Local API error: HTTP {status} - {message}")]
    LocalApi {
        /// HTTP status code returned by the Zotero Local API.
        status: u16,
        /// Error message or body returned by the Zotero Local API.
        message: String,
    },

    /// Write rejected because the target object's library version no longer
    /// matches the `If-Unmodified-Since-Version` header (HTTP 412).
    ///
    /// Another client modified the object since it was last fetched. Refetch
    /// the object's current version and retry, or surface the conflict to the
    /// user.
    #[error("Version conflict: {0}")]
    VersionConflict(String),

    /// Better BibTeX JSON-RPC endpoint returned an error or invalid response.
    #[error("Better BibTeX error: {0}")]
    BetterBibTeX(String),

    /// Better Notes companion bridge endpoint returned an error or invalid
    /// response.
    #[error("Better Notes error: {0}")]
    BetterNotes(String),

    /// PDF text extraction failed.
    #[error("PDF extraction error: {0}")]
    PdfExtract(String),

    /// Local embedding generation failed.
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// I/O failure from [`std::io`].
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Local Zotero SQLite database could not be located or read.
    #[error("Local database error: {0}")]
    LocalDb(String),

    /// SQLite query or connection failed. Requires the `sqlite` feature.
    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    Sqlite(#[from] sqlx::Error),

    /// Write operation rejected by the embedding application's security policy.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// User-controlled input failed local security policy validation.
    #[error("Input rejected: {0}")]
    InputRejected(String),

    /// Requested Zotero library item, collection, or resource was not found
    /// (HTTP 404).
    #[error("Item not found: {0}")]
    NotFound(String),

    /// JSON serialization or deserialization failure from [`serde_json`].
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}
