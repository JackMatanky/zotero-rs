//! Note and annotation operations for the Zotero Local HTTP API.

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
    keys::ItemKey,
    objects::ZoteroItem,
    types::{AnnotationType, ItemType},
};

/// Opaque annotation position payload passed through to the Zotero API as-is.
///
/// The JSON structure varies by annotation type and is not interpreted by this
/// crate. Construct via [`From<serde_json::Value>`].
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct AnnotationPosition(serde_json::Value);

impl AnnotationPosition {
    fn as_zotero_string(&self) -> String {
        self.0.to_string()
    }
}

impl From<serde_json::Value> for AnnotationPosition {
    #[inline]
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

/// Payload for creating a PDF annotation via
/// [`ZoteroClient::create_annotation`].
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnnotationDraft {
    /// Key of the parent PDF attachment item.
    pub parent_attachment_key: ItemKey,
    /// Annotation kind (`highlight`, `underline`, `note`, etc.).
    pub annotation_type: AnnotationType,
    /// Highlighted or extracted text string.
    pub text: Option<String>,
    /// User comment attached to the annotation.
    pub comment: Option<String>,
    /// CSS hex color string (e.g. `"#ffd400"`). Defaults to `"#ffd400"`.
    pub color: Option<String>,
    /// PDF page label where the annotation appears.
    pub page_label: Option<String>,
    /// Serialized annotation coordinates. See [`AnnotationPosition`].
    pub position: AnnotationPosition,
}

impl ZoteroClient {
    /// Creates an HTML note item attached to `parent_item_key`.
    ///
    /// `note_content` must be an HTML string (e.g. `"<p>Hello</p>"`).
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the request
    /// - [`Network`] on connection failure
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn create_note<K: AsRef<str>>(
        &self,
        parent_item_key: K,
        note_content: &str,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let payload = serde_json::json!([{
            "itemType": ItemType::Note,
            "parentItem": parent_item_key.as_ref(),
            "note": note_content,
        }]);

        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.post("/items").json(payload).send().await?;
        crate::client::first_created(res.data, "note")
    }

    /// Creates a PDF annotation item attached to a parent PDF attachment.
    ///
    /// The [`AnnotationDraft::position`] field must be a valid Zotero
    /// annotation position JSON object matching the target PDF's coordinate
    /// system.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the request
    /// - [`Network`] on connection failure
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn create_annotation(
        &self,
        draft: AnnotationDraft,
    ) -> Result<ZoteroItem, ZoteroApiError> {
        let position = draft.position.as_zotero_string();
        let payload = serde_json::json!([{
            "itemType": ItemType::Annotation,
            "parentItem": draft.parent_attachment_key.as_str(),
            "annotationType": draft.annotation_type,
            "annotationText": draft.text,
            "annotationComment": draft.comment.as_deref().unwrap_or(""),
            "annotationColor": draft.color.as_deref().unwrap_or("#ffd400"),
            "annotationPageLabel": draft.page_label,
            "annotationPosition": position,
        }]);
        let res: ZoteroResponse<Vec<ZoteroItem>> =
            self.post("/items").json(payload).send().await?;
        crate::client::first_created(res.data, "annotation")
    }

    /// Synthesizes all annotations and notes attached to `item_key` into
    /// Markdown.
    ///
    /// Output structure:
    /// - `# Annotations & Notes: {title}` heading with DOI and date metadata
    /// - `## PDF Annotations` section with blockquoted highlights and comments
    /// - `## Note Content` / `## Child Notes` sections with HTML note bodies
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if `item_key` does not exist
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    ///
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn synthesize_annotations<K: AsRef<str>>(
        &self,
        item_key: K,
    ) -> Result<String, ZoteroApiError> {
        use std::fmt::Write as _;

        let key = item_key.as_ref();
        let item = self.get_item(key).await?;
        let children = self.get_item_children(key).await.unwrap_or_default();

        let mut md = String::new();
        let title = item.data.title.as_deref().unwrap_or(key);
        let _ = writeln!(md, "# Annotations & Notes: {title}\n");

        if let Some(doi) = item.data.doi.as_deref() {
            let _ = writeln!(md, "**DOI:** {doi}");
        }
        if let Some(date) = item.data.date.as_deref() {
            let _ = writeln!(md, "**Date:** {date}");
        }
        md.push('\n');
        md.push_str(&format_annotations_section(&children));
        md.push_str(&format_notes_section(&item, &children));

        Ok(md)
    }
}

/// Formats PDF annotation children into a `## PDF Annotations` Markdown
/// section.
fn format_annotations_section(children: &[ZoteroItem]) -> String {
    use std::fmt::Write as _;

    let mut section = String::new();
    let annotations: Vec<_> = children
        .iter()
        .filter(|c| c.data.item_type == ItemType::Annotation)
        .collect();

    if annotations.is_empty() {
        return section;
    }

    let _ = writeln!(section, "## PDF Annotations\n");
    for ann in annotations {
        let text = ann.data.annotation_text.as_deref().unwrap_or("");
        let comment = ann.data.annotation_comment.as_deref().unwrap_or("");
        let page = ann.data.annotation_page_label.as_deref().unwrap_or("");

        if !text.is_empty() {
            if page.is_empty() {
                let _ = writeln!(section, "> \"{text}\"");
            } else {
                let _ = writeln!(section, "> \"{text}\" (p. {page})");
            }
        }
        if !comment.is_empty() {
            let _ = writeln!(section, "Comment: {comment}");
        }
        section.push('\n');
    }

    section
}

/// Formats standalone item notes and child note items into Markdown sections.
fn format_notes_section(item: &ZoteroItem, children: &[ZoteroItem]) -> String {
    use std::fmt::Write as _;

    let mut section = String::new();
    let child_notes: Vec<_> = children
        .iter()
        .filter(|c| c.data.item_type == ItemType::Note)
        .collect();

    if item.data.item_type == ItemType::Note {
        if let Some(note) = item.data.note.as_deref() {
            let _ = writeln!(section, "## Note Content\n\n{note}\n");
        }
    }

    if !child_notes.is_empty() {
        let _ = writeln!(section, "## Child Notes\n");
        for (idx, note_item) in child_notes.iter().enumerate() {
            if let Some(body) = note_item.data.note.as_deref() {
                let num = idx.saturating_add(1);
                let _ = writeln!(section, "### Note {num}\n\n{body}\n");
            }
        }
    }

    section
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;
    use crate::client::test_http::{MockServer, http_response, request_body};

    #[tokio::test]
    async fn posts_note_payload_for_parent_item() {
        let response = json!([{
            "key":"NOTE0001",
            "version":1,
            "data":{
                "key":"NOTE0001",
                "version":1,
                "itemType":"note"
            }
        }])
        .to_string();
        let (server, recorded) =
            MockServer::recording(vec![http_response("200 OK", &response)]);
        let client = ZoteroClient::new(server.url());

        let result = client.create_note("PARENT01", "<p>Note</p>").await;

        assert!(result.is_ok(), "note creation should succeed: {result:?}");
        let requests = recorded.lock().expect("request log lock");
        let payload = requests
            .first()
            .and_then(|request| request_body(request).ok())
            .and_then(|body| {
                body.as_array().and_then(|array| array.first()).cloned()
            })
            .unwrap_or_default();
        assert_eq!(payload.get("itemType"), Some(&json!("note")));
        assert_eq!(payload.get("parentItem"), Some(&json!("PARENT01")));
        assert_eq!(payload.get("note"), Some(&json!("<p>Note</p>")));
    }

    mod annotations {
        use super::*;
        use crate::{objects::ZoteroItemData, version::LibraryVersion};

        mod formatting {
            use pretty_assertions::assert_eq;

            use super::*;
            #[test]
            fn formats_annotations_section_with_highlights_and_notes() {
                let mut data = ZoteroItemData {
                    key: ItemKey::from("ANN00001"),
                    version: LibraryVersion::new(1),
                    item_type: ItemType::Annotation,
                    ..Default::default()
                };
                data.annotation_type = Some("highlight".to_owned());
                data.annotation_text = Some("Important concept".to_owned());
                data.annotation_comment = Some("Check this out".to_owned());
                data.annotation_page_label = Some("42".to_owned());

                let annotation = ZoteroItem {
                    key: ItemKey::from("ANN00001"),
                    version: LibraryVersion::new(1),
                    library: None,
                    links: None,
                    meta: None,
                    data,
                };

                let annotations = vec![annotation];
                let result = format_annotations_section(&annotations);

                assert_eq!(
                    result,
                    "## PDF Annotations\n\n> \"Important concept\" (p. \
                     42)\nComment: Check this out\n\n"
                );
            }

            #[test]
            fn formats_standalone_note_section() {
                let mut data = ZoteroItemData {
                    key: ItemKey::from("NOTE0001"),
                    version: LibraryVersion::new(1),
                    item_type: ItemType::Note,
                    ..Default::default()
                };
                data.note = Some("<p>Main note text</p>".to_owned());

                let note_item = ZoteroItem {
                    key: ItemKey::from("NOTE0001"),
                    version: LibraryVersion::new(1),
                    library: None,
                    links: None,
                    meta: None,
                    data,
                };

                let result = format_notes_section(&note_item, &[]);

                assert_eq!(
                    result,
                    "## Note Content\n\n<p>Main note text</p>\n\n"
                );
            }

            #[test]
            fn formats_child_notes_section() {
                let main_item = ZoteroItem {
                    key: ItemKey::from("ITEM0001"),
                    version: LibraryVersion::new(1),
                    library: None,
                    links: None,
                    meta: None,
                    data: ZoteroItemData {
                        key: ItemKey::from("ITEM0001"),
                        version: LibraryVersion::new(1),
                        item_type: ItemType::JournalArticle,
                        ..Default::default()
                    },
                };
                let mut child_data = ZoteroItemData {
                    key: ItemKey::from("NOTE0001"),
                    version: LibraryVersion::new(1),
                    item_type: ItemType::Note,
                    ..Default::default()
                };
                child_data.note = Some("<p>Child note text</p>".to_owned());

                let child_note = ZoteroItem {
                    key: ItemKey::from("NOTE0001"),
                    version: LibraryVersion::new(1),
                    library: None,
                    links: None,
                    meta: None,
                    data: child_data,
                };

                let child_notes = vec![child_note];
                let result = format_notes_section(&main_item, &child_notes);

                assert_eq!(
                    result,
                    "## Child Notes\n\n### Note 1\n\n<p>Child note \
                     text</p>\n\n"
                );
            }
        }
    }
}
