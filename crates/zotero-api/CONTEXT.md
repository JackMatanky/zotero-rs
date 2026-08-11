# zotero-api

Async client and types for the Zotero Local API, Better BibTeX, and Better Notes. Provides typed Rust abstractions for reading and writing to a running Zotero instance via its HTTP Local API on `localhost:23119`.

## Language

**Library**:
A Zotero user or group collection of items. Identified by a numeric ID; user libraries use `/users/<id>` and group libraries use `/groups/<id>`.
_Avoid_: account, workspace

**Item**:
A bibliographic entry, note, attachment, or annotation in a library. Identified by an 8-character `ItemKey`. Has a version number for optimistic concurrency.
_Avoid_: record, entry, document

**Collection**:
A folder-like grouping of items. Identified by a `CollectionKey`. Collections form a tree via `CollectionParent` (top-level or child).
_Avoid_: folder, group, category

**Attachment**:
An item of type `attachment` that holds a file. Has a `LinkMode` indicating how the file is stored: `imported_file`, `linked_file`, `linked_url`, or `imported_url`.
_Avoid_: file (too ambiguous — attachment is the Zotero term for the item that references the file)

**Annotation**:
A PDF annotation (highlight, underline, or note) on a parent attachment. Identified by an `ItemKey` and positioned via `AnnotationPosition` coordinates.
_Avoid_: highlight, markup

**Note**:
An HTML note item attached to a parent item. Distinguished from annotations by being standalone items (item type `note`).
_Avoid_: comment (ambiguous with annotation comments)

**Tag**:
A label applied to items. Has an origin: user-created (`TagOrigin::User`) or auto-assigned on import (`TagOrigin::Automatic`).
_Avoid_: label

**Creator**:
An author, editor, or other role credited on an item. Has a `CreatorType` (author, editor, translator, or other).
_Avoid_: author (too narrow — covers editors/translators too)

**ItemDraft**:
A payload for creating a new item, containing type, title, creators, DOI, ISBN, and optional fields. Populated from metadata resolution or manually before passing to `create_item_from_metadata`.
_Avoid_: new item, item template

**CitationKey**:
Better BibTeX's cite key for an item (e.g., `smith2020deep`). Stored in the `citationKey` field or the `extra` field as `citation key: ...`. Used for LaTeX/BibTeX workflows.
_Avoid_: citekey, cite key

**CollectionPath**:
Slash-separated path in Better BibTeX's collection hierarchy. `"//"` is the personal library root.
_Avoid_: folder path

**LibraryVersion**:
Monotonically increasing `u64` counter for optimistic concurrency. Passed via `If-Unmodified-Since-Version` header on writes. Incremented per library per transaction.
_Aavoid_: version, etag

**ServerID**:
A string identifying a specific Zotero instance, returned in the `Zotero-Server-ID` header. Required on write requests. Cached data should be partitioned by server ID.
_Avoid_: instance ID

**SavedSearch**:
A persisted search query on the server with a name and JSON conditions array. Identified by a `SearchKey`.
_Avoid_: query, filter

**Relation**:
A bidirectional `dc:relation` link between items, stored as URIs in the item's `relations` JSON map.
_Aavoid_: link, reference

**DuplicateGroup**:
Items sharing the same normalized DOI or title (title must exceed 5 characters). Detected by `analysis::find_duplicates`.
_Avoid_: dupe, clone

**Fulltext**:
Zotero's indexed full-text content for an attachment, returned as plain text. Queried via `GET /items/<key>/fulltext`.
_Avoid_: plain text, OCR output

**Better BibTeX**:
A Zotero plugin that manages citation keys and provides JSON-RPC auto-export, AUX scanning, and bibliography generation. Accessed via `BetterBibtexClient` over JSON-RPC 2.0.
_Avoid_: bibtex plugin, bbt

**Better Notes**:
A Zotero plugin for rich Markdown-based notes. Exposes `Zotero.BetterNotes.api` in-process; the companion bridge script provides HTTP access to it.
_Avoid_: notes plugin

**Local API**:
The HTTP API exposed by the Zotero desktop client at `http://localhost:23119/api/`. Serves data from the local database; no network required. Supports reads without auth, writes with a local API key granted at runtime.
_Aavoid_: Zotero API (too broad — this is specifically the local variant)

**Metadata Resolution**:
Resolving a public identifier (DOI, arXiv, ISBN) into an `ItemDraft` via external APIs (Crossref, Semantic Scholar, Open Library). Behind the `metadata` feature gate.
_Aavoid_: lookup, fetch metadata

**ZoteroResponse\<T\>**:
Generic API response wrapper carrying the deserialized `data` and optional header metadata: `total_results`, `last_modified_version`, `server_id`.
_Avoid_: response wrapper, api result

**LibraryTarget**:
Enum selecting user (`User(0)` for active local user) or group library for API requests.
_Aavoid_: scope, library type
