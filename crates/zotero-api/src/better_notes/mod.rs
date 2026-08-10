//! Integration with the Better Notes Zotero plugin.
//!
//! This module exposes [`BetterNotesClient`], an async client for the plugin's
//! HTTP companion endpoint, plus small model types for note export and template
//! execution.
//!
//! # Examples
//!
//! ```no_run
//! use zotero_api::{BetterNotesClient, NoteExportFormat};
//!
//! # async fn run() -> Result<(), zotero_api::ZoteroApiError> {
//! let client = BetterNotesClient::default();
//! let markdown =
//!     client.export("ABC12345", Some(NoteExportFormat::Markdown)).await?;
//! assert!(!markdown.is_empty());
//! # Ok(())
//! # }
//! ```

mod client;
mod models;

pub use client::BetterNotesClient;
pub use models::{NoteExportFormat, TemplateName};
