# Context Map

## Contexts

- [zotero-api](./crates/zotero-api/CONTEXT.md) — async client and types for the Zotero Local API, Better BibTeX, and Better Notes
- [zotero-semantic](./crates/zotero-semantic/CONTEXT.md) — local semantic search: embeddings, chunking, vector similarity
- [zotero-mcp](./crates/zotero-mcp/CONTEXT.md) — MCP server exposing Zotero tools to AI agents
- [zotero-cli](./crates/zotero-cli/CONTEXT.md) — CLI interface (stub)

## Relationships

- **zotero-mcp → zotero-api**: MCP tools call the API client for CRUD, search, and metadata operations
- **zotero-mcp → zotero-semantic**: MCP tools call the semantic index for vector search
- **zotero-semantic → zotero-api**: indexing reads items, collections, and attachment fulltext via the API client (sqlite feature for direct DB access)
- **bridge → zotero-mcp**: the companion bridge script runs inside Zotero and exposes in-process APIs via HTTP to the Rust MCP server
