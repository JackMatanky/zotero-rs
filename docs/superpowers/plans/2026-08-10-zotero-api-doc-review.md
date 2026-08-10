# zotero-api Doc Comment Review & Revision Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite all module-level (`//!`) and public item-level (`///`) doc comments in `crates/zotero-api/` so they read as if written in a single document by one author — terse technical prose, consistent voice, precise and useful for developers integrating with the Zotero Local API.

**Architecture:** Each task targets one source file (or small cluster). The agent reads the file, rewrites every doc comment to match the style guide below, then verifies compilation and doc tests.

**Tech Stack:** Rust, rustdoc, `cargo doc`, `cargo test --doc`, `harper` CLI (grammar check)

---

## Style Guide

### Voice

Write like `reqwest` or `serde` — terse technical prose. Every doc comment follows this rhythm:

1. **First line:** Action-oriented summary. Starts with a verb. No "This function..." prefix.
2. **Body:** Behavior, intent, and protocol details that affect how the caller uses the code. Not types (the signature already shows those).
3. **Sections:** `# Errors` for every `Result`-returning function. `# Examples` for every public API. `# Arguments` only when 3+ parameters need explanation. `# Panics` when applicable.

**Rules:**
- First line is a sentence fragment ending with a period. "Fetches recent items sorted by modification date descending." not "This method fetches..."
- Never document the obvious: "Returns the key" on a `key()` getter.
- Never restate the type signature in prose.
- Include protocol specifics when they affect calling code (retry semantics, concurrency guards, upload protocols). Skip implementation internals.
- Use intra-doc links for every type reference: [`ZoteroItem`], [`LibraryVersion`], etc.
- Consistent terminology: "item" not "entry", "collection" not "folder", "library" not "database".

### Before / After Examples

**Before (bad — vague, no protocol detail, no voice):**
```rust
/// Gets items.
pub async fn get_recent_items(...)
```

**After (good — action verb, sort order, exclusion behavior):**
```rust
/// Fetches recent items sorted by modification date descending.
///
/// Returns up to `limit` items, excluding notes and annotations. Items are
/// ordered by `dateModified` descending (most recently modified first).
pub async fn get_recent_items(...)
```

**Before (bad — restates the type):**
```rust
/// Returns an Option of LinkMode.
pub fn link_mode(&self) -> Option<LinkMode>
```

**After (good — explains what it reads and when it's None):**
```rust
/// Returns the attachment storage mode (e.g. `imported_file`,
/// `linked_file`) if this item is an attachment.
///
/// Reads the `linkMode` field from [`extra_fields`](ZoteroItemData::extra_fields).
/// Returns `None` for non-attachment items or when the field is absent.
pub fn link_mode(&self) -> Option<LinkMode>
```

**Before (bad — no retry semantics, no error detail):**
```rust
/// Sends the request.
pub async fn send_raw(&self) -> Result<reqwest::Response, ZoteroApiError>
```

**After (good — retry behavior, error conditions):**
```rust
/// Sends the request, returning the raw [`reqwest::Response`].
///
/// Retries on 429 (Too Many Requests) and 5xx server errors up to 3
/// times with exponential backoff, respecting the `Retry-After` header
/// when present.
///
/// # Errors
///
/// Returns [`ZoteroApiError::Network`] if every retry attempt fails at the
/// transport level.
pub async fn send_raw(&self) -> Result<reqwest::Response, ZoteroApiError>
```

---

## File Structure

All files under `crates/zotero-api/src/`. No new files created.

| File | Scope |
|------|-------|
| `lib.rs` | Crate-level docs, feature flag documentation, re-export summaries |
| `errors.rs` | `ZoteroApiError` enum variants |
| `types.rs` | `ItemType`, `AnnotationType`, `CreatorType`, `LinkMode`, `CollectionParent`, `TagOrigin` |
| `keys.rs` | `ItemKey`, `CollectionKey`, `TagName`, `CitationKey`, `LibraryVersion`, `RelationUri` |
| `objects.rs` | `ZoteroItem`, `ZoteroItemData`, `ZoteroCreator`, `ZoteroTag`, `ZoteroCollection`, `LocalApiStatus`, `BatchWriteResponse`, `ItemDraft`, `ItemLinks`, `ItemMeta`, `LibraryInfo` |
| `client.rs` | `ZoteroClient`, `ApiRequestBuilder`, `ZoteroResponse`, `LibraryTarget`, `LocalAuthResponse` |
| `items.rs` | `TrashAction`, `ZoteroClient` item methods |
| `collections.rs` | `CollectionItemAction`, `ZoteroClient` collection methods |
| `tags.rs` | `ZoteroClient` tag methods |
| `searches.rs` | `SavedSearch`, `ZoteroClient` saved search methods |
| `search.rs` | `SearchField`, `SearchOperator`, `SearchCondition`, `JoinMode`, `SortField`, `SortOrder`, `PaginationInfo`, `SearchPage`, `ZoteroClient` search methods |
| `relations.rs` | `RelatedItem`, `ZoteroClient` relation methods |
| `notes.rs` | `AnnotationDraft`, `AnnotationPosition`, `ZoteroClient` note/annotation methods |
| `settings.rs` | `SettingEntry`, `ZoteroClient` settings methods |
| `deleted.rs` | `DeletedObjectsResponse`, `ZoteroClient::get_deleted` |
| `analysis.rs` | `LibraryCoverage`, `LibraryCoveragePage`, `DuplicateGroup`, `DuplicateType`, `ZoteroClient` analysis methods |
| `better_bibtex/` | `BetterBibtexClient`, `BibliographyFormat`, `AutoExportAddRequest`, and all model types |
| `better_notes/` | `BetterNotesClient`, `NoteExportFormat`, `TemplateName`, and all model types |
| `metadata.rs` | `IdentifierKind`, `resolve_metadata`, `resolve_metadata_with_urls` |
| `pdf.rs` | `extract_pdf_pages`, `extract_pdf_outline`, `PdfOutlineEntry` |
| `sqlite.rs` | `LocalZoteroDb`, `FulltextHit`, `NoteAnnotationHit`, `find_zotero_db` |

---

### Task 1: Crate Root (`lib.rs`)

**Files:**
- Modify: `crates/zotero-api/src/lib.rs`

- [ ] **Step 1: Read the file**

Read `src/lib.rs` in full. Note current crate-level docs, module declarations, and re-exports.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every `//!` doc comment and every `///` doc comment on public items to match the style guide. Specifically:

- Crate-level docs must include: one-line summary, `# Main Components` with intra-doc links to `ZoteroClient`, `BetterBibtexClient`, `BetterNotesClient`, `LocalZoteroDb`, a `# Features` table documenting `metadata`, `pdf`, `sqlite`, `test-util`, `full`, and a `# Examples` section with a working `no_run` example.
- Module re-exports should have brief doc summaries if they appear in the public API.
- Ensure the example compiles: imports `ZoteroApiError` and `ZoteroClient`, uses `# async fn run()` wrapper.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api`
Expected: No warnings or errors.

- [ ] **Step 4: Run doc tests**

Run: `cargo test --doc -p zotero-api`
Expected: All doc tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/zotero-api/src/lib.rs
git commit -m "docs(zotero-api): rewrite crate-level docs to match style guide"
```

---

### Task 2: Error Types (`errors.rs`)

**Files:**
- Modify: `crates/zotero-api/src/errors.rs`

- [ ] **Step 1: Read the file**

Read `src/errors.rs` in full. Note current enum docs, variant docs, and `From` impls.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- Module-level `//!` doc: brief summary of unified error type.
- `ZoteroApiError` enum doc: one-line summary, explain what it wraps, include `# Examples` showing variant matching.
- Each variant: explain the failure condition in developer terms. `VersionConflict` must explain `If-Unmodified-Since-Version` and retry semantics. `Sqlite` must note the `sqlite` feature gate. Skip docs on `From` impls (standard trait).
- Variants that map to HTTP status codes should mention the status when relevant (e.g., `NotFound` → 404, `VersionConflict` → 412).

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/errors.rs
git commit -m "docs(zotero-api): rewrite error type docs for developer usefulness"
```

---

### Task 3: Controlled Vocabulary Types (`types.rs`)

**Files:**
- Modify: `crates/zotero-api/src/types.rs`

- [ ] **Step 1: Read the file**

Read `src/types.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- Module doc: brief summary of controlled vocabulary types.
- `ItemType` enum: explain these are Zotero item type identifiers. Variant docs should explain what each type represents (e.g., `Attachment` = file attachments, `Note` = standalone notes). Add `is_indexable()` doc explaining which types are excluded and why.
- `CollectionParent`: explain wire format (`false` = top-level, string = child key). Add `# Examples`.
- `TagOrigin`: explain Zotero's tag origin system (0 = user, 1 = automatic). Add `# Examples`.
- `AnnotationType`, `CreatorType`, `LinkMode`: keep existing variant docs, ensure consistent voice.

- [ ] **Step 3: Verify docs compile and examples pass**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features && cargo test --doc -p zotero-api --all-features`
Expected: No warnings, all doc tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/types.rs
git commit -m "docs(zotero-api): rewrite controlled vocabulary type docs"
```

---

### Task 4: Key Newtypes (`keys.rs`)

**Files:**
- Modify: `crates/zotero-api/src/keys.rs`

- [ ] **Step 1: Read the file**

Read `src/keys.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `string_newtype!` macro-generated docs: the macro passes strings as doc comments. If those strings are inconsistent, note which newtypes need manual doc overrides after macro expansion.
- `LibraryVersion`: explain optimistic concurrency control, `If-Unmodified-Since-Version` header, `412 Precondition Failed` retry semantics. Add `# Examples`.
- `RelationUri` and `RelationUriError`: explain the URI format (`http://zotero.org/users/0/items/{KEY}`), validation rules (8-char alphanumeric key). Add `# Errors` to `TryFrom` impl.
- `ItemKey`, `CollectionKey`, `TagName`, `CitationKey`: ensure brief, accurate docs from macro are consistent.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/keys.rs
git commit -m "docs(zotero-api): rewrite key newtype docs with concurrency semantics"
```

---

### Task 5: Data Structures (`objects.rs`)

**Files:**
- Modify: `crates/zotero-api/src/objects.rs`

- [ ] **Step 1: Read the file**

Read `src/objects.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `BatchWriteResponse`: explain it's returned by batch create/update, describe what `successful`, `unchanged`, `failed` contain.
- `ZoteroItemData`: document the core fields (`key`, `itemType`, `title`, `dateAdded`, `dateModified`, `creators`, `tags`, `extra`, `version`) and the `extra_fields` catch-all. Document accessor methods (`get_str`, `get_field`, `set_field`, `link_mode`) explaining what they read and when they return `None`.
- `ZoteroCollection`: explain the tree structure, `CollectionKey`, `LibraryVersion` for concurrency.
- `LocalApiStatus`: document each field (`online`, `url`, `version`, `error`).
- `ItemDraft`: explain it's used for creating items, link to `crate::metadata::resolve_metadata`.
- Skip docs on `From`/`Into` impls (standard trait).

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/objects.rs
git commit -m "docs(zotero-api): rewrite data structure docs with field descriptions"
```

---

### Task 6: HTTP Client (`client.rs`)

**Files:**
- Modify: `crates/zotero-api/src/client.rs`

- [ ] **Step 1: Read the file**

Read `src/client.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `ZoteroResponse<T>`: explain it wraps API response payloads with Zotero response headers, derefs to inner `T`.
- `ZoteroClient::new`: document default URL (`http://127.0.0.1:23119/api`), default library target (`User(0)`). Add `# Examples`.
- `check_status`: explain the probe request (`GET /items?limit=1`), what `LocalApiStatus` contains. Add `# Examples`.
- `request_local_authorization`: explain the auth check, add `# Examples`.
- `send_raw`: explain retry behavior (429, 5xx, exponential backoff, `Retry-After`). Add `# Errors`.
- `send`: explain JSON deserialization, `ZoteroResponse<T>`. Add `# Errors`.
- `send_or_not_found`, `send_unit`: add `# Errors`.
- `ApiRequestBuilder` methods: add `# Errors` sections where applicable.
- Skip docs on trait impls (`Clone`, `Debug`, `Default`).

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/client.rs
git commit -m "docs(zotero-api): rewrite client docs with retry semantics and examples"
```

---

### Task 7: Item Operations (`items.rs`)

**Files:**
- Modify: `crates/zotero-api/src/items.rs`

- [ ] **Step 1: Read the file**

Read `src/items.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `get_recent_items`: explain sort order (`dateModified` descending), exclusion of notes/annotations.
- `get_all_items`: explain automatic pagination (100 items/page), concatenation, exclusion of notes.
- `get_all_items_with_keys`: explain key-based filtering.
- `create_item_from_metadata`: explain it takes an `ItemDraft`, returns created items.
- `import_pdf_file`: explain the 3-phase MD5 upload protocol (metadata record → upload ticket → raw bytes). Explain when upload is skipped (MD5 match). Document all error conditions.
- `attach_file_link`: explain difference from `import_pdf_file` (path reference only, no upload).
- `TrashAction`: keep variant docs, ensure consistent voice.
- All methods: ensure `# Errors` sections with specific variant references.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/items.rs
git commit -m "docs(zotero-api): rewrite item operation docs with upload protocol details"
```

---

### Task 8: Collection Operations (`collections.rs`)

**Files:**
- Modify: `crates/zotero-api/src/collections.rs`

- [ ] **Step 1: Read the file**

Read `src/collections.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `get_collections`: explain it returns the full tree as a flat list.
- `search_collections`: explain case-insensitive substring matching.
- `create_collection`: explain `parent_key` semantics (None = top-level, Some = child).
- `manage_collection_items`: explain `Add`/`Remove` behavior, what happens if item already in collection.
- `update_collection`: explain which fields are optional (None = keep current).
- `CollectionItemAction`: keep variant docs, ensure consistent voice.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/collections.rs
git commit -m "docs(zotero-api): rewrite collection operation docs"
```

---

### Task 9: Tag Operations (`tags.rs`)

**Files:**
- Modify: `crates/zotero-api/src/tags.rs`

- [ ] **Step 1: Read the file**

Read `src/tags.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `list_tags`: explain the `limit` parameter, return type (`TagName` wrappers).
- `batch_update_tags`: explain the diff semantics (add without duplicating, remove by exact name), return count.
- `rename_tag`: explain search-and-replace behavior across all items.
- `delete_tags`: explain exact name matching, version guard, 50-tag limit.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/tags.rs
git commit -m "docs(zotero-api): rewrite tag operation docs"
```

---

### Task 10: Saved Searches (`searches.rs`)

**Files:**
- Modify: `crates/zotero-api/src/searches.rs`

- [ ] **Step 1: Read the file**

Read `src/searches.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `SavedSearch`: explain it stores query conditions server-side, add `# Examples`.
- `execute_saved_search`: explain server-side evaluation, return type.
- `create_searches`: explain expected JSON format for search conditions.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/searches.rs
git commit -m "docs(zotero-api): rewrite saved search docs"
```

---

### Task 11: Search & Query Types (`search.rs`)

**Files:**
- Modify: `crates/zotero-api/src/search.rs`

- [ ] **Step 1: Read the file**

Read `src/search.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `SearchField`: explain it represents which item field to match, `Other` variant for forward compatibility.
- `SearchOperator`: explain comparison semantics, note which operators are date/numeric-only.
- `SearchCondition`: explain it pairs field + operator + value.
- `JoinMode`: explain AND (`All`) vs OR (`Any`).
- `SortField`, `SortOrder`: brief docs.
- `PaginationInfo`, `SearchPage`: explain pagination metadata.
- `advanced_search`: explain server-side pushdown when conditions are simple, client-side fallback for complex queries. Document all error conditions.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/search.rs
git commit -m "docs(zotero-api): rewrite search type and query docs"
```

---

### Task 12: Relations (`relations.rs`)

**Files:**
- Modify: `crates/zotero-api/src/relations.rs`

- [ ] **Step 1: Read the file**

Read `src/relations.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `RelatedItem`: explain it's a minimal reference to a related item via `dc:relation` URI.
- `get_related_items`: explain URI resolution, silent skipping of unresolvable URIs.
- `add_item_relation`: explain bidirectional update, non-atomic nature (asymmetric failure possible).
- `remove_item_relation`: explain it removes the relation from one item only.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/relations.rs
git commit -m "docs(zotero-api): rewrite relation docs with bidirectional semantics"
```

---

### Task 13: Notes & Annotations (`notes.rs`)

**Files:**
- Modify: `crates/zotero-api/src/notes.rs`

- [ ] **Step 1: Read the file**

Read `src/notes.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `AnnotationPosition`: explain it wraps opaque JSON coordinates, pass through to API as-is.
- `AnnotationDraft`: explain it's a payload for creating PDF annotations, document key fields.
- `create_note`: explain HTML note format requirement.
- `create_annotation`: explain position format requirements.
- `synthesize_annotations`: explain Markdown output structure (heading, metadata, annotations, notes sections).
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/notes.rs
git commit -m "docs(zotero-api): rewrite note and annotation docs"
```

---

### Task 14: Settings & Deleted (`settings.rs`, `deleted.rs`)

**Files:**
- Modify: `crates/zotero-api/src/settings.rs`
- Modify: `crates/zotero-api/src/deleted.rs`

- [ ] **Step 1: Read both files**

Read `src/settings.rs` and `src/deleted.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `SettingEntry`: explain it's a key-value setting payload, `value` is JSON-typed.
- `get_settings`, `get_setting`, `update_setting`: explain behavior, `update_setting` should note what happens if key doesn't exist.
- `DeletedObjectsResponse`: explain incremental sync protocol, `since` parameter semantics. Add `# Examples`.
- `get_deleted`: explain version-based filtering, how to use `Last-Modified-Version` from previous calls.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/settings.rs crates/zotero-api/src/deleted.rs
git commit -m "docs(zotero-api): rewrite settings and deleted sync docs"
```

---

### Task 15: Analysis (`analysis.rs`)

**Files:**
- Modify: `crates/zotero-api/src/analysis.rs`

- [ ] **Step 1: Read the file**

Read `src/analysis.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- `LibraryCoverage`: explain it's client-side aggregate statistics for PDF/DOI/note coverage.
- `LibraryCoveragePage`: explain pagination metadata alongside coverage data.
- `DuplicateGroup`: explain matching criteria (DOI or title, normalization rules).
- `DuplicateType`: brief doc.
- `get_library_coverage`: explain what metrics are computed, `# Arguments` for `collection_key`, `offset`, `limit`.
- `find_duplicates`: explain matching criteria (case-insensitive, trimmed, titles >5 chars, groups of 2+).
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/analysis.rs
git commit -m "docs(zotero-api): rewrite analysis module docs"
```

---

### Task 16: Better BibTeX (`better_bibtex/`)

**Files:**
- Modify: `crates/zotero-api/src/better_bibtex/mod.rs`
- Modify: `crates/zotero-api/src/better_bibtex/client.rs`
- Modify: `crates/zotero-api/src/better_bibtex/models.rs`

- [ ] **Step 1: Read all three files**

Read all files in `src/better_bibtex/`.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- Module doc: brief summary of Better BibTeX integration.
- `BetterBibtexClient`: explain it communicates via JSON-RPC 2.0, list capabilities. Add `# Examples`.
- `bibliography`: explain format parameter, default behavior.
- `scan_aux`: explain AUX scanning (reads `\citation{...}` from `.aux` files).
- `autoexport_add`: explain `replace` semantics.
- Model types: ensure struct-level docs, field docs where non-obvious.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/better_bibtex/
git commit -m "docs(zotero-api): rewrite Better BibTeX client and model docs"
```

---

### Task 17: Better Notes (`better_notes/`)

**Files:**
- Modify: `crates/zotero-api/src/better_notes/mod.rs`
- Modify: `crates/zotero-api/src/better_notes/client.rs`
- Modify: `crates/zotero-api/src/better_notes/models.rs`

- [ ] **Step 1: Read all three files**

Read all files in `src/better_notes/`.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- Module doc: brief summary of Better Notes integration.
- `BetterNotesClient`: explain it communicates via HTTP companion endpoint, list capabilities. Add `# Examples`.
- `export`: explain format parameter (Markdown vs HTML).
- `convert_from_markdown`: explain Markdown-to-HTML conversion, parent attachment.
- `run_template`: explain template execution.
- `NoteExportFormat`: explain it controls output encoding.
- Model types: ensure struct-level docs, field docs where non-obvious.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/better_notes/
git commit -m "docs(zotero-api): rewrite Better Notes client and model docs"
```

---

### Task 18: Metadata Resolution (`metadata.rs`)

**Files:**
- Modify: `crates/zotero-api/src/metadata.rs`

- [ ] **Step 1: Read the file**

Read `src/metadata.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- Module doc: explain metadata resolution for DOI/arXiv/ISBN, include table of external APIs and default base URLs.
- `IdentifierKind`: explain when to use each variant.
- `resolve_metadata`: convenience wrapper, add `# Examples`.
- `resolve_metadata_with_urls`: explain custom base URL overrides for testing. Fix doc placement (must come before `#[expect]` attribute).
- All functions: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/metadata.rs
git commit -m "docs(zotero-api): rewrite metadata module docs with API table"
```

---

### Task 19: PDF Extraction (`pdf.rs`)

**Files:**
- Modify: `crates/zotero-api/src/pdf.rs`

- [ ] **Step 1: Read the file**

Read `src/pdf.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- Module doc: keep existing example, ensure consistent voice.
- `extract_pdf_pages`: keep `# Arguments` and `# Errors`, ensure consistent voice.
- `extract_pdf_outline`: add `# Arguments` section, explain empty-outline behavior.
- `PdfOutlineEntry`: explain nesting `level` field.
- Skip docs on test-util functions.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/pdf.rs
git commit -m "docs(zotero-api): rewrite PDF extraction docs"
```

---

### Task 20: SQLite Access (`sqlite.rs`)

**Files:**
- Modify: `crates/zotero-api/src/sqlite.rs`

- [ ] **Step 1: Read the file**

Read `src/sqlite.rs` in full.

- [ ] **Step 2: Rewrite all doc comments**

Rewrite every doc comment to match the style guide. Specifically:

- Module doc: keep existing example, ensure consistent voice.
- `LocalZoteroDb`: explain immutable read-only mode, `SQLITE_BUSY` avoidance. Add `# Examples`.
- `find_zotero_db`: explain search order (override path → env var → profile dirs → default).
- `search_fulltext`: explain tokenization (split on punctuation, lowercase, dedup), metadata vs full-text matching, relevance ordering.
- `search_notes_annotations`: explain HTML stripping.
- `FulltextHit`: explain it's a single full-text search hit with item metadata.
- `NoteAnnotationHit`: explain it distinguishes child notes from PDF annotations.
- All methods: ensure `# Errors` sections.

- [ ] **Step 3: Verify docs compile**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/zotero-api/src/sqlite.rs
git commit -m "docs(zotero-api): rewrite SQLite module docs with search behavior"
```

---

### Task 21: Final Verification

- [ ] **Step 1: Full doc build with all features**

Run: `cargo doc --no-deps --document-private-items -p zotero-api --all-features 2>&1`
Expected: Zero warnings.

- [ ] **Step 2: Run all doc tests**

Run: `cargo test --doc -p zotero-api --all-features`
Expected: All doc tests pass.

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p zotero-api --all-features`
Expected: All tests pass.

- [ ] **Step 4: Grammar check with harper** (if available)

Run: `harper check crates/zotero-api/src/*.rs crates/zotero-api/src/**/*.rs`
Fix any flagged issues.

- [ ] **Step 5: Final commit if any grammar fixes were needed**

```bash
git add crates/zotero-api/src/
git commit -m "docs(zotero-api): fix grammar issues found by harper"
```
