//! Better BibTeX integration for Zotero.
//!
//! This module provides [`BetterBibtexClient`], a JSON-RPC 2.0 client for the
//! Better BibTeX Zotero plugin. It supports citation-key lookup, item export,
//! bibliography generation, AUX scanning, search, Pandoc metadata, and
//! auto-export registration.

mod client;
mod models;

pub use client::BetterBibtexClient;
pub use models::{
    AutoExportAddRequest, AuxFilePath, BibliographyContentType,
    BibliographyFormat, CollectionPath, CslStyleId, Locale, SearchQuery,
    TranslatorName,
};
