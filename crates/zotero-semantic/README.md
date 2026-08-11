# zotero-semantic

Local semantic search for Zotero libraries. Embeds item content (titles,
abstracts, attachment fulltext) using a local ONNX model (BGE-small-en-v1.5),
stores vectors in SQLite, and answers natural-language queries via
cosine-similarity ranking.

No cloud APIs, no external services — everything runs on-device.

## Features

- **Paragraph-aware chunking** — splits text on paragraph boundaries, falls back
  to sentence separators, then hard character limits
- **Local embeddings** — ONNX-based BGE-small-en-v1.5 (384-dim) via `fastembed`,
  no API keys needed
- **SQLite vector store** — WAL mode, foreign keys, BLOB-encoded embeddings
- **Incremental indexing** — skips unchanged items by `dateModified`, removes
  stale entries
- **Cosine-similarity search** — L2-normalized dot product, best-chunk-per-item
  deduplication
- **Pluggable providers** — `EmbeddingProvider` trait for custom models

## Usage

```rust
use std::sync::Arc;
use zotero_semantic::{
    FastEmbedProvider, SemanticIndex, index_library, search_library,
    resolve_db_path, resolve_model_cache_dir,
};

// 1. Open or create the index
let db_path = resolve_db_path(None)?;
let index = SemanticIndex::open(&db_path).await?;

// 2. Load the embedding model (downloads ~130 MB on first run)
let model_dir = resolve_model_cache_dir(&db_path);
let provider = Arc::new(FastEmbedProvider::load(&model_dir)?);

// 3. Index your library
let report = index_library(&client, &index, &provider, false).await?;
println!(
    "Indexed {} items, {} chunks",
    report.items_indexed, report.chunks_written
);

// 4. Search
let chunks = index.load_all_chunks().await?;
let hits = search_library(
    &provider, &chunks, "quantum entanglement", 10, 0.3
).await?;
for hit in &hits {
    println!(
        "[{:.3}] {} — {}",
        hit.similarity,
        hit.title.as_deref().unwrap_or("untitled"),
        &hit.chunk_text[..80]
    );
}
```

## API Overview

| Item                      | Description                                                                            |
| ------------------------- | -------------------------------------------------------------------------------------- |
| `EmbeddingProvider`       | Trait — `fn embed(&self, texts: &[String]) -> Result<Vec<Embedding>>`                  |
| `FastEmbedProvider`       | Concrete implementation using `fastembed` (BGE-small-en-v1.5)                          |
| `SemanticIndex`           | SQLite-backed store — `open`, `upsert_item`, `delete_item`, `load_all_chunks`, `stats` |
| `index_library`           | Full library scan from Zotero API, incremental or forced                               |
| `search_library`          | Embed query, score all chunks, return ranked hits                                      |
| `chunk_text`              | Paragraph-aware text splitter                                                          |
| `Embedding`               | Newtype over `Vec<f32>` with `normalize`, `dot`, `encode`/`decode`                     |
| `resolve_db_path`         | Determines SQLite DB location (override or default)                                    |
| `resolve_model_cache_dir` | Determines ONNX model cache directory                                                  |

### Constants

| Constant                 | Default | Description                                     |
| ------------------------ | ------- | ----------------------------------------------- |
| `MAX_INDEXABLE_CHARS`    | 400,000 | Max text per item (title + abstract + fulltext) |
| `MAX_CHUNK_CHARS`        | 6,000   | Max characters per chunk                        |
| `DEFAULT_MIN_SIMILARITY` | 0.3     | Minimum cosine similarity to include in results |

### Key Types

```rust
pub struct SemanticSearchHit {
    pub item_key: ItemKey,
    pub title: Option<String>,
    pub similarity: f32,
    pub chunk_index: i64,
    pub chunk_text: String,
}

pub struct IndexReport {
    pub items_scanned: usize,
    pub items_indexed: usize,
    pub items_skipped_unchanged: usize,
    pub items_skipped_empty: usize,
    pub items_deleted: usize,
    pub chunks_written: usize,
}

pub struct SemanticIndexStats {
    pub indexed_items: i64,
    pub indexed_chunks: i64,
}
```

## How It Works

1. **Indexing** — fetches all items from the Zotero API, assembles text (title +
   abstract + first child attachment fulltext), chunks it, embeds in batches of 32,
   L2-normalizes, and upserts into SQLite. Stale items (deleted or modified since
   last index) are removed.
2. **Search** — embeds the query text, computes dot product against all stored
   chunks (pre-normalized), keeps the highest-scoring chunk per item above
   `min_similarity`, and returns results sorted by descending score.
3. **Chunking** — recursive splitting: paragraph breaks (`\n\n`) first, then
   sentence separators (`". "`, `"! "`, `"? "`), then hard character cuts. Ensures
   no chunk exceeds `MAX_CHUNK_CHARS`.

## Integration

Part of the `zotero-rs` workspace. Used by [`zotero-mcp`](../zotero-mcp/) to
power the `zotero_semantic_search` MCP tool.

```
zotero-mcp → zotero-semantic → zotero-api (sqlite feature)
                                fastembed
                                sqlx (sqlite)
```

## Development

Run the full integration test (downloads the real ONNX model):

```sh
cargo test --test ort_smoke -- --ignored --nocapture
```

Unit tests cover chunking, embedding encode/decode, SQLite upsert/delete
roundtrips, indexing with mock servers, and search ranking logic.
