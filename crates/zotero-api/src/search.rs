//! Search and query operations for the Zotero Local HTTP API.
//!
//! Provides type-safe URL parameter builders ([`ItemQueryParams`]) and
//! [`ZoteroClient`] methods for free-text search, tag queries, and citation key
//! lookup.

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
    objects::ZoteroItem,
    types::ItemType,
};

/// Quick search modes supported by Zotero Local API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuickSearchMode {
    /// Search title, creator, and year fields.
    TitleCreatorYear,
    /// Search all fields including fulltext.
    Everything,
}

/// Sort direction for query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

/// Type-safe builder for Zotero Local API `/items` query parameters.
#[derive(Debug, Clone, Default, bon::Builder)]
#[builder(on(String, into))]
pub struct ItemQueryParams {
    /// Free-text search query string (`q`).
    pub q: Option<String>,
    /// Quick search mode (`qmode`).
    pub qmode: Option<QuickSearchMode>,
    /// Filter by item type (`itemType`).
    pub item_type: Option<ItemType>,
    /// Filter by tag (`tag`).
    pub tag: Option<String>,
    /// Sort field (`sort`).
    pub sort: Option<String>,
    /// Sort direction (`direction`).
    pub direction: Option<SortDirection>,
    /// Page result limit (`limit`).
    pub limit: Option<usize>,
    /// 0-based page start index (`start`).
    pub start: Option<usize>,
    /// Whether to include trashed items (`includeTrashed`).
    pub include_trashed: Option<bool>,
}

/// Searchable item field in structured searches.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchField {
    Title,
    Creator,
    Date,
    Year,
    ItemType,
    Tag,
    Extra,
    Doi,
    #[serde(untagged)]
    Other(String),
}

/// Comparison operator in structured searches.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchOperator {
    #[default]
    Contains,
    Is,
    StartsWith,
    EndsWith,
    IsNot,
    DoesNotContain,
    IsGreaterThan,
    IsLessThan,
    IsBefore,
    IsAfter,
    #[serde(untagged)]
    Other(String),
}

/// Structured search condition matching a specific item field.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchCondition {
    pub field: SearchField,
    #[serde(default)]
    pub operator: SearchOperator,
    pub value: String,
}

/// Logical combination mode for multiple search conditions.
#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize,
)]
#[serde(rename_all = "camelCase")]
pub enum JoinMode {
    #[default]
    All,
    Any,
}

/// Item field used to order search results.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    DateAdded,
    DateModified,
    Title,
    Date,
    Creator,
}

/// Direction for ordering search results.
#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize,
)]
#[serde(rename_all = "camelCase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Pagination metadata returned alongside search result pages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub limit: usize,
    pub offset: usize,
    pub total: usize,
    pub has_more: bool,
}

/// Paginated result container wrapping items and pagination metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchPage<T> {
    pub items: Vec<T>,
    pub pagination: PaginationInfo,
}

impl ZoteroClient {
    /// Queries the `/items` endpoint using a structured [`ItemQueryParams`].
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    #[inline]
    pub async fn query_items(
        &self,
        params: &ItemQueryParams,
    ) -> Result<SearchPage<ZoteroItem>, ZoteroApiError> {
        let mut req = self.get("/items");
        if let Some(q) = &params.q {
            req = req.query("q", q);
        }
        if let Some(qmode) = params.qmode {
            let mode_str = match qmode {
                QuickSearchMode::TitleCreatorYear => "titleCreatorYear",
                QuickSearchMode::Everything => "everything",
            };
            req = req.query("qmode", mode_str);
        }
        if let Some(item_type) = &params.item_type {
            req = req.query("itemType", item_type.as_str());
        }
        if let Some(tag) = &params.tag {
            req = req.query("tag", tag);
        }
        if let Some(sort) = &params.sort {
            req = req.query("sort", sort);
        }
        if let Some(dir) = params.direction {
            let dir_str = match dir {
                SortDirection::Asc => "asc",
                SortDirection::Desc => "desc",
            };
            req = req.query("direction", dir_str);
        }
        if let Some(start) = params.start {
            req = req.query("start", start.to_string());
        }
        if let Some(limit) = params.limit {
            req = req.query("limit", limit.to_string());
        }
        if let Some(true) = params.include_trashed {
            req = req.query("includeTrashed", "1");
        }

        let res: ZoteroResponse<Vec<ZoteroItem>> = req.send().await?;
        let offset = params.start.unwrap_or(0);
        let limit = params.limit.unwrap_or(res.data.len());
        let total = res.total_results.unwrap_or(res.data.len());
        let has_more = offset.saturating_add(limit) < total;

        Ok(SearchPage {
            items: res.data,
            pagination: PaginationInfo {
                limit,
                offset,
                total,
                has_more,
            },
        })
    }

    /// Searches items matching `query`, returning a paginated page.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    pub async fn search_items<K: AsRef<str>>(
        &self,
        query: &str,
        collection_key: Option<K>,
        offset: usize,
        limit: usize,
    ) -> Result<SearchPage<ZoteroItem>, ZoteroApiError> {
        let path = match collection_key {
            Some(col) => format!("/collections/{}/items", col.as_ref()),
            None => "/items".to_owned(),
        };
        let req = self
            .get(&path)
            .query("q", query)
            .query("start", offset.to_string())
            .query("limit", limit.to_string())
            .query("itemType", "-note");

        let res: ZoteroResponse<Vec<ZoteroItem>> = req.send().await?;
        let total = res.total_results.unwrap_or(res.data.len());
        let has_more = offset.saturating_add(limit) < total;

        Ok(SearchPage {
            items: res.data,
            pagination: PaginationInfo {
                limit,
                offset,
                total,
                has_more,
            },
        })
    }

    /// Searches items tagged with `tag`.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    pub async fn search_by_tag<K: AsRef<str>>(
        &self,
        tag: K,
        limit: usize,
    ) -> Result<Vec<ZoteroItem>, ZoteroApiError> {
        let res: ZoteroResponse<Vec<ZoteroItem>> = self
            .get("/items")
            .query("tag", tag.as_ref())
            .query("limit", limit.to_string())
            .query("itemType", "-note")
            .send()
            .await?;
        Ok(res.data)
    }

    /// Searches items by native or `extra` field citation key.
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the underlying search request fails.
    pub async fn search_by_citation_key<K: AsRef<str>>(
        &self,
        citekey: K,
    ) -> Result<Option<ZoteroItem>, ZoteroApiError> {
        let key = citekey.as_ref();
        let page = self.search_items(key, None::<&str>, 0, 20).await?;
        let citekey_lc = key.to_lowercase();
        for item in page.items {
            if let Some(native) = item.data.citation_key.as_deref() {
                if native.to_lowercase() == citekey_lc {
                    return Ok(Some(item));
                }
                continue;
            }
            if let Some(extra) = item.data.extra.as_deref() {
                let extra_lc = extra.to_lowercase();
                if extra_lc.contains(&format!("citation key: {citekey_lc}"))
                    || extra_lc.contains(&format!("citationkey: {citekey_lc}"))
                    || extra_lc.contains(&citekey_lc)
                {
                    return Ok(Some(item));
                }
            }
        }
        Ok(None)
    }
}

impl ZoteroClient {
    /// Executes an advanced multi-condition structured search over item fields.
    ///
    /// If `join_mode` is [`JoinMode::All`], `sort` is unset, and all conditions
    /// can be pushed down to Zotero quick-search parameters, the search is
    /// executed server-side. Otherwise the library is fetched and filtered
    /// client-side.
    #[expect(
        clippy::too_many_arguments,
        reason = "six orthogonal search parameters; a params struct adds \
                  indirection without removing them"
    )]
    #[inline]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
    pub async fn advanced_search(
        &self,
        conditions: Vec<SearchCondition>,
        join_mode: JoinMode,
        sort: Option<SortField>,
        sort_direction: SortOrder,
        offset: usize,
        limit: usize,
    ) -> Result<SearchPage<ZoteroItem>, ZoteroApiError> {
        if join_mode == JoinMode::All && sort.is_none() {
            if let Some(url) = self.pushdown_url(&conditions) {
                let full_url = format!("{url}&start={offset}&limit={limit}");
                let page = self.get_items_with_total(&full_url).await?;
                return Ok(finish_page(page.items, page.total, offset, limit));
            }
        }

        let items = self.get_all_items().await?;
        let prepared: Vec<_> =
            conditions.iter().map(PreparedCondition::from).collect();
        if let Some(field) = sort {
            let matches: Vec<ZoteroItem> = items
                .into_iter()
                .filter(|item| {
                    is_searchable_item(item)
                        && item_matches_conditions(item, &prepared, join_mode)
                })
                .collect();
            return Ok(paginate(
                sort_items(matches, field, sort_direction),
                offset,
                limit,
            ));
        }

        let mut page = PageAccumulator::new(offset, limit);
        for item in items {
            if is_searchable_item(&item)
                && item_matches_conditions(&item, &prepared, join_mode)
            {
                page.push_match(item);
            }
        }
        Ok(page.into_page())
    }

    /// Builds a server-search URL for `conditions` when they are fully
    /// expressible as Zotero quick-search parameters, or `None` to fall back
    /// to a client-side scan.
    fn pushdown_url(&self, conditions: &[SearchCondition]) -> Option<String> {
        if conditions.is_empty() {
            return None;
        }
        let mut q: Option<&str> = None;
        let mut qmode = "titleCreatorYear";
        let mut item_type: Option<&str> = None;
        let mut tag: Option<&str> = None;

        for cond in conditions {
            let value = cond.value.as_str();
            let operator_pushable = matches!(
                cond.operator,
                SearchOperator::Contains
                    | SearchOperator::Is
                    | SearchOperator::StartsWith
            );
            if !operator_pushable {
                return None;
            }
            match &cond.field {
                SearchField::Creator
                | SearchField::Year
                | SearchField::Date => {
                    if q.is_some() {
                        return None;
                    }
                    q = Some(value);
                    qmode = if cond.field == SearchField::Creator {
                        "creator"
                    } else {
                        "year"
                    };
                }
                SearchField::ItemType
                    if cond.operator == SearchOperator::Is =>
                {
                    if item_type.is_some() {
                        return None;
                    }
                    item_type = Some(value);
                }
                SearchField::Tag if cond.operator == SearchOperator::Is => {
                    if tag.is_some() {
                        return None;
                    }
                    tag = Some(value);
                }
                _ => return None,
            }
        }

        let mut url = format!(
            "{}{}/items",
            self.base_url.trim_end_matches('/'),
            self.target_prefix()
        );
        let mut params = Vec::new();
        if let Some(q) = q {
            params.push(format!("q={}", urlencoding::encode(q)));
            params.push(format!("qmode={qmode}"));
        }
        if let Some(item_type) = item_type {
            params.push(format!(
                "itemType={item_type},-note,-attachment,-annotation"
            ));
        } else {
            params.push("itemType=-note,-attachment,-annotation".to_owned());
        }
        if let Some(tag) = tag {
            params.push(format!("tag={}", urlencoding::encode(tag)));
        }
        url.push('?');
        url.push_str(&params.join("&"));
        Some(url)
    }
}

/// Search condition prepared once for client-side scans.
struct PreparedCondition<'a> {
    field: &'a SearchField,
    operator: &'a SearchOperator,
    value: &'a str,
    value_lc: String,
}

impl<'a> From<&'a SearchCondition> for PreparedCondition<'a> {
    fn from(cond: &'a SearchCondition) -> Self {
        Self {
            field: &cond.field,
            operator: &cond.operator,
            value: cond.value.as_str(),
            value_lc: cond.value.to_lowercase(),
        }
    }
}

impl PreparedCondition<'_> {
    fn matches_str(&self, s: &str) -> bool {
        match self.operator {
            SearchOperator::Is => s.to_lowercase() == self.value_lc,
            SearchOperator::IsNot => s.to_lowercase() != self.value_lc,
            SearchOperator::StartsWith => {
                s.to_lowercase().starts_with(&self.value_lc)
            }
            SearchOperator::EndsWith => {
                s.to_lowercase().ends_with(&self.value_lc)
            }
            SearchOperator::DoesNotContain => {
                !s.to_lowercase().contains(&self.value_lc)
            }
            SearchOperator::Contains | SearchOperator::Other(_) => {
                s.to_lowercase().contains(&self.value_lc)
            }
            SearchOperator::IsGreaterThan | SearchOperator::IsAfter => {
                compare_dates(s, self.value).is_gt()
            }
            SearchOperator::IsLessThan | SearchOperator::IsBefore => {
                compare_dates(s, self.value).is_lt()
            }
        }
    }

    fn matches_item(&self, item: &ZoteroItem) -> bool {
        match self.field {
            SearchField::Title => {
                item.data.title.as_deref().is_some_and(|s| self.matches_str(s))
            }
            SearchField::Creator => item.data.creators.iter().any(|c| {
                c.name.as_deref().is_some_and(|s| self.matches_str(s))
                    || c.first_name
                        .as_deref()
                        .is_some_and(|s| self.matches_str(s))
                    || c.last_name
                        .as_deref()
                        .is_some_and(|s| self.matches_str(s))
                    || matches_creator_full_name(c, self)
            }),
            SearchField::Date => {
                item.data.date.as_deref().is_some_and(|s| self.matches_str(s))
            }
            SearchField::Year => item.data.date.as_deref().is_some_and(|d| {
                self.matches_str(d.split('-').next().unwrap_or(d))
            }),
            SearchField::ItemType => {
                self.matches_str(item.data.item_type.as_str())
            }
            SearchField::Tag => {
                item.data.tags.iter().any(|t| self.matches_str(t.tag.as_str()))
            }
            SearchField::Extra => {
                item.data.extra.as_deref().is_some_and(|s| self.matches_str(s))
            }
            SearchField::Doi => {
                item.data.doi.as_deref().is_some_and(|s| self.matches_str(s))
            }
            SearchField::Other(field_name) => match field_name.as_str() {
                "title" => item
                    .data
                    .title
                    .as_deref()
                    .is_some_and(|s| self.matches_str(s)),
                "doi" => item
                    .data
                    .doi
                    .as_deref()
                    .is_some_and(|s| self.matches_str(s)),
                _ => false,
            },
        }
    }
}

/// Accumulates only the requested page while still counting all matches.
struct PageAccumulator<T> {
    offset: usize,
    limit: usize,
    total: usize,
    items: Vec<T>,
}

impl<T> PageAccumulator<T> {
    fn new(offset: usize, limit: usize) -> Self {
        Self {
            offset,
            limit,
            total: 0,
            items: Vec::with_capacity(limit),
        }
    }

    fn push_match(&mut self, item: T) {
        if self.total >= self.offset && self.items.len() < self.limit {
            self.items.push(item);
        }
        self.total = self.total.saturating_add(1);
    }

    fn into_page(self) -> SearchPage<T> {
        let offset = self.offset.min(self.total);
        let returned = self.items.len();
        SearchPage {
            items: self.items,
            pagination: PaginationInfo {
                limit: self.limit,
                offset,
                total: self.total,
                has_more: offset.saturating_add(returned) < self.total,
            },
        }
    }
}

/// Returns a `{items, pagination}` page slicing `results` at `offset`/`limit`.
fn paginate<T>(results: Vec<T>, offset: usize, limit: usize) -> SearchPage<T> {
    let total = results.len();
    let skip = offset.min(total);
    let items: Vec<T> = results.into_iter().skip(skip).take(limit).collect();
    SearchPage {
        items,
        pagination: PaginationInfo {
            limit,
            offset: skip,
            total,
            has_more: skip.saturating_add(limit) < total,
        },
    }
}

/// Wraps a server-fetched page, falling back to `offset + items.len()` when
/// the server reports no total.
fn finish_page(
    items: Vec<ZoteroItem>,
    server_total: Option<usize>,
    offset: usize,
    limit: usize,
) -> SearchPage<ZoteroItem> {
    let returned = items.len();
    let total = server_total.unwrap_or_else(|| offset.saturating_add(returned));
    let has_more = server_total.map_or(returned == limit, |exact| {
        offset.saturating_add(returned) < exact
    });

    SearchPage {
        items,
        pagination: PaginationInfo {
            limit,
            offset,
            total,
            has_more,
        },
    }
}

/// Returns true for items that are not attachments, notes, or annotations.
fn is_searchable_item(item: &ZoteroItem) -> bool {
    item.data.item_type.is_indexable()
}

fn matches_creator_full_name(
    creator: &crate::objects::ZoteroCreator,
    cond: &PreparedCondition<'_>,
) -> bool {
    let (Some(first), Some(last)) =
        (creator.first_name.as_deref(), creator.last_name.as_deref())
    else {
        return false;
    };
    let mut full = String::with_capacity(
        first.len().saturating_add(1).saturating_add(last.len()),
    );
    full.push_str(first);
    full.push(' ');
    full.push_str(last);
    cond.matches_str(&full)
}

fn item_matches_conditions(
    item: &ZoteroItem,
    conditions: &[PreparedCondition<'_>],
    join_mode: JoinMode,
) -> bool {
    match join_mode {
        JoinMode::All => conditions.iter().all(|cond| cond.matches_item(item)),
        JoinMode::Any => conditions.iter().any(|cond| cond.matches_item(item)),
    }
}

/// Compares two date-or-year strings by their leading numeric components.
fn compare_dates(a: &str, b: &str) -> std::cmp::Ordering {
    date_key(a).cmp(&date_key(b))
}

fn date_key(s: &str) -> (u32, u32, u32) {
    let mut parts = s.split('-').filter(|p| !p.is_empty());
    (
        next_date_part(&mut parts),
        next_date_part(&mut parts),
        next_date_part(&mut parts),
    )
}

fn next_date_part<'a>(parts: &mut impl Iterator<Item = &'a str>) -> u32 {
    parts.next().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0)
}

/// Sorts `items` by `field` in `direction` and returns the sorted items.
fn sort_items(
    items: Vec<ZoteroItem>,
    field: SortField,
    direction: SortOrder,
) -> Vec<ZoteroItem> {
    let mut keyed: Vec<(String, ZoteroItem)> = items
        .into_iter()
        .map(|item| {
            let key = sort_key(&item, field);
            (key, item)
        })
        .collect();
    match direction {
        SortOrder::Asc => keyed.sort_by(|a, b| a.0.cmp(&b.0)),
        SortOrder::Desc => keyed.sort_by(|a, b| b.0.cmp(&a.0)),
    }
    keyed.into_iter().map(|(_, item)| item).collect()
}

/// Returns the sort key string for `item` under `field`.
fn sort_key(item: &ZoteroItem, field: SortField) -> String {
    match field {
        SortField::Title => item.data.title.clone().unwrap_or_default(),
        SortField::Date => item.data.date.clone().unwrap_or_default(),
        SortField::DateAdded => {
            item.data.date_added.clone().unwrap_or_default()
        }
        SortField::DateModified => {
            item.data.date_modified.clone().unwrap_or_default()
        }
        SortField::Creator => {
            item.data.creators.first().map_or_else(String::new, |c| {
                c.name.clone().unwrap_or_else(|| {
                    format!(
                        "{} {}",
                        c.first_name.as_deref().unwrap_or(""),
                        c.last_name.as_deref().unwrap_or("")
                    )
                })
            })
        }
    }
}

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
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if the request fails.
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
    /// Returns [`ZoteroApiError::LocalApi`]/[`ZoteroApiError::Network`]/
    /// [`ZoteroApiError::Json`] if fetching library items fails.
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::client::test_http::{MockServer, http_response_with_headers};

    #[tokio::test]
    async fn search_items_parses_total_and_pagination() {
        let items = r#"[{"key":"ITEM1","version":1,"data":{"key":"ITEM1","version":1,"itemType":"journalArticle","title":"Test"}},{"key":"ITEM2","version":1,"data":{"key":"ITEM2","version":1,"itemType":"journalArticle","title":"Test 2"}}]"#;
        let server = MockServer::new(vec![http_response_with_headers(
            "200 OK",
            &[("Total-Results", "50")],
            items,
        )]);
        let client = ZoteroClient::new(server.url());

        let page =
            client.search_items("test", None::<&str>, 0, 2).await.unwrap();

        assert_eq!(page.items.len(), 2);
        assert_eq!(page.pagination.total, 50);
        assert_eq!(page.pagination.limit, 2);
        assert_eq!(page.pagination.offset, 0);
        assert!(page.pagination.has_more);
    }
}
