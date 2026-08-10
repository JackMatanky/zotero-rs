//! Better `BibTeX` integration for Zotero.
//!
//! This module provides [`BetterBibtexClient`], a JSON-RPC 2.0 client for the
//! Better `BibTeX` Zotero plugin. It supports citation-key lookup, item export,
//! bibliography generation, AUX scanning, search, Pandoc metadata, and
//! auto-export registration.
//!
//! # Examples
//!
//! ```no_run
//! use zotero_api::{
//!     BetterBibtexClient, ItemKey, better_bibtex::TranslatorName,
//! };
//!
//! # async fn run() -> Result<(), zotero_api::ZoteroApiError> {
//! let client = BetterBibtexClient::default();
//! let citekeys = client.get_citekeys(&[ItemKey::from("ABC12345")]).await?;
//! let citekeys: Vec<_> = citekeys.values().flatten().cloned().collect();
//! let bibtex = client
//!     .export_items(&citekeys, &TranslatorName::from("Better BibTeX"))
//!     .await?;
//! assert!(!bibtex.is_empty());
//! # Ok(())
//! # }
//! ```

mod client;
mod models;

pub use client::BetterBibtexClient;
pub use models::{
    AutoExportAddRequest, AuxFilePath, BibliographyContentType,
    BibliographyFormat, CollectionPath, CslStyleId, Locale, SearchQuery,
    TranslatorName,
};
