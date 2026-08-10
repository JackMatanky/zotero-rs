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

/// Aggregate PDF, DOI, and note coverage statistics for a library or
/// collection.
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

/// One page of library coverage results alongside pagination metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibraryCoveragePage {
    pub coverage: LibraryCoverage,
    pub pagination: PaginationInfo,
}

impl ZoteroClient {
    /// Computes library or optional `collection_key` coverage statistics for
    /// PDF attachments, DOIs, and child notes.
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
        let pagination =
            coverage_pagination(offset, limit, page.items.len(), page.total);

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

fn coverage_pagination(
    offset: usize,
    limit: usize,
    returned: usize,
    server_total: Option<usize>,
) -> PaginationInfo {
    let total = server_total.unwrap_or_else(|| offset.saturating_add(returned));
    let page_offset =
        server_total.map_or(offset, |known_total| offset.min(known_total));
    PaginationInfo {
        limit,
        offset: page_offset,
        total,
        has_more: server_total.map_or(returned == limit, |known_total| {
            page_offset.saturating_add(returned) < known_total
        }),
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

/// Type of duplication criterion matched.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DuplicateType {
    Doi,
    Title,
}

/// Group of items identified as potential duplicates.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DuplicateGroup {
    pub match_type: DuplicateType,
    pub match_value: String,
    pub item_keys: Vec<crate::keys::ItemKey>,
}

impl ZoteroClient {
    /// Scans the library or optional `collection_key` for potential duplicate
    /// items matching by title or DOI.
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
