//! Client-side library analytics: coverage statistics and duplicate detection.
//!
//! These operations fetch items via [`ZoteroClient`] and compute aggregate
//! metrics client-side; none of them are Zotero API endpoints.

use serde::{Deserialize, Serialize};

use crate::{
    client::ZoteroClient, errors::ZoteroApiError, objects::ZoteroItem,
    search::PaginationInfo, types::ItemType,
};

/// Coverage indicators for a single library item.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "domain model tracks 3 distinct boolean flags"
)]
struct ItemCoverageFlags {
    has_pdf: bool,
    has_doi: bool,
    has_notes: bool,
}

/// Client-side aggregate coverage statistics for a library or collection.
///
/// The counts and percentages describe how many returned parent items have:
///
/// - a PDF attachment
/// - a non-empty DOI
/// - at least one child note
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LibraryCoverage {
    pub total_items: usize,
    pub with_pdf: usize,
    pub with_doi: usize,
    pub with_notes: usize,
    pub pdf_percentage: f64,
    pub doi_percentage: f64,
    pub notes_percentage: f64,
}

/// Paginated library coverage results.
///
/// Combines one page of [`LibraryCoverage`] data with [`PaginationInfo`] so
/// callers can display the aggregate metrics and request the next page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibraryCoveragePage {
    pub coverage: LibraryCoverage,
    pub pagination: PaginationInfo,
}

impl ZoteroClient {
    /// Computes client-side PDF, DOI, and note coverage for one item page.
    ///
    /// The selected page excludes standalone notes when scanning the full
    /// library. For each returned item, this method fetches child items and
    /// computes counts plus percentages for:
    ///
    /// - items with PDF attachments
    /// - items with non-empty DOIs
    /// - items with child notes
    ///
    /// # Arguments
    ///
    /// * `collection_key` - Optional collection key to limit the scan. When
    ///   omitted, the full target library is scanned.
    /// * `offset` - Zero-based item offset for the page to analyze.
    /// * `limit` - Maximum number of items to fetch and classify.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn get_library_coverage<K: AsRef<str>>(
        &self,
        collection_key: Option<K>,
        offset: usize,
        limit: usize,
    ) -> Result<LibraryCoveragePage, ZoteroApiError> {
        let base = match &collection_key {
            Some(col) => format!(
                "{}{}/collections/{}/items",
                self.base_url.trim_end_matches('/'),
                self.target_prefix(),
                col.as_ref()
            ),
            None => format!(
                "{}{}/items?itemType=-note&sort=dateModified&direction=desc",
                self.base_url.trim_end_matches('/'),
                self.target_prefix()
            ),
        };
        let page_url = crate::client::add_pagination(&base, offset, limit);
        let page = self.get_items_with_total(&page_url).await?;
        let pagination = PaginationInfo::from_page(
            offset,
            limit,
            page.items.len(),
            page.total,
        );

        let mut children_by_idx = Vec::with_capacity(page.items.len());
        for item in &page.items {
            children_by_idx.push(
                self.get_item_children(&item.key).await.unwrap_or_default(),
            );
        }

        Ok(classify_coverage_page(&page.items, &children_by_idx, pagination))
    }
}

/// Evaluates PDF, DOI, and note availability flags for a single `item`.
fn coverage_flags(
    item: &ZoteroItem,
    children: &[ZoteroItem],
) -> ItemCoverageFlags {
    let has_doi =
        item.data.doi.as_deref().is_some_and(|d| !d.trim().is_empty());
    let has_pdf = children.iter().any(|child| {
        child.data.item_type == ItemType::Attachment
            && child
                .data
                .content_type
                .as_deref()
                .is_some_and(|ct| ct.contains("pdf"))
    });
    let has_notes =
        children.iter().any(|child| child.data.item_type == ItemType::Note);

    ItemCoverageFlags {
        has_pdf,
        has_doi,
        has_notes,
    }
}

fn classify_coverage_page(
    selected: &[ZoteroItem],
    children_by_idx: &[Vec<ZoteroItem>],
    pagination: PaginationInfo,
) -> LibraryCoveragePage {
    let mut flags = Vec::with_capacity(selected.len());
    for (item, children) in selected.iter().zip(children_by_idx) {
        flags.push(coverage_flags(item, children));
    }
    LibraryCoveragePage {
        coverage: classify_coverage(&flags),
        pagination,
    }
}

/// Aggregates coverage flags across library items into [`LibraryCoverage`].
fn classify_coverage(flags: &[ItemCoverageFlags]) -> LibraryCoverage {
    let total = flags.len();
    if total == 0 {
        return LibraryCoverage {
            total_items: 0,
            with_pdf: 0,
            with_doi: 0,
            with_notes: 0,
            pdf_percentage: 0.0,
            doi_percentage: 0.0,
            notes_percentage: 0.0,
        };
    }

    let with_pdf = flags.iter().filter(|f| f.has_pdf).count();
    let with_doi = flags.iter().filter(|f| f.has_doi).count();
    let with_notes = flags.iter().filter(|f| f.has_notes).count();

    LibraryCoverage {
        total_items: total,
        with_pdf,
        with_doi,
        with_notes,
        pdf_percentage: compute_percentage(with_pdf, total),
        doi_percentage: compute_percentage(with_doi, total),
        notes_percentage: compute_percentage(with_notes, total),
    }
}

#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "percentages calculation requires float conversion"
)]
fn compute_percentage(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (count as f64 / total as f64) * 100.0
    }
}

/// Matching criterion that caused a duplicate group.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DuplicateType {
    /// Items share the same normalized DOI.
    Doi,
    /// Items share the same normalized title.
    Title,
}

/// Items identified as potential duplicates by normalized DOI or title.
///
/// DOI and title values are trimmed and compared case-insensitively. Title
/// matches only use normalized titles longer than five characters.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DuplicateGroup {
    pub match_type: DuplicateType,
    pub match_value: String,
    pub item_keys: Vec<crate::keys::ItemKey>,
}

impl ZoteroClient {
    /// Finds potential duplicate items by DOI or title.
    ///
    /// Matching is client-side and groups items only when at least two items
    /// share a normalized value:
    ///
    /// - DOIs are trimmed and compared case-insensitively.
    /// - Titles are trimmed and compared case-insensitively.
    /// - Titles must be longer than five characters after trimming.
    /// - Groups with fewer than two items are ignored.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn find_duplicates<K: AsRef<str>>(
        &self,
        collection_key: Option<K>,
    ) -> Result<Vec<DuplicateGroup>, ZoteroApiError> {
        let items = if let Some(col) = collection_key {
            self.get_collection_items(col).await?
        } else {
            self.get_all_items().await?
        };

        Ok(find_duplicate_groups(&items))
    }
}

/// Group items by matching DOI or title to identify potential duplicate items.
fn find_duplicate_groups(items: &[ZoteroItem]) -> Vec<DuplicateGroup> {
    let mut doi_map: std::collections::BTreeMap<String, Vec<&ZoteroItem>> =
        std::collections::BTreeMap::new();
    let mut title_map: std::collections::BTreeMap<String, Vec<&ZoteroItem>> =
        std::collections::BTreeMap::new();

    for item in items {
        if let Some(doi) = item.data.doi.as_deref() {
            if !doi.trim().is_empty() {
                doi_map
                    .entry(doi.trim().to_lowercase())
                    .or_default()
                    .push(item);
            }
        }
        if let Some(ref title) = item.data.title {
            let t = title.trim().to_lowercase();
            if t.len() > 5 {
                title_map.entry(t).or_default().push(item);
            }
        }
    }

    let mut duplicates = Vec::new();
    for (doi, grouped) in doi_map {
        if grouped.len() > 1 {
            duplicates.push(DuplicateGroup {
                match_type: DuplicateType::Doi,
                match_value: doi,
                item_keys: grouped.iter().map(|i| i.key.clone()).collect(),
            });
        }
    }
    for (title, grouped) in title_map {
        if grouped.len() > 1 {
            duplicates.push(DuplicateGroup {
                match_type: DuplicateType::Title,
                match_value: title,
                item_keys: grouped.iter().map(|i| i.key.clone()).collect(),
            });
        }
    }

    duplicates
}
