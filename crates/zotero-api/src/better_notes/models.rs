//! Data models for Better Notes bridge requests and responses.
//!
//! This module defines serializable Rust data types for Better Notes requests
//! and responses. These models cover note exporting, Markdown conversion,
//! template execution, relation links, and note tree structures used by
//! [`BetterNotesClient`](crate::better_notes::BetterNotesClient).
//!
//! # Main Types
//!
//! - [`TemplateName`] - Template name wrapper
//! - [`NoteExportFormat`] - Export format
//!   ([`Markdown`](NoteExportFormat::Markdown) or
//!   [`Html`](NoteExportFormat::Html))
//! - [`NoteExportResponse`] - Response payload for note export
//! - [`NoteItemResponse`] - Response payload containing a created note's item
//!   key
//! - [`TemplateResponse`] - Response payload for template rendering
//! - [`RelationsResponse`] - Response payload containing note relations
//! - [`NoteRelations`] - Container for inbound and outbound relation links
//! - [`NoteRelationLink`] - Representation of a single directed note link
//! - [`NoteTreeResponse`] - Response payload for nested note trees
//!
//! # Examples
//!
//! ```
//! use zotero_api::better_notes::{NoteExportFormat, TemplateName};
//!
//! let template = TemplateName::from("default");
//! assert_eq!(template.as_str(), "default");
//!
//! let format = NoteExportFormat::Markdown;
//! assert_eq!(format.as_str(), "markdown");
//! ```

use serde::{Deserialize, Serialize};

use crate::keys::ItemKey;

string_newtype!(
    pub TemplateName,
    "Name of a Better Notes template, such as `\"default\"` or a custom \
     template name.",
);

/// Controls the output encoding for Better Notes note export.
///
/// The format is serialized as the lowercase value expected by the bridge:
/// `markdown` or `html`.
#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum NoteExportFormat {
    /// Export the note as Markdown text.
    #[default]
    Markdown,
    /// Export the note as HTML text.
    Html,
}

impl NoteExportFormat {
    /// Returns the lowercase bridge parameter for this export format.
    #[must_use]
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Html => "html",
        }
    }
}

/// Response body returned by the Better Notes note-export endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct NoteExportResponse {
    /// Exported note content formatted as Markdown or HTML.
    pub(crate) content: String,
}

/// Response body returned by the Better Notes note-creation endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct NoteItemResponse {
    /// Item key of the created note.
    #[serde(rename = "itemKey")]
    pub(crate) item_key: ItemKey,
}

/// Response body returned by the Better Notes template-run endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct TemplateResponse {
    /// Rendered template output string.
    pub(crate) result: String,
}

/// Response body returned by the Better Notes note-relations endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct RelationsResponse {
    /// Inbound and outbound note relation links.
    pub(crate) relations: NoteRelations,
}

/// Inbound and outbound note-link relation sets for a Zotero note.
#[derive(Debug, Deserialize, Serialize)]
pub struct NoteRelations {
    /// Links from this note to other notes.
    pub(crate) outbound: Vec<NoteRelationLink>,
    /// Links from other notes to this note.
    pub(crate) inbound: Vec<NoteRelationLink>,
}

/// Single directed Better Notes note-link relation between two notes.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NoteRelationLink {
    /// Library ID of the source note.
    #[serde(rename = "fromLibID")]
    pub(crate) from_lib_id: u64,
    /// Item key of the source note.
    pub(crate) from_key: ItemKey,
    /// Library ID of the target note.
    #[serde(rename = "toLibID")]
    pub(crate) to_lib_id: u64,
    /// Item key of the target note.
    pub(crate) to_key: ItemKey,
    /// Line index containing the source link.
    pub(crate) from_line: u64,
    /// Target line index, if the link targets a line.
    pub(crate) to_line: Option<u64>,
    /// Target heading section, if the link targets a section.
    pub(crate) to_section: Option<String>,
    /// Raw `zotero://note/...` URL string.
    pub(crate) url: String,
}

/// Response body returned by the Better Notes note-tree endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct NoteTreeResponse {
    /// Hierarchical tree structure of notes as JSON
    /// [`Value`](serde_json::Value).
    pub(crate) tree: serde_json::Value,
}
