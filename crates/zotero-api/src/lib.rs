//! Async client and types for the Zotero Local API, Better BibTeX, and Better
//! Notes.
//!
//! `zotero-api` provides typed, async Rust abstractions for inspecting and
//! mutating local Zotero reference management libraries via the HTTP Local API,
//! Better BibTeX export engine, Better Notes companion plugin, and read-only
//! SQLite database access.
//!
//! # Main Components
//!
//! - [`ZoteroClient`] — Core HTTP client for the Zotero Local API (items,
//!   collections, tags, searches, keys).
//! - [`BetterBibtexClient`] — Client for the Better BibTeX extension (citation
//!   keys, JSON-RPC auto-export, Aux scanning).
//! - [`BetterNotesClient`] — Client for the Better Notes plugin (Markdown
//!   conversion, note exporting).
//! - `LocalZoteroDb` (behind the `sqlite` feature) — Direct read-only SQLite
//!   database query interface.
//!
//! # Features
//!
//! | Feature     | Description                                            |
//! |-------------|--------------------------------------------------------|
//! | `metadata`  | Enables `resolve_metadata` for identifier resolution.  |
//! | `pdf`       | Enables PDF annotation extraction and export.           |
//! | `sqlite`    | Enables `LocalZoteroDb` for direct SQLite access.      |
//! | `test-util` | Exposes test helpers and fixtures for downstream tests. |
//! | `full`      | Enables all optional features.                         |
//!
//! # Examples
//!
//! Check whether a local Zotero instance is reachable:
//!
//! ```no_run
//! use zotero_api::{ZoteroApiError, ZoteroClient};
//!
//! # async fn run() -> Result<(), ZoteroApiError> {
//! let client = ZoteroClient::new("http://127.0.0.1:23119/api");
//! let status = client.check_status().await;
//! println!("Status online: {}", status.online);
//! # Ok(())
//! # }
//! ```

#[macro_use]
mod macros;
pub(crate) mod analysis;
pub mod better_bibtex;
pub mod better_notes;
pub mod client;
pub(crate) mod collections;
pub(crate) mod deleted;
pub mod errors;
pub(crate) mod items;
pub(crate) mod keys;
#[cfg(feature = "metadata")]
pub mod metadata;
pub(crate) mod notes;
pub(crate) mod objects;
#[cfg(feature = "pdf")]
pub mod pdf;
pub(crate) mod relations;
pub(crate) mod search;
pub(crate) mod searches;
pub(crate) mod settings;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub(crate) mod tags;
pub(crate) mod types;

pub use analysis::{
    DuplicateGroup, DuplicateType, LibraryCoverage, LibraryCoveragePage,
};
pub use better_bibtex::{
    AutoExportAddRequest, AuxFilePath, BetterBibtexClient,
    BibliographyContentType, BibliographyFormat, CollectionPath, CslStyleId,
    Locale, SearchQuery, TranslatorName,
};
pub use better_notes::{BetterNotesClient, NoteExportFormat, TemplateName};
pub use client::{LibraryTarget, LocalAuthResponse, ZoteroClient};
pub use collections::CollectionItemAction;
pub use deleted::DeletedObjectsResponse;
pub use errors::ZoteroApiError;
pub use items::TrashAction;
pub use keys::{CitationKey, CollectionKey, ItemKey, LibraryVersion, TagName};
#[cfg(feature = "metadata")]
pub use metadata::{
    IdentifierKind, resolve_metadata, resolve_metadata_with_urls,
};
pub use notes::{AnnotationDraft, AnnotationPosition};
pub use objects::{
    BatchWriteResponse, ItemDraft, ItemLinks, ItemMeta, LibraryInfo,
    LocalApiStatus, ZoteroCollection, ZoteroItem,
};
#[cfg(feature = "pdf")]
pub use pdf::*;
pub use relations::RelatedItem;
pub use search::{
    JoinMode, PaginationInfo, SearchCondition, SearchField, SearchOperator,
    SearchPage, SortField, SortOrder,
};
pub use searches::SavedSearch;
pub use settings::SettingEntry;
#[cfg(feature = "sqlite")]
pub use sqlite::{
    FulltextHit, LocalZoteroDb, NoteAnnotationHit, find_zotero_db,
};
pub use types::{
    AnnotationType, CollectionParent, CreatorType, ItemType, LinkMode,
    TagOrigin,
};
