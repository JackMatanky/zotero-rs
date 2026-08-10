//! Tag operations for the Zotero Local HTTP API.

use std::collections::BTreeSet;

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
    keys::TagName,
    objects::{ZoteroItem, ZoteroTag},
};

impl ZoteroClient {
    /// Lists tag names present in the library, returning up to `limit` entries
    /// as [`TagName`] wrappers.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    /// - [`Json`] if the tag array cannot be decoded
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn list_tags(
        &self,
        limit: usize,
    ) -> Result<Vec<TagName>, ZoteroApiError> {
        let res: ZoteroResponse<Vec<serde_json::Value>> =
            self.get("/tags").query("limit", limit.to_string()).send().await?;
        Ok(res
            .data
            .into_iter()
            .filter_map(|v| {
                v.get("tag").and_then(|t| t.as_str()).map(TagName::from)
            })
            .collect())
    }

    /// Adds and removes tags across multiple items in a single batch.
    ///
    /// Tags in `add_tags` are merged without duplicating existing entries. Tags
    /// in `remove_tags` are removed by exact [`TagName`] match. Returns the
    /// count of items updated.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if any item key does not exist
    /// - [`LocalApi`] if Zotero rejects the update
    /// - [`Network`] on connection failure
    ///
    /// [`NotFound`]: ZoteroApiError::NotFound
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn batch_update_tags<K: AsRef<str>>(
        &self,
        item_keys: &[K],
        add_tags: &[TagName],
        remove_tags: &[TagName],
    ) -> Result<usize, ZoteroApiError> {
        let mut count: usize = 0;
        for key in item_keys {
            let item = self.get_item(key.as_ref()).await?;
            self.apply_tag_patch(item, add_tags, remove_tags).await?;
            count = count.saturating_add(1);
        }
        Ok(count)
    }

    /// Renames a tag across up to 100 items that have it.
    ///
    /// Searches for items tagged with `old_tag`, replaces that tag with
    /// `new_tag`, and returns the number of items updated. Items beyond the
    /// 100-item search limit are not renamed.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the update
    /// - [`Network`] on connection failure
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn rename_tag<K: AsRef<str>, V: AsRef<str>>(
        &self,
        old_tag: K,
        new_tag: V,
    ) -> Result<usize, ZoteroApiError> {
        let old = old_tag.as_ref();
        let new = TagName::from(new_tag.as_ref());
        let items = self.search_by_tag(old, 100).await?;
        let old_tag_name = TagName::from(old);
        let mut count: usize = 0;
        for item in items {
            self.apply_tag_patch(
                item,
                std::slice::from_ref(&new),
                std::slice::from_ref(&old_tag_name),
            )
            .await?;
            count = count.saturating_add(1);
        }
        Ok(count)
    }

    /// Applies a tag diff to `item`'s existing tags and `PATCH`es it.
    async fn apply_tag_patch(
        &self,
        item: ZoteroItem,
        add: &[TagName],
        remove: &[TagName],
    ) -> Result<(), ZoteroApiError> {
        let new_tags = diff_tags(item.data.tags, add, remove);
        let patch =
            serde_json::json!({ "tags": new_tags, "version": item.version });
        self.update_item(item.key, patch).await?;
        Ok(())
    }

    /// Deletes tags by exact name from the entire library.
    ///
    /// The Zotero API limits this to 50 tags per request. A version guard
    /// prevents conflicting concurrent modifications.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the deletion
    /// - [`Network`] on connection failure
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn delete_tags<K: AsRef<str>>(
        &self,
        tags: &[K],
    ) -> Result<(), ZoteroApiError> {
        let version = self.get_library_version().await?;
        let joined = tags
            .iter()
            .map(|t| urlencoding::encode(t.as_ref()).into_owned())
            .collect::<Vec<_>>()
            .join(" || ");
        self.delete_req("/tags")
            .query("tag", joined)
            .unmodified_since_version(version.into())
            .send_unit()
            .await?;
        Ok(())
    }
}

/// Computes the updated tag array for an item after applying additions and
/// removals.
pub(crate) fn diff_tags(
    existing: Vec<ZoteroTag>,
    add: &[TagName],
    remove: &[TagName],
) -> Vec<serde_json::Value> {
    let mut tags_set: BTreeSet<TagName> =
        existing.into_iter().map(|t| t.tag).collect();
    tags_set.extend(add.iter().cloned());
    for r in remove {
        tags_set.remove(r);
    }
    tags_set
        .into_iter()
        .map(|t| serde_json::json!({ "tag": t.as_str() }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TagOrigin;

    mod diff_tags {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn adds_new_tags_and_removes_specified_existing_tags() {
            let existing = vec![ZoteroTag {
                tag: TagName::from("old"),
                origin: TagOrigin::default(),
            }];
            let add = vec![TagName::from("new")];
            let remove = vec![TagName::from("old")];

            let result = super::diff_tags(existing, &add, &remove);

            assert_eq!(result.len(), 1);
            assert_eq!(
                result.first().and_then(|v| v.get("tag")),
                Some(&serde_json::Value::String("new".to_owned()))
            );
        }

        #[test]
        fn handles_empty_add_and_remove_tag_lists() {
            let existing = vec![ZoteroTag {
                tag: TagName::from("keep_me"),
                origin: TagOrigin::default(),
            }];

            let result = super::diff_tags(existing, &[], &[]);

            assert_eq!(result.len(), 1);
            assert_eq!(
                result.first().and_then(|v| v.get("tag")),
                Some(&serde_json::Value::String("keep_me".to_owned()))
            );
        }

        #[test]
        fn sorts_resulting_tags_deterministically() {
            let existing = vec![ZoteroTag {
                tag: TagName::from("zeta"),
                origin: TagOrigin::default(),
            }];
            let add = vec![TagName::from("alpha"), TagName::from("middle")];

            let result = super::diff_tags(existing, &add, &[]);
            let tags: Vec<_> = result
                .iter()
                .filter_map(|value| {
                    value.get("tag").and_then(|tag| tag.as_str())
                })
                .collect();

            assert_eq!(tags, vec!["alpha", "middle", "zeta"]);
        }

        #[test]
        fn deduplicates_added_tags_when_already_present() {
            let existing = vec![ZoteroTag {
                tag: TagName::from("rust"),
                origin: TagOrigin::default(),
            }];
            let add = vec![TagName::from("rust")];

            let result = super::diff_tags(existing, &add, &[]);

            assert_eq!(result.len(), 1);
            assert_eq!(
                result.first().and_then(|v| v.get("tag")),
                Some(&serde_json::Value::String("rust".to_owned()))
            );
        }
    }
}
