//! Metadata resolution for DOI, arXiv ID, and ISBN imports.
//!
//! This module turns public identifiers into [`ItemDraft`] values that can be
//! passed to Zotero item creation APIs. Each resolver calls the external
//! metadata service for the identifier kind, extracts the fields Zotero needs,
//! and leaves unsupported or missing fields at their [`Default`] values.
//!
//! | Identifier | API                        | Default base URL                  |
//! | ---------- | -------------------------- | --------------------------------- |
//! | DOI        | Crossref Works API         | `https://api.crossref.org`        |
//! | arXiv      | Semantic Scholar Graph API | `https://api.semanticscholar.org` |
//! | ISBN       | Open Library Books API     | `https://openlibrary.org`         |
//!
//! Use [`resolve_metadata`] for normal resolution against the default services.
//! Use [`resolve_metadata_with_urls`] when tests or offline tools need local
//! service doubles.
//!
//! # Examples
//!
//! ```no_run
//! use zotero_api::{IdentifierKind, ZoteroApiError, resolve_metadata};
//!
//! # async fn run() -> Result<(), ZoteroApiError> {
//! let http = reqwest::Client::new();
//! let draft =
//!     resolve_metadata(&http, IdentifierKind::Doi, "10.1038/nphys1170")
//!         .await?;
//! assert_eq!(draft.doi, "10.1038/nphys1170");
//! # Ok(())
//! # }
//! ```

use serde::Deserialize;

use crate::{
    errors::ZoteroApiError,
    objects::{ItemDraft, ZoteroCreator},
    types::{CreatorType, ItemType},
};

const DEFAULT_CROSSREF_URL: &str = "https://api.crossref.org";
const DEFAULT_SEMANTIC_SCHOLAR_URL: &str = "https://api.semanticscholar.org";
const DEFAULT_OPEN_LIBRARY_URL: &str = "https://openlibrary.org";

/// Public identifier type accepted by metadata resolution.
///
/// Choose the variant that matches the namespace of the identifier string:
///
/// - [`Doi`] for Digital Object Identifiers, such as `10.1000/xyz123`
/// - [`Arxiv`] for arXiv identifiers, such as `2401.01234`
/// - [`Isbn`] for ISBN-10 or ISBN-13 book identifiers
///
/// [`Doi`]: IdentifierKind::Doi
/// [`Arxiv`]: IdentifierKind::Arxiv
/// [`Isbn`]: IdentifierKind::Isbn
#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierKind {
    /// Resolve a Digital Object Identifier through Crossref.
    Doi,
    /// Resolve an arXiv identifier through Semantic Scholar.
    Arxiv,
    /// Resolve an International Standard Book Number through Open Library.
    Isbn,
}

/// Resolves a public identifier with the default metadata APIs.
///
/// This is the convenience wrapper for production callers. It uses the default
/// base URLs listed in the module documentation and returns an [`ItemDraft`]
/// populated with the metadata available from the selected service.
///
/// # Examples
///
/// ```rust,no_run
/// use zotero_api::metadata::{IdentifierKind, resolve_metadata};
///
/// # async fn example() -> Result<(), zotero_api::ZoteroApiError> {
/// let http = reqwest::Client::new();
/// let draft =
///     resolve_metadata(&http, IdentifierKind::Doi, "10.1038/nphys1170")
///         .await?;
/// assert_eq!(draft.doi, "10.1038/nphys1170");
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// - [`NotFound`] if the identifier cannot be resolved
/// - [`LocalApi`] if the metadata API returns a non-success status
/// - [`Network`] if the HTTP request fails
/// - [`Json`] if the response cannot be decoded
///
/// [`NotFound`]: ZoteroApiError::NotFound
/// [`LocalApi`]: ZoteroApiError::LocalApi
/// [`Network`]: ZoteroApiError::Network
/// [`Json`]: ZoteroApiError::Json
#[inline]
pub async fn resolve_metadata(
    http: &reqwest::Client,
    kind: IdentifierKind,
    id: &str,
) -> Result<ItemDraft, ZoteroApiError> {
    resolve_metadata_with_urls(http, kind, id, None, None, None).await
}

/// Resolves a public identifier with optional metadata API base URL overrides.
///
/// Each `*_base` argument replaces the default service URL for its matching
/// identifier kind. Pass [`None`] for services that should keep their default
/// URL. This keeps tests small: a test can point only the service it exercises
/// at a local mock server.
///
/// # Errors
///
/// - [`NotFound`] if the identifier cannot be resolved
/// - [`LocalApi`] if the metadata API returns a non-success status
/// - [`Network`] if the HTTP request fails
/// - [`Json`] if the response cannot be decoded
///
/// [`NotFound`]: ZoteroApiError::NotFound
/// [`LocalApi`]: ZoteroApiError::LocalApi
/// [`Network`]: ZoteroApiError::Network
/// [`Json`]: ZoteroApiError::Json
#[expect(
    clippy::too_many_arguments,
    reason = "five optional base-url overrides for offline testing; a params \
              struct adds indirection without removing them"
)]
#[inline]
pub async fn resolve_metadata_with_urls(
    http: &reqwest::Client,
    kind: IdentifierKind,
    id: &str,
    crossref_base: Option<&str>,
    semantic_scholar_base: Option<&str>,
    open_library_base: Option<&str>,
) -> Result<ItemDraft, ZoteroApiError> {
    match kind {
        IdentifierKind::Doi => {
            let base = crossref_base.unwrap_or(DEFAULT_CROSSREF_URL);
            resolve_doi(http, base, id).await
        }
        IdentifierKind::Arxiv => {
            let base =
                semantic_scholar_base.unwrap_or(DEFAULT_SEMANTIC_SCHOLAR_URL);
            resolve_arxiv(http, base, id).await
        }
        IdentifierKind::Isbn => {
            let base = open_library_base.unwrap_or(DEFAULT_OPEN_LIBRARY_URL);
            resolve_isbn(http, base, id).await
        }
    }
}

async fn fetch_json(
    http: &reqwest::Client,
    url: &str,
) -> Result<serde_json::Value, ZoteroApiError> {
    let resp = http.get(url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(ZoteroApiError::NotFound(format!(
            "No metadata found for {url}"
        )));
    }
    if !resp.status().is_success() {
        return Err(ZoteroApiError::LocalApi {
            status: resp.status().as_u16(),
            message: resp.status().to_string(),
        });
    }
    Ok(resp.json().await?)
}

/// Walks `path` through nested JSON objects/arrays, treating each segment as
/// an array index when it parses as `usize` and an object key otherwise.
fn value_at<'a>(
    value: &'a serde_json::Value,
    path: &[&str],
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path {
        current = match segment.parse::<usize>() {
            Ok(index) => current.get(index)?,
            Err(_) => current.get(segment)?,
        };
    }
    Some(current)
}

fn str_at<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    value_at(value, path)?.as_str()
}

fn i64_at(value: &serde_json::Value, path: &[&str]) -> Option<i64> {
    value_at(value, path)?.as_i64()
}

async fn resolve_doi(
    http: &reqwest::Client,
    base_url: &str,
    doi: &str,
) -> Result<ItemDraft, ZoteroApiError> {
    let url = format!(
        "{}/works/{}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(doi)
    );
    let body = fetch_json(http, &url).await?;
    let msg = body.get("message").cloned().unwrap_or_default();
    let title = str_at(&msg, &["title", "0"]).unwrap_or_default().to_owned();
    let creators = msg
        .get("author")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|a| ZoteroCreator {
            creator_type: Some(CreatorType::Author),
            first_name: Some(
                str_at(a, &["given"]).unwrap_or_default().to_owned(),
            ),
            last_name: Some(
                str_at(a, &["family"]).unwrap_or_default().to_owned(),
            ),
            name: None,
        })
        .collect();
    let year = i64_at(&msg, &["published", "date-parts", "0", "0"])
        .or_else(|| i64_at(&msg, &["issued", "date-parts", "0", "0"]));
    Ok(ItemDraft {
        item_type: ItemType::JournalArticle,
        title,
        creators,
        date: year.map(|y| y.to_string()).unwrap_or_default(),
        doi: str_at(&msg, &["DOI"]).unwrap_or(doi).to_owned(),
        url: str_at(&msg, &["URL"]).unwrap_or_default().to_owned(),
        publication_title: str_at(&msg, &["container-title", "0"])
            .unwrap_or_default()
            .to_owned(),
        ..ItemDraft::default()
    })
}

async fn resolve_arxiv(
    http: &reqwest::Client,
    base_url: &str,
    arxiv_id: &str,
) -> Result<ItemDraft, ZoteroApiError> {
    let url = format!(
        "{}/graph/v1/paper/arXiv:{}?fields=title,authors,year,abstract,\
         externalIds,venue",
        base_url.trim_end_matches('/'),
        arxiv_id
    );
    let body = fetch_json(http, &url).await?;
    let title = str_at(&body, &["title"]).unwrap_or_default().to_owned();
    let creators = named_creators(&body);
    let doi = str_at(&body, &["externalIds", "DOI"]);
    Ok(ItemDraft {
        item_type: if doi.is_some() {
            ItemType::JournalArticle
        } else {
            ItemType::Preprint
        },
        title,
        creators,
        date: i64_at(&body, &["year"])
            .map(|y| y.to_string())
            .unwrap_or_default(),
        doi: doi.unwrap_or_default().to_owned(),
        url: format!("https://arxiv.org/abs/{arxiv_id}"),
        abstract_note: str_at(&body, &["abstract"])
            .unwrap_or_default()
            .to_owned(),
        publication_title: str_at(&body, &["venue"])
            .unwrap_or_default()
            .to_owned(),
        ..ItemDraft::default()
    })
}

async fn resolve_isbn(
    http: &reqwest::Client,
    base_url: &str,
    isbn: &str,
) -> Result<ItemDraft, ZoteroApiError> {
    let url = format!(
        "{}/api/books?bibkeys=ISBN:{}&jscmd=data&format=json",
        base_url.trim_end_matches('/'),
        isbn
    );
    let body = fetch_json(http, &url).await?;
    let key = format!("ISBN:{isbn}");
    let Some(record) = body.get(&key) else {
        return Err(ZoteroApiError::NotFound(format!(
            "No book found for ISBN {isbn}"
        )));
    };
    let title = str_at(record, &["title"]).unwrap_or_default().to_owned();
    let creators = named_creators(record);
    let publisher = str_at(record, &["publishers", "0", "name"])
        .unwrap_or_default()
        .to_owned();
    Ok(ItemDraft {
        item_type: ItemType::Book,
        title,
        creators,
        date: str_at(record, &["publish_date"]).unwrap_or_default().to_owned(),
        isbn: isbn.to_owned(),
        publisher,
        url: str_at(record, &["url"]).unwrap_or_default().to_owned(),
        ..ItemDraft::default()
    })
}

/// Extracts name-only `ZoteroCreator`s from `container`'s `authors` array.
///
/// Used by APIs that report a single display name per author rather than
/// separate given/family names (Semantic Scholar, Open Library).
fn named_creators(container: &serde_json::Value) -> Vec<ZoteroCreator> {
    container
        .get("authors")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|a| ZoteroCreator {
            creator_type: Some(CreatorType::Author),
            first_name: None,
            last_name: None,
            name: Some(str_at(a, &["name"]).unwrap_or_default().to_owned()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;
    use crate::client::test_http::{MockServer, http_response};

    #[tokio::test]
    async fn parses_crossref_response_into_item_draft() {
        let body = json!({"message": {
            "title": ["A Great Paper"],
            "author": [{"given": "Sam", "family": "McAuthor"}],
            "published": {"date-parts": [[2021]]},
            "DOI": "10.1/xyz",
            "URL": "https://doi.org/10.1/xyz",
            "container-title": ["Journal of Things"]
        }});
        let server =
            MockServer::new(vec![http_response("200 OK", &body.to_string())]);
        let http = reqwest::Client::new();

        let draft = resolve_metadata_with_urls(
            &http,
            IdentifierKind::Doi,
            "10.1/xyz",
            Some(server.url()),
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(draft.title, "A Great Paper");
        assert_eq!(draft.item_type, ItemType::JournalArticle);
        assert_eq!(
            draft.creators.first().and_then(|c| c.last_name.as_deref()),
            Some("McAuthor")
        );
        assert_eq!(draft.date, "2021");
    }
}
