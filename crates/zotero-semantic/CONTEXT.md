# zotero-semantic

Local semantic search for Zotero libraries. Embeds item content using a local ONNX model, stores vectors in SQLite, and answers natural-language queries via cosine-similarity ranking. No cloud APIs — everything runs on-device.

## Language

**SemanticIndex**:
The side-car SQLite database holding indexed items and their text chunks with vector embeddings. Uses WAL journaling, foreign keys, and BLOB-encoded embeddings.
_Avoid_: vector store, embedding database (SemanticIndex is the canonical name in code)

**Chunk**:
A bounded segment of text (at most `MAX_CHUNK_CHARS` bytes) ready for embedding. Produced by `chunk_text` from assembled item text. Stored with a `chunk_index` ordering.
_Avoid_: segment, fragment, block

**Embedding**:
A dense `Vec<f32>` vector representing the semantic meaning of text. L2-normalized. 384-dimensional with the default BGE-small-en-v1.5 model.
_Avoid_: vector, representation (too vague)

**EmbeddingProvider**:
Trait that produces `Embedding`s from text. `FastEmbedProvider` is the concrete implementation using local ONNX inference.
_Avoid_: model, encoder

**Cosine similarity**:
Dot product of two L2-normalized vectors. Used to measure semantic similarity between a query embedding and stored chunk embeddings. The scoring metric for search.
_Avoid_: distance, relevance score

**Similarity threshold** (`min_similarity`):
Minimum cosine similarity score required for a search hit. Default is 0.3. Chunks scoring below this are discarded.
_Avoid_: cutoff, threshold

**Best-per-item deduplication**:
Search strategy where only the highest-scoring chunk per item is returned. Prevents a single document from dominating results.
_Avoid_: top chunk, winner-take-all

**Assemble item text**:
Concatenates title + abstract + attachment fulltext for an item before chunking. Subject to `MAX_INDEXABLE_CHARS` (400,000 chars).
_Avoid_: text extraction, content assembly

**Incremental indexing**:
Indexing mode that skips unchanged items by comparing `dateModified`. Controlled by a `force` flag to bypass and re-index everything.
_Aavoid_: delta sync, partial reindex

**IndexReport**:
Summary counters from an indexing run: items scanned, indexed, skipped (unchanged or empty), deleted, and chunks written.
_Avoid_: index stats, indexing result

**SemanticSearchHit**:
One search result: `item_key`, `title`, `similarity` score, `chunk_index`, and `chunk_text`. Sorted by descending similarity.
_Aavoid_: search result, match

**Pluggable providers**:
The `EmbeddingProvider` trait allows swapping the default BGE model for custom embedding models without changing the indexing or search code.
_Aavoid_: swappable model, provider abstraction
