//! Local semantic search: embedding generation, chunk storage, and vector
//! similarity search over Zotero item content.

pub mod chunking;
pub mod embedding;
pub mod index;
pub mod search;
pub mod store;

use std::{env, path::PathBuf};

pub use chunking::chunk_text;
pub use embedding::{Embedding, FastEmbedProvider};
pub use index::{IndexReport, index_library};
pub use search::{SemanticSearchHit, search_library};
pub use store::{NewChunk, SemanticIndex, SemanticIndexStats, StoredChunk};
pub use zotero_api::ZoteroApiError;

/// Maximum characters of assembled text (title + abstract + fulltext) indexed
/// per item.
pub const MAX_INDEXABLE_CHARS: usize = 400_000;

/// Ceiling on characters per chunk.
pub const MAX_CHUNK_CHARS: usize = 6000;

/// Minimum cosine similarity threshold required for a search hit to be
/// returned.
pub const DEFAULT_MIN_SIMILARITY: f32 = 0.3;

/// Trait boundary around embedding generation.
pub trait EmbeddingProvider: Send + Sync + std::fmt::Debug {
    /// Embeds a batch of texts, returning one vector per input in matching
    /// order.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::Embedding`] if model inference fails.
    fn embed(&self, texts: &[String])
    -> Result<Vec<Embedding>, ZoteroApiError>;
}

/// Resolves the `SQLite` index file path.
///
/// # Errors
///
/// Returns [`ZoteroApiError::LocalDb`] if `override_path` is [`None`] and no
/// data dir found.
#[inline]
pub fn resolve_db_path(
    override_path: Option<&std::path::Path>,
) -> Result<PathBuf, ZoteroApiError> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    default_semantic_data_dir()
        .map(|dir| dir.join("embeddings.sqlite"))
        .ok_or_else(|| {
            ZoteroApiError::LocalDb(
                "Could not determine a data directory for the semantic search \
                 index; set ZOTERO_SEMANTIC_DB_PATH"
                    .to_owned(),
            )
        })
}

/// Resolves the directory where ONNX model files are cached.
#[must_use]
#[inline]
pub fn resolve_model_cache_dir(db_path: &std::path::Path) -> PathBuf {
    db_path
        .parent()
        .map_or_else(|| PathBuf::from("models"), |parent| parent.join("models"))
}

/// Returns the per-user default app data directory for server files.
fn default_semantic_data_dir() -> Option<PathBuf> {
    if let Some(appdata) = env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata).join("zotero-mcp-rs"));
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        return Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("zotero-mcp-rs"),
        );
    }
    #[cfg(target_os = "linux")]
    if let Some(home) = env::var_os("HOME") {
        return Some(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("zotero-mcp-rs"),
        );
    }
    None
}
