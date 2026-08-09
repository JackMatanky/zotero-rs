//! Domain library for the Zotero Local API, Better `BibTeX`, Better Notes, and
//! semantic search.
//!
//! `zotero-api` provides strongly-typed, async Rust abstractions for inspecting
//! and mutating local Zotero reference management libraries. It supports the
//! HTTP Local API, Better `BibTeX` export engine, Better Notes companion
//! plugin, local `SQLite` database access, and vector semantic search.
//!
//! # Main Components
//!
//! - [`ZoteroClient`]: Core HTTP client for the Zotero Local API (items,
//!   collections, tags, searches, keys).
//! - [`BetterBibtexClient`]: Client for the Better `BibTeX` extension (citation
//!   keys, JSON-RPC auto-export, Aux scanning).
//! - [`BetterNotesClient`]: Client for the Better Notes plugin (Markdown
//!   conversion, note exporting).
//! - [`LocalZoteroDb`]: Direct read-only `SQLite` database query interface.
//! - [`SemanticIndex`]: Local vector embedding index for note and annotation
//!   similarity search.
//! # Examples
//!
//! ```no_run
//! use zotero_api::ZoteroClient;
//!
//! # async fn run() -> Result<(), zotero_api::ZoteroApiError> {
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
pub(crate) mod bibtex;
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
pub use bibtex::{item_to_bibtex, items_to_bibtex};
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
    ItemQueryParams, JoinMode, PaginationInfo, QuickSearchMode,
    SearchCondition, SearchField, SearchOperator, SearchPage, SortDirection,
    SortField, SortOrder,
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
