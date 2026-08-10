//! Integration with the Better Notes Zotero plugin.
//!
//! This module exposes [`BetterNotesClient`], an async client for the plugin's
//! HTTP companion endpoint, plus small model types for note export and template
//! execution.

mod client;
mod models;

pub use client::BetterNotesClient;
pub use models::{NoteExportFormat, TemplateName};
