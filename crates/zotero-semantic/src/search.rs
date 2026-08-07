//! Query embedding and cosine similarity search scoring.

use std::{collections::HashMap, sync::Arc};

use serde::Serialize;
use zotero_api::{ItemKey, ZoteroApiError};

use crate::{EmbeddingProvider, store::StoredChunk};

/// Represents a semantic search match for a single item in the library.
#[derive(Clone, Debug, Serialize)]
pub struct SemanticSearchHit {
    /// Unique Zotero item key.
    pub item_key: ItemKey,
    /// Item title, if available.
    pub title: Option<String>,
    /// Cosine similarity score between the query and this chunk.
    pub similarity: f32,
    /// Zero-based index of the chunk within the item's text.
    pub chunk_index: i64,
    /// Text content of the matching chunk.
    pub chunk_text: String,
}

/// Embeds a search query and returns ranked search hits across all stored
/// chunks.
#[inline]
/// # Errors
///
/// Returns [`ZoteroApiError::Embedding`] if query embedding fails or task
/// execution fails.
pub async fn search_library(
    provider: &Arc<dyn EmbeddingProvider>,
    all_chunks: &[StoredChunk],
    query: &str,
    limit: usize,
    min_similarity: f32,
) -> Result<Vec<SemanticSearchHit>, ZoteroApiError> {
    let provider = Arc::clone(provider);
    let query_owned = query.to_owned();
    let mut query_embedding = tokio::task::spawn_blocking(move || {
        provider.embed(&[query_owned]).and_then(|mut v| {
            v.pop().ok_or_else(|| {
                ZoteroApiError::Embedding(
                    "embedding provider returned no vector for the query"
                        .to_owned(),
                )
            })
        })
    })
    .await
    .map_err(|e| ZoteroApiError::Embedding(e.to_string()))??;
    query_embedding.normalize();

    let mut best_per_item: HashMap<&ItemKey, SemanticSearchHit> =
        HashMap::new();
    for chunk in all_chunks {
        let score = query_embedding.dot(&chunk.embedding);
        if score < min_similarity {
            continue;
        }
        let entry = best_per_item.get(&chunk.item_key);
        if entry.is_none_or(|existing| score > existing.similarity) {
            best_per_item.insert(&chunk.item_key, SemanticSearchHit {
                item_key: chunk.item_key.clone(),
                title: chunk.title.clone(),
                similarity: score,
                chunk_index: chunk.chunk_index,
                chunk_text: chunk.chunk_text.clone(),
            });
        }
    }
    let mut hits: Vec<SemanticSearchHit> =
        best_per_item.into_values().collect();
    hits.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
    hits.truncate(limit);
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::Embedding;

    #[derive(Debug)]
    struct FixedProvider {
        vector: Vec<f32>,
    }

    impl EmbeddingProvider for FixedProvider {
        fn embed(
            &self,
            texts: &[String],
        ) -> Result<Vec<Embedding>, ZoteroApiError> {
            Ok(texts
                .iter()
                .map(|_| Embedding::from(self.vector.clone()))
                .collect())
        }
    }

    fn stored(
        item_key: &str,
        chunk_index: i64,
        embedding: Vec<f32>,
    ) -> StoredChunk {
        StoredChunk {
            item_key: ItemKey::from(item_key),
            title: Some(format!("Title {item_key}")),
            chunk_index,
            chunk_text: format!("chunk {chunk_index} of {item_key}"),
            embedding: Embedding::from(embedding),
        }
    }

    #[tokio::test]
    async fn returns_best_chunk_per_item_above_min_similarity() {
        let provider: Arc<dyn EmbeddingProvider> = Arc::new(FixedProvider {
            vector: vec![1.0, 0.0],
        });
        let chunks = vec![
            stored("ITEM1", 0, vec![1.0, 0.0]),
            stored("ITEM1", 1, vec![0.0, 1.0]),
            stored("ITEM2", 0, vec![0.0, 1.0]),
        ];

        let hits =
            search_library(&provider, &chunks, "query", 10, 0.3).await.unwrap();

        assert_eq!(hits.len(), 1);
        let hit = hits.first().unwrap();
        assert_eq!(hit.item_key.as_str(), "ITEM1");
        assert_eq!(hit.chunk_index, 0);
        assert!((hit.similarity - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn filters_hits_below_min_similarity() {
        let provider: Arc<dyn EmbeddingProvider> = Arc::new(FixedProvider {
            vector: vec![1.0, 0.0],
        });
        let chunks = vec![stored("ITEM1", 0, vec![0.0, 1.0])];

        let hits =
            search_library(&provider, &chunks, "query", 10, 0.3).await.unwrap();

        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn truncates_to_limit_sorted_by_descending_similarity() {
        let provider: Arc<dyn EmbeddingProvider> = Arc::new(FixedProvider {
            vector: vec![1.0, 0.0],
        });
        let chunks = vec![
            stored("ITEM1", 0, vec![0.5, 0.866_025_4]),
            stored("ITEM2", 0, vec![1.0, 0.0]),
            stored("ITEM3", 0, vec![
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ]),
        ];

        let hits =
            search_library(&provider, &chunks, "query", 2, 0.0).await.unwrap();

        assert_eq!(hits.len(), 2);
        assert_eq!(hits.first().unwrap().item_key.as_str(), "ITEM2");
        assert_eq!(hits.get(1).unwrap().item_key.as_str(), "ITEM3");
    }
}
