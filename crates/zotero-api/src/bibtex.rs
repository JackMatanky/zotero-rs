//! Naive `BibTeX` and `BibLaTeX` exporter for Zotero items.

use std::fmt::Write as _;

use crate::{
    objects::{ZoteroCreator, ZoteroItem},
    types::ItemType,
};

/// Maps a Zotero [`ItemType`] to a standard `BibTeX` or `BibLaTeX` entry type
/// string.
fn bibtex_entry_type(item_type: &ItemType, is_biblatex: bool) -> &'static str {
    match item_type {
        ItemType::JournalArticle => "article",
        ItemType::Book => "book",
        ItemType::BookSection => "incollection",
        ItemType::ConferencePaper => "inproceedings",
        ItemType::Report => "techreport",
        ItemType::Thesis => "phdthesis",
        ItemType::Webpage | ItemType::BlogPost | ItemType::ForumPost => {
            if is_biblatex {
                "online"
            } else {
                "misc"
            }
        }
        ItemType::Preprint => {
            if is_biblatex {
                "online"
            } else {
                "unpublished"
            }
        }
        _ => "misc",
    }
}

fn escape_bibtex(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("\\&"),
            '%' => out.push_str("\\%"),
            '$' => out.push_str("\\$"),
            '#' => out.push_str("\\#"),
            '_' => out.push_str("\\_"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\textasciicircum{}"),
            '\\' => out.push_str("\\backslash{}"),
            _ => out.push(c),
        }
    }
    out
}

fn format_creators(creators: &[ZoteroCreator]) -> String {
    let mut names = Vec::new();
    for creator in creators {
        if let Some(name) = &creator.name {
            names.push(name.clone());
            continue;
        }
        let last = creator.last_name.as_deref().unwrap_or_default();
        let first = creator.first_name.as_deref().unwrap_or_default();
        let combined = match (last.is_empty(), first.is_empty()) {
            (false, false) => Some(format!("{last}, {first}")),
            (false, true) => Some(last.to_owned()),
            (true, false) => Some(first.to_owned()),
            (true, true) => None,
        };
        if let Some(name) = combined {
            names.push(name);
        }
    }
    names.join(" and ")
}

/// Serializes a single [`ZoteroItem`] into a formatted `BibTeX` or `BibLaTeX`
/// string entry.
#[must_use]
#[inline]
#[expect(
    clippy::too_many_lines,
    reason = "formats full BibTeX entry fields across all types"
)]
pub fn item_to_bibtex(item: &ZoteroItem, citekey: &str, style: &str) -> String {
    let is_biblatex = style.eq_ignore_ascii_case("biblatex");
    let entry_type = bibtex_entry_type(&item.data.item_type, is_biblatex);

    let key = if citekey.trim().is_empty() {
        if let Some(citation_key) = item.data.citation_key.as_deref() {
            citation_key
        } else {
            item.data.key.as_str()
        }
    } else {
        citekey
    };

    let mut entry = format!("@{entry_type}{{{key},\n");

    if let Some(title) = &item.data.title {
        let _ = writeln!(entry, "  title = {{{}}},", escape_bibtex(title));
    }

    let authors = format_creators(&item.data.creators);
    if !authors.is_empty() {
        let _ = writeln!(entry, "  author = {{{}}},", escape_bibtex(&authors));
    }

    match item.data.item_type {
        ItemType::JournalArticle => {
            if let Some(journal) = item.data.publication_title.as_deref() {
                let _ = writeln!(
                    entry,
                    "  journal = {{{}}},",
                    escape_bibtex(journal)
                );
            }
        }
        ItemType::BookSection | ItemType::ConferencePaper => {
            if let Some(booktitle) = item.data.publication_title.as_deref() {
                let _ = writeln!(
                    entry,
                    "  booktitle = {{{}}},",
                    escape_bibtex(booktitle)
                );
            }
            if let Some(publisher) = item.data.publisher.as_deref() {
                let _ = writeln!(
                    entry,
                    "  publisher = {{{}}},",
                    escape_bibtex(publisher)
                );
            }
        }
        ItemType::Book => {
            if let Some(publisher) = item.data.publisher.as_deref() {
                let _ = writeln!(
                    entry,
                    "  publisher = {{{}}},",
                    escape_bibtex(publisher)
                );
            }
        }
        ItemType::Report => {
            if let Some(inst) = item.data.institution.as_deref() {
                let _ = writeln!(
                    entry,
                    "  institution = {{{}}},",
                    escape_bibtex(inst)
                );
            }
        }
        ItemType::Webpage
        | ItemType::BlogPost
        | ItemType::ForumPost
        | ItemType::Preprint => {
            if let Some(site) = item.data.publication_title.as_deref() {
                if is_biblatex {
                    let _ = writeln!(
                        entry,
                        "  organization = {{{}}},",
                        escape_bibtex(site)
                    );
                } else {
                    let _ = writeln!(
                        entry,
                        "  howpublished = {{{}}},",
                        escape_bibtex(site)
                    );
                }
            }
        }
        _ => {
            if let Some(pub_title) = item.data.publication_title.as_deref() {
                let _ = writeln!(
                    entry,
                    "  journal = {{{}}},",
                    escape_bibtex(pub_title)
                );
            }
        }
    }
    if let Some(date) = item.data.date.as_deref() {
        let year = date.chars().take(4).collect::<String>();
        if year.chars().all(|c| c.is_ascii_digit()) && year.len() == 4 {
            let _ = writeln!(entry, "  year = {{{year}}},");
        } else {
            let _ = writeln!(entry, "  year = {{{}}},", escape_bibtex(date));
        }
    }

    if let Some(vol) = item.data.volume.as_deref() {
        let _ = writeln!(entry, "  volume = {{{}}},", escape_bibtex(vol));
    }

    if let Some(issue) = item.data.issue.as_deref() {
        let _ = writeln!(entry, "  number = {{{}}},", escape_bibtex(issue));
    }

    if let Some(pages) = item.data.pages.as_deref() {
        let _ = writeln!(entry, "  pages = {{{}}},", escape_bibtex(pages));
    }

    if let Some(doi) = item.data.doi.as_deref() {
        let _ = writeln!(entry, "  doi = {{{}}},", escape_bibtex(doi));
    }

    if let Some(url) = item.data.url.as_deref() {
        let _ = writeln!(entry, "  url = {{{}}},", escape_bibtex(url));
    }

    if let Some(isbn) = item.data.isbn.as_deref() {
        let _ = writeln!(entry, "  isbn = {{{}}},", escape_bibtex(isbn));
    }

    entry.push('}');
    entry
}

/// Formats multiple Zotero items into a single combined `BibTeX` string.
#[must_use]
#[inline]
pub fn items_to_bibtex(items: &[ZoteroItem], style: &str) -> String {
    let mut out = String::new();
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            out.push_str("\n\n");
        }
        out.push_str(&item_to_bibtex(item, "", style));
    }
    out
}
